import {test} from "node:test";
import assert from "node:assert/strict";
import {checkStoreConnection} from "./check-store-connection.mjs";

const identity = {productId: "9NFDCGL30V3K", name: "NakYmi.DiscordtoVR", publisher: "CN=test"};
function fixture({app = {}, published = {}} = {}) {
  const calls = [];
  const root = `/applications/${identity.productId}`;
  return {calls, client: {
    async authenticate() { calls.push("AUTH"); },
    async request(path, options) {
      // Any mutation, including an explicit POST/PUT, fails this fixture.
      assert.equal(options, undefined);
      calls.push(path);
      if (path === root) return {id: identity.productId, packageIdentityName: identity.name, publisherName: identity.publisher, lastPublishedApplicationSubmission: {id: "123"}, ...app};
      assert.equal(path, `${root}/submissions/123`);
      return {status: "Published", pricing: {priceId: "Free"}, listings: {"ja-jp": {baseListing: {}}}, applicationPackages: [{architecture: "X64", version: "0.2.1.0", fileName: "app.msix", fileStatus: "Uploaded"}], ...published};
    },
  }};
}

test("connection verification authenticates and only reads the configured published product", async () => {
  const f = fixture();
  assert.deepEqual(await checkStoreConnection(identity, f), {productId: identity.productId, submissionId: "123", versions: ["0.2.1.0"]});
  assert.deepEqual(f.calls, ["AUTH", `/applications/${identity.productId}`, `/applications/${identity.productId}/submissions/123`]);
});

test("wrong identity, first publication and pending submissions fail without reading or changing a draft", async () => {
  for (const app of [{id: "OTHER"}, {publisherName: "CN=other"}, {packageIdentityName: "Other"}, {lastPublishedApplicationSubmission: null}, {lastPublishedApplicationSubmission: {id: "../other"}}, {pendingApplicationSubmission: {id: "draft"}}]) {
    const f = fixture({app});
    await assert.rejects(checkStoreConnection(identity, f));
    assert.equal(f.calls.length, 2);
  }
});

test("unpublished, paid and incomplete metadata cannot pass the connection gate", async () => {
  for (const published of [{status: "Certification"}, {pricing: {priceId: "Tier2"}}, {pricing: {priceId: "Free", marketSpecificPricings: {US: "Tier2"}}}, {listings: {}}, {applicationPackages: []}, {applicationPackages: [{architecture: "ARM64", version: "0.2.1.0", fileName: "arm.msix", fileStatus: "Uploaded"}]}, {applicationPackages: [{architecture: "X64", version: "unknown", fileName: "app.msix", fileStatus: "Uploaded"}]}]) {
    await assert.rejects(checkStoreConnection(identity, fixture({published})));
  }
});

test("API Release and Base pricing aliases can be checked without changing published metadata", async () => {
  const published = {status: "Release", pricing: {priceId: "Base"}};
  await checkStoreConnection(identity, fixture({published}));
  assert.equal(published.pricing.priceId, "Base");
});

test("authentication failure stops before any Store API operation", async () => {
  let requested = false;
  await assert.rejects(checkStoreConnection(identity, {client: {authenticate: async () => {throw new Error("authentication failed");}, request: async () => {requested = true;}}}), /authentication failed/);
  assert.equal(requested, false);
});
