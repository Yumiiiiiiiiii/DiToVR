import test from "node:test";
import assert from "node:assert/strict";
import {storeHasPublished} from "./store-release-ready.mjs";

test("a tag is ready only when the Store has published exactly that version", () => {
  assert.equal(storeHasPublished("v0.3.1", ["0.3.0.0"]), false);
  assert.equal(storeHasPublished("v0.3.1", ["0.3.1.0"]), true);
  assert.equal(storeHasPublished("v0.3.1", ["0.3.1.2"]), true);
  assert.equal(storeHasPublished("v0.3.1", ["0.3.0.0", "0.3.1.0"]), true);
  assert.equal(storeHasPublished("v0.3.10", ["0.3.9.0"]), false);
  assert.equal(storeHasPublished("v1.0.0", ["0.99.99.0"]), false);
});

test("a newer or older published version never makes a tag ready", () => {
  assert.equal(storeHasPublished("v0.3.1", ["0.4.0.0"]), false);
  assert.equal(storeHasPublished("v0.4.2", ["0.4.3.0"]), false);
  assert.equal(storeHasPublished("v0.4.2", ["1.0.0.0"]), false);
  assert.equal(storeHasPublished("v0.4.2", ["0.4.1.0"]), false);
  assert.equal(storeHasPublished("v0.4.2", []), false);
});

test("malformed tags and versions are never treated as published", () => {
  assert.throws(() => storeHasPublished("store-v0.3.1", ["0.3.1.0"]));
  assert.throws(() => storeHasPublished(undefined, ["0.3.1.0"]));
  assert.equal(storeHasPublished("v0.3.1", ["0.3.1", "x.y.z.w"]), false);
  assert.equal(storeHasPublished("v0.3.1", ["0.3.1.0.0"]), false);
});
