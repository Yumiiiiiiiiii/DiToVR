import {readFile, appendFile} from "node:fs/promises";
import {basename, join, resolve} from "node:path";
import {fileURLToPath} from "node:url";
import {createHash} from "node:crypto";
import {execFileSync} from "node:child_process";
import {setTimeout as sleep} from "node:timers/promises";
import {singleFileZip, zipEntry} from "./store-archive.mjs";
import {storeManifest} from "./store-manifest.mjs";
import {repository, validateVersion} from "./release-manifest.mjs";

const apiRoot = "https://manage.devcenter.microsoft.com/v1.0/my";
const accepted = new Set(["PreProcessing", "Certification", "PendingPublication", "Publishing", "Release", "Published"]);
const failed = new Set(["Canceled", "CommitFailed", "PreProcessingFailed", "CertificationFailed", "PublishFailed", "ReleaseFailed"]);
const guid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

export function storeReleaseTag(env) {
  if (env.GITHUB_REPOSITORY !== repository || env.MICROSOFT_STORE_PUBLISH_ENABLED !== "true") throw new Error("Store submission requires this repository and explicit publication enablement.");
  const automatic = env.GITHUB_EVENT_NAME === "push" && env.GITHUB_REF_TYPE === "tag";
  const retry = env.GITHUB_EVENT_NAME === "workflow_dispatch" && env.STORE_SUBMISSION_TASK === "retry-submission";
  const tag = automatic ? env.GITHUB_REF_NAME : retry ? env.STORE_RELEASE_TAG : undefined;
  if (!/^v\d+\.\d+\.\d+$/.test(tag ?? "")) throw new Error("Store submission requires a version tag push or an explicit retry of an existing vX.Y.Z tag.");
  return tag;
}

function versionParts(value) {
  if (!/^\d+\.\d+\.\d+\.\d+$/.test(value ?? "") || value.split(".").some(part => Number(part) > 65535)) throw new Error("Invalid Store package version.");
  return value.split(".").map(Number);
}

function newer(version, previous) {
  const a = versionParts(version), b = versionParts(previous);
  for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return a[i] > b[i];
  return false;
}

export function validateArtifact({identity, version, fileName, msix, report, notes}) {
  storeManifest(identity, version); // Also validates the configured identity and MSIX version range.
  if (!/^[A-Z0-9]{12}$/.test(identity.productId ?? "") || fileName !== `DiToVR_${version}.0_x64.msix`) throw new Error("Unexpected Store product or MSIX filename.");
  if (typeof notes !== "string" || !notes.trim() || notes.trim().length > 1500) throw new Error("Store release notes must contain 1-1500 characters of changes.");
  if (!Array.isArray(report?.Files) || !report.Files.length || report.Files.some(file => file.ScannerExitCode !== 0) || !report.EngineVersion || !report.SignatureVersion) throw new Error("A successful Defender security report is required.");
  const sha256 = createHash("sha256").update(msix).digest("hex");
  const scans = report.Files.filter(file => file.File === fileName);
  if (scans.length !== 1 || scans[0].SHA256 !== sha256 || !report.Files.some(file => file.File === "DiToVR.exe")) throw new Error("The MSIX does not match the Defender security report.");
  // Compare against the actual manifest inside the scanned MSIX, not a sidecar.
  const manifest = zipEntry(msix, "AppxManifest.xml").toString("utf8").replaceAll("\r\n", "\n").replace(/^\uFEFF/, "");
  if (manifest !== storeManifest(identity, version).replaceAll("\r\n", "\n")) throw new Error("MSIX identity, version or manifest differs from the configured Store build.");
  return sha256;
}

export function validateApp(app, identity) {
  if (app?.id !== identity.productId || app.packageIdentityName !== identity.name || app.publisherName?.toUpperCase() !== identity.publisher.toUpperCase()) throw new Error("Partner Center product identity does not match store/identity.json.");
  if (!app.lastPublishedApplicationSubmission?.id) throw new Error("Publish the first Store submission in Partner Center before enabling automatic updates.");
  if (app.pendingApplicationSubmission) throw new Error("A Store draft or submission is already pending. Resolve it in Partner Center; it will not be deleted or overwritten.");
}

function submissionId(value) {
  if (!/^\d+$/.test(value ?? "")) throw new Error("Invalid Store submission ID.");
  return value;
}

