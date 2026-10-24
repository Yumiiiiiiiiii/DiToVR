import {test} from "node:test";
import assert from "node:assert/strict";
import {createHash} from "node:crypto";
import {deflateRawSync} from "node:zlib";
import {singleFileZip, zipEntry} from "./store-archive.mjs";
import {storeManifest} from "./store-manifest.mjs";
import {validateArtifact, updateSubmission, uploadUrl, createStoreClient, publishStore} from "./publish-store.mjs";

const identity = {productId: "9NFDCGL30V3K", name: "NakYmi.DiscordtoVR", publisher: "CN=test-publisher", publisherDisplayName: "NakYmi"};
const sas = "https://example.blob.core.windows.net/ingestion/archive?sig=secret-sas";
function zip64Fixture(name, data) {
  const ordinary = singleFileZip(name, data);
  const packed = deflateRawSync(data);
  const local = Buffer.from(ordinary.subarray(0, 30 + name.length));
  local.writeUInt16LE(8, 8);
  local.writeUInt32LE(packed.length, 18);
  const central = Buffer.from(ordinary.subarray(30 + name.length + data.length, -22));
  central.writeUInt16LE(8, 10);
  central.writeUInt32LE(0xffffffff, 20);
  central.writeUInt32LE(0xffffffff, 24);
  central.writeUInt32LE(0xffffffff, 42);
  const extra = Buffer.alloc(28);
  extra.writeUInt16LE(1, 0);
  extra.writeUInt16LE(24, 2);
  extra.writeBigUInt64LE(BigInt(data.length), 4);
  extra.writeBigUInt64LE(BigInt(packed.length), 12);
  extra.writeBigUInt64LE(0n, 20);
  central.writeUInt16LE(extra.length, 30);
  const centralStart = local.length + packed.length;
  const recordStart = centralStart + central.length + extra.length;
  const record = Buffer.alloc(56);
  record.writeUInt32LE(0x06064b50, 0);
  record.writeBigUInt64LE(44n, 4);
  record.writeUInt16LE(45, 12);
  record.writeUInt16LE(45, 14);
  record.writeBigUInt64LE(1n, 24);
  record.writeBigUInt64LE(1n, 32);
  record.writeBigUInt64LE(BigInt(central.length + extra.length), 40);
  record.writeBigUInt64LE(BigInt(centralStart), 48);
  const locator = Buffer.alloc(20);
  locator.writeUInt32LE(0x07064b50, 0);
  locator.writeBigUInt64LE(BigInt(recordStart), 8);
  locator.writeUInt32LE(1, 16);
  const end = Buffer.alloc(22, 0xff);
  end.writeUInt32LE(0x06054b50, 0);
  end.writeUInt16LE(0, 20);
  return Buffer.concat([local, packed, central, extra, record, locator, end]);
}
function artifact() {
  const msix = singleFileZip("AppxManifest.xml", Buffer.from(storeManifest(identity, "0.2.2")));
  const fileName = "DiscordToVR_0.2.2.0_x64.msix";
  return {identity, version: "0.2.2", fileName, msix, notes: "- Improved Store updates.", report: {
    EngineVersion: "1.1", SignatureVersion: "1.2", Files: [
      {File: fileName, SHA256: createHash("sha256").update(msix).digest("hex"), ScannerExitCode: 0},
      {File: "discord_To_VR.exe", ScannerExitCode: 0},
    ],
  }};
}
function published() {
  return {id: "1", status: "Published", fileUploadUrl: sas, pricing: {priceId: "Free", marketSpecificPricings: {}, isAdvancedPricingModel: true},
    listings: {"ja-jp": {baseListing: {description: "Existing description", releaseNotes: "Old notes", images: [{id: "image-id", fileStatus: "Uploaded"}]}}},
    applicationPackages: [
      {id: "x64-package", fileName: "old.msix", fileStatus: "Uploaded", architecture: "X64", version: "0.2.1.0"},
      {id: "arm-package", fileName: "arm.msix", fileStatus: "Uploaded", architecture: "ARM64", version: "0.2.1.0"},
    ], visibility: "Public", allowTargetFutureDeviceFamilies: {Desktop: true}, notesForCertification: "Existing test instructions"};
}
function fixture({app = {}, old = published(), draft = {}, statuses = ["PreProcessing"], uploadError = false, putError = false, commitStatus = "CommitStarted", secondApp = {}} = {}) {
  const calls = [];
  let reads = 0;
  const application = {id: identity.productId, packageIdentityName: identity.name, publisherName: identity.publisher, lastPublishedApplicationSubmission: {id: "1"}, pendingApplicationSubmission: null, ...app};
  const root = `/applications/${identity.productId}`;
  const client = {
    async authenticate() { calls.push({method: "AUTH"}); },
    async request(path, {method = "GET", body} = {}) {
      calls.push({path, method, body});
      if (path === root) return reads++ ? {...application, ...secondApp} : application;
      if (path === `${root}/submissions/1`) return structuredClone(old);
      if (path === `${root}/submissions` && method === "POST") return {...structuredClone(old), id: "2", ...draft};
      if (path === `${root}/submissions/2` && method === "PUT") { if (putError) throw new Error("update failed"); return body; }
      if (path.endsWith("/commit")) return {status: commitStatus};
      if (path.endsWith("/status")) return {status: statuses.length > 1 ? statuses.shift() : statuses[0]};
      throw new Error("Unexpected fixture request");
    },
    async upload(url, zip) { calls.push({method: "UPLOAD", url, zip}); if (uploadError) throw new Error("upload failed"); },
  };
  return {client, calls, writes: () => calls.filter(call => ["POST", "PUT", "UPLOAD", "DELETE"].includes(call.method))};
}