export function updateSubmission(source, {version, fileName, notes}) {
  const draft = structuredClone(source);
  if (!draft.listings?.["ja-jp"]?.baseListing) throw new Error("A published Japanese Store listing is required.");
  // This product is distributed for free. Pricing v2 can return Base even for a
  // free app; PUT requires the explicit Free tier. Never accept a paid tier.
  if (!["Free", "Base"].includes(draft.pricing?.priceId)) throw new Error("Automatic submission supports only this free product. Check pricing in Partner Center.");
  draft.pricing.priceId = "Free";
  if (Object.values(draft.pricing.marketSpecificPricings ?? {}).some(price => !["Free", "Base", "NotAvailable"].includes(price))) throw new Error("Paid market overrides must be reviewed in Partner Center.");
  const packages = draft.applicationPackages;
  if (!Array.isArray(packages) || !packages.length || packages.some(pkg => pkg.fileStatus !== "Uploaded")) throw new Error("Expected only published Store packages.");
  const oldX64 = packages.filter(pkg => pkg.architecture?.toLowerCase() === "x64");
  if (!oldX64.length) throw new Error("No published x64 package exists; review the initial Store submission.");
  for (const pkg of packages) {
    if (!newer(`${version}.0`, pkg.version)) throw new Error("The new Store version must be greater than every published package version.");
    if (!pkg.architecture || !pkg.fileName) throw new Error("Published package metadata is incomplete.");
    if (pkg.architecture.toLowerCase() === "neutral") throw new Error("A neutral/bundle package requires manual review before replacing x64 packages.");
  }
  for (const pkg of oldX64) pkg.fileStatus = "PendingDelete";
  packages.push({fileName, fileStatus: "PendingUpload", minimumDirectXVersion: "None", minimumSystemRam: "None"});
  draft.listings["ja-jp"].baseListing.releaseNotes = notes.trim();
  draft.targetPublishMode = "Immediate";
  delete draft.targetPublishDate;
  // Read-only response fields (especially the SAS URL) do not belong in PUT.
  for (const key of ["id", "status", "statusDetails", "fileUploadUrl", "friendlyName"]) delete draft[key];
  delete draft.pricing.isAdvancedPricingModel;
  return draft;
}

export function uploadUrl(value) {
  let url;
  try { url = new URL(value); } catch { throw new Error("Store returned an invalid upload URL."); }
  if (url.protocol !== "https:" || !/^[a-z0-9]+\.blob\.core\.windows\.net$/.test(url.hostname) || url.username || url.password || url.port || !url.searchParams.get("sig")) throw new Error("Store upload requires an HTTPS Azure Blob SAS URL.");
  return url.href;
}

// The transport deliberately excludes response bodies and URLs from errors:
// authentication tokens and Azure SAS query strings must never enter CI logs.
export function createStoreClient({tenantId, clientId, clientSecret}, {fetchImpl = fetch, mask = () => {}} = {}) {
  if (!guid.test(tenantId ?? "") || !guid.test(clientId ?? "") || !clientSecret?.trim()) throw new Error("Set AZURE_AD_TENANT_ID, AZURE_AD_APPLICATION_CLIENT_ID and AZURE_AD_APPLICATION_SECRET in GitHub Secrets.");
  let token;
  async function send(url, options, label, timeout = 120000) {
    let response;
    try { response = await fetchImpl(url, {...options, redirect: "error", signal: AbortSignal.timeout(timeout)}); }
    catch { throw new Error(`${label} could not complete. Check Partner Center before retrying; any draft is retained.`); }
    if (!response.ok) throw new Error(`${label} failed (HTTP ${response.status}). Check Partner Center; any draft is retained.`);
    return response;
  }
  async function json(response) {
    try { return await response.json(); } catch { throw new Error("Store returned an invalid JSON response."); }
  }
  return {
    async authenticate() {
      const response = await send(`https://login.microsoftonline.com/${tenantId}/oauth2/token`, {
        method: "POST", headers: {"Content-Type": "application/x-www-form-urlencoded"},
        body: new URLSearchParams({grant_type: "client_credentials", client_id: clientId, client_secret: clientSecret, resource: "https://manage.devcenter.microsoft.com"}),
      }, "Store authentication");
      token = (await json(response)).access_token;
      if (typeof token !== "string" || !token) throw new Error("Store authentication did not return an access token.");
      mask(token);
    },
    async request(path, {method = "GET", body} = {}) {
      if (!token || !/^\/applications\/[A-Z0-9]{12}(?:\/submissions(?:\/\d+(?:\/(?:commit|status))?)?)?$/.test(path) || !["GET", "POST", "PUT"].includes(method)) throw new Error("Unexpected Store API operation.");
      return json(await send(apiRoot + path, {
        method, headers: {Authorization: `Bearer ${token}`, ...(body ? {"Content-Type": "application/json"} : {})},
        ...(body ? {body: JSON.stringify(body)} : {}),
      }, "Store API request"));
    },
    async upload(url, zip) {
      const safeUrl = uploadUrl(url);
      mask(safeUrl);
      await send(safeUrl, {method: "PUT", headers: {"x-ms-blob-type": "BlockBlob", "Content-Type": "application/zip"}, body: zip}, "Store package upload", 600000);
    },
  };
}