test("single-file ZIP round trips, excludes other files and rejects corrupt archives", () => {
  const zip = singleFileZip("AppxManifest.xml", Buffer.from("manifest"));
  assert.equal(zipEntry(zip, "AppxManifest.xml").toString(), "manifest");
  assert.throws(() => singleFileZip("../private.key", Buffer.from("secret")));
  for (const input of [Buffer.alloc(0), zip.subarray(0, 10), zip.subarray(0, -1)]) assert.throws(() => zipEntry(input, "AppxManifest.xml"));
  const corrupt = Buffer.from(zip);
  corrupt[30 + "AppxManifest.xml".length] ^= 1;
  assert.throws(() => zipEntry(corrupt, "AppxManifest.xml"), /checksum/);
});

test("MakeAppx-style ZIP64 and compressed manifests are verified with bounded offsets", () => {
  const name = "AppxManifest.xml", data = Buffer.from(storeManifest(identity, "0.2.2"));
  const zip = zip64Fixture(name, data);
  assert.deepEqual(zipEntry(zip, name), data);
  const badOffset = Buffer.from(zip);
  badOffset.writeBigUInt64LE(0xffffffffffffffffn, zip.length - 34);
  assert.throws(() => zipEntry(badOffset, name), /safe bounds/);
  const multidisk = Buffer.from(zip);
  multidisk.writeUInt32LE(2, zip.length - 26);
  assert.throws(() => zipEntry(multidisk, name), /Multi-disk/);
  const a = artifact();
  a.msix = zip;
  a.report.Files[0].SHA256 = createHash("sha256").update(zip).digest("hex");
  assert.match(validateArtifact(a), /^[a-f0-9]{64}$/);
});

test("only the scanned MSIX with the actual expected manifest passes artifact checks", () => {
  const a = artifact();
  assert.match(validateArtifact(a), /^[a-f0-9]{64}$/);
  for (const bad of [
    {...a, fileName: "other.msix"}, {...a, notes: ""}, {...a, notes: "x".repeat(1501)},
    {...a, report: {...a.report, Files: a.report.Files.map(f => ({...f, ScannerExitCode: 2}))}},
    {...a, report: {...a.report, Files: a.report.Files.slice(1)}},
    {...a, msix: Buffer.from("tampered")}, {...a, identity: {...identity, publisher: "CN=another"}},
    {...a, version: "0.2.1-beta"},
  ]) assert.throws(() => validateArtifact(bad));
});

test("metadata preserves listings, images, other architectures and certification notes", () => {
  const original = published();
  const next = updateSubmission(original, artifact());
  assert.equal(original.applicationPackages[0].fileStatus, "Uploaded");
  assert.equal(next.applicationPackages[0].fileStatus, "PendingDelete");
  assert.deepEqual(next.applicationPackages[1], original.applicationPackages[1]);
  assert.deepEqual(next.applicationPackages[2], {fileName: artifact().fileName, fileStatus: "PendingUpload", minimumDirectXVersion: "None", minimumSystemRam: "None"});
  assert.equal(next.listings["ja-jp"].baseListing.description, "Existing description");
  assert.deepEqual(next.listings["ja-jp"].baseListing.images, original.listings["ja-jp"].baseListing.images);
  assert.equal(next.listings["ja-jp"].baseListing.releaseNotes, artifact().notes);
  assert.equal(next.notesForCertification, original.notesForCertification);
  assert.equal(next.targetPublishMode, "Immediate");
  for (const field of ["fileUploadUrl", "id", "status", "statusDetails"]) assert.equal(next[field], undefined);
});

test("the API pricing-v2 Base alias is submitted explicitly as Free for this free product", () => {
  const old = published();
  old.pricing.priceId = "Base";
  const next = updateSubmission(old, artifact());
  assert.equal(next.pricing.priceId, "Free");
  assert.equal(old.pricing.priceId, "Base");
});

test("first publication, pending drafts and identity mismatch stop before mutations", async () => {
  for (const app of [
    {lastPublishedApplicationSubmission: null}, {pendingApplicationSubmission: {id: "manual-draft"}},
    {pendingApplicationSubmission: {id: "certification"}}, {packageIdentityName: "Other.App"}, {publisherName: "CN=other"}, {id: "OTHERPRODUCT"},
  ]) {
    const f = fixture({app});
    await assert.rejects(publishStore(artifact(), f));
    assert.deepEqual(f.writes(), []);
  }
});