export async function publishStore(artifact, {client, wait = sleep, log = () => {}, attempts = 40, interval = 15000}) {
  const sha256 = validateArtifact(artifact);
  const {identity, version, fileName, notes, msix} = artifact;
  const root = `/applications/${identity.productId}`;
  await client.authenticate();
  const app = await client.request(root);
  validateApp(app, identity);
  const published = await client.request(`${root}/submissions/${submissionId(app.lastPublishedApplicationSubmission.id)}`);
  updateSubmission(published, {version, fileName, notes}); // All known problems fail before creating a draft.
  const zip = singleFileZip(fileName, msix);
  // Recheck just before creating. POST also rejects a competing submission.
  const current = await client.request(root);
  validateApp(current, identity);
  if (current.lastPublishedApplicationSubmission.id !== app.lastPublishedApplicationSubmission.id) throw new Error("The published Store version changed during this run; rerun after reviewing it.");
  const draft = await client.request(`${root}/submissions`, {method: "POST"});
  const id = submissionId(draft.id);
  log(`Created Store submission ${id}. It will be retained if this run fails.`);
  const url = uploadUrl(draft.fileUploadUrl);
  const updated = updateSubmission(draft, {version, fileName, notes});
  const path = `${root}/submissions/${id}`;
  await client.request(path, {method: "PUT", body: updated});
  await client.upload(url, zip);
  const commit = await client.request(`${path}/commit`, {method: "POST"});
  if (failed.has(commit.status) || !["CommitStarted", ...accepted].includes(commit.status)) throw new Error(`Store did not accept the commit for submission ${id}. Check Partner Center.`);
  for (let i = 0; i < attempts; i++) {
    const status = await client.request(`${path}/status`);
    if (accepted.has(status.status)) return {id, status: status.status, version, sha256};
    if (failed.has(status.status)) throw new Error(`Store submission ${id} failed: ${status.status}. Check Partner Center.`);
    if (!["PendingCommit", "CommitStarted"].includes(status.status)) throw new Error(`Store submission ${id} returned an unknown status. Check Partner Center.`);
    if (i + 1 < attempts) await wait(interval);
  }
  throw new Error(`Store submission ${id} was committed but acceptance is not confirmed. Check Partner Center before retrying.`);
}

async function main() {
  const tag = storeReleaseTag(process.env);
  const gitCommit = ref => execFileSync("git", ["rev-parse", "--verify", ref], {encoding: "utf8", stdio: ["ignore", "pipe", "pipe"]}).trim();
  if (gitCommit("HEAD") !== gitCommit(`refs/tags/${tag}^{commit}`)) throw new Error("The checked-out Store source does not match the requested release tag.");
  const config = JSON.parse(await readFile("src-tauri/tauri.conf.json", "utf8"));
  const version = validateVersion(config, await readFile("src-tauri/Cargo.toml", "utf8"), JSON.parse(await readFile("package.json", "utf8")), tag);
  const notes = await readFile(join("docs/release-notes", `${tag}.md`), "utf8");
  if (!notes.trim() || notes.trim().length > 1500) throw new Error("Store release notes must contain 1-1500 characters of changes.");
  if (process.argv.includes("--validate-release")) {
    console.log(`Store source, version and release notes verified against ${tag}. No update was submitted.`);
    return;
  }
  const packagePath = process.env.STORE_PACKAGE_PATH;
  const reportPath = process.env.STORE_SECURITY_REPORT_PATH;
  if (!packagePath || !reportPath) throw new Error("Store build outputs are missing.");
  const artifact = {
    identity: JSON.parse(await readFile("store/identity.json", "utf8")), version,
    fileName: basename(packagePath), msix: await readFile(packagePath),
    report: JSON.parse(await readFile(reportPath, "utf8")),
    notes,
  };
  const mask = value => console.log(`::add-mask::${value.replaceAll("%", "%25").replaceAll("\r", "%0D").replaceAll("\n", "%0A")}`);
  const client = createStoreClient({tenantId: process.env.AZURE_AD_TENANT_ID, clientId: process.env.AZURE_AD_APPLICATION_CLIENT_ID, clientSecret: process.env.AZURE_AD_APPLICATION_SECRET}, {mask});
  const result = await publishStore(artifact, {client, log: console.log});
  console.log(`Store submission ${result.id} accepted: ${result.status}. Certification and public availability are separate steps.`);
  if (process.env.GITHUB_STEP_SUMMARY) await appendFile(process.env.GITHUB_STEP_SUMMARY, `## Microsoft Store\n\n- Version: ${result.version}.0\n- Submission: ${result.id}\n- Status: ${result.status}\n- MSIX SHA-256: ${result.sha256}\n\nAccepted for processing; certification and publication may still be pending.\n`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { await main(); } catch (error) { console.error(error.message); process.exitCode = 1; }
}