test("paid/unavailable prices, non-increasing versions and incomplete packages stop before mutations", async () => {
  const sources = [];
  for (const priceId of ["Tier2", "", "NotAvailable"]) sources.push({...published(), pricing: {priceId}});
  sources.push({...published(), pricing: {priceId: "Free", marketSpecificPricings: {US: "Tier2"}}});
  for (const version of ["0.2.2.0", "0.3.0.0", "unknown"]) {
    const old = published(); old.applicationPackages[0].version = version; sources.push(old);
  }
  sources.push({...published(), listings: {}}, {...published(), applicationPackages: []});
  const neutral = published(); neutral.applicationPackages[1].architecture = "neutral"; sources.push(neutral);
  const pending = published(); pending.applicationPackages[0].fileStatus = "PendingUpload"; sources.push(pending);
  for (const old of sources) {
    const f = fixture({old});
    await assert.rejects(publishStore(artifact(), f));
    assert.deepEqual(f.writes(), []);
  }
});

test("a changed Store submission just before draft creation is protected", async () => {
  for (const secondApp of [{pendingApplicationSubmission: {id: "concurrent"}}, {lastPublishedApplicationSubmission: {id: "3"}}]) {
    const f = fixture({secondApp});
    await assert.rejects(publishStore(artifact(), f));
    assert.deepEqual(f.writes(), []);
  }
});

test("submission updates metadata, uploads only the MSIX, commits and confirms acceptance in order", async () => {
  const a = artifact(), f = fixture({statuses: ["CommitStarted", "PreProcessing"]});
  const result = await publishStore(a, {...f, wait: async () => {}});
  assert.equal(result.status, "PreProcessing");
  assert.equal(result.id, "2");
  assert.deepEqual(f.writes().map(c => c.method), ["POST", "PUT", "UPLOAD", "POST"]);
  const zip = f.calls.find(c => c.method === "UPLOAD").zip;
  assert.deepEqual(zipEntry(zip, a.fileName), a.msix);
  assert.throws(() => zipEntry(zip, "config.json"));
});

test("upload and metadata failures retain the draft and do not commit or delete it", async () => {
  for (const options of [{uploadError: true}, {putError: true}, {draft: {fileUploadUrl: "http://unsafe.example/?sig=secret"}}]) {
    const f = fixture(options);
    await assert.rejects(publishStore(artifact(), f));
    assert.equal(f.calls.some(c => c.path?.endsWith("/commit") || c.method === "DELETE"), false);
  }
});

test("commit failures and polling timeout do not report a successful submission", async () => {
  for (const options of [{statuses: ["CommitFailed"]}, {statuses: ["CertificationFailed"]}, {statuses: ["Unknown"]}, {statuses: ["CommitStarted"]}, {commitStatus: "CommitFailed"}]) {
    const f = fixture(options);
    await assert.rejects(publishStore(artifact(), {...f, attempts: 2, wait: async () => {}}));
    assert.equal(f.calls.some(c => c.method === "DELETE"), false);
  }
});

test("upload rejects non-Azure endpoints, redirects, userinfo and insecure URLs", () => {
  assert.equal(uploadUrl(sas), sas);
  for (const url of ["http://example.blob.core.windows.net/a?sig=s", "https://evil.example/a?sig=s", "https://example.blob.core.windows.net.evil.example/a?sig=s", "https://user@example.blob.core.windows.net/a?sig=s", "https://example.blob.core.windows.net/a", "bad-url"]) assert.throws(() => uploadUrl(url));
});

const credentials = {tenantId: "00000000-0000-0000-0000-000000000001", clientId: "00000000-0000-0000-0000-000000000002", clientSecret: "client-secret"};
test("transport keeps bearer tokens off Blob uploads and disables redirects", async () => {
  const calls = [], masked = [];
  const client = createStoreClient(credentials, {mask: value => masked.push(value), fetchImpl: async (url, options) => {
    calls.push({url, ...options});
    return {ok: true, json: async () => url.includes("oauth2") ? {access_token: "access-token"} : {}};
  }});
  await client.authenticate();
  await client.request(`/applications/${identity.productId}`);
  await client.upload(sas, Buffer.from("zip"));
  assert.equal(calls[0].body.get("resource"), "https://manage.devcenter.microsoft.com");
  assert.equal(calls[1].headers.Authorization, "Bearer access-token");
  assert.equal(calls[2].headers.Authorization, undefined);
  assert.equal(calls[2].headers["x-ms-blob-type"], "BlockBlob");
  assert.ok(calls.every(call => call.redirect === "error"));
  assert.deepEqual(masked, ["access-token", sas]);
  await assert.rejects(client.request(`/applications/${identity.productId}`, {method: "DELETE"}));
});

test("transport errors never echo response secrets or network exception URLs", async () => {
  for (const fetchImpl of [async () => { throw new Error(sas); }, async () => ({ok: false, status: 403, text: async () => "client-secret"})]) {
    const client = createStoreClient(credentials, {fetchImpl});
    await assert.rejects(client.authenticate(), error => !/secret-sas|client-secret|blob\.core/.test(error.message));
  }
  assert.throws(() => createStoreClient({...credentials, tenantId: "invalid"}));
});
