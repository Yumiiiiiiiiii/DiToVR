import test from "node:test";
import assert from "node:assert/strict";
import {storeHasPublished} from "./store-release-ready.mjs";

test("a tag is ready only when the Store has published that version or newer", () => {
  assert.equal(storeHasPublished("v0.3.1", ["0.3.0.0"]), false);
  assert.equal(storeHasPublished("v0.3.1", ["0.3.1.0"]), true);
  assert.equal(storeHasPublished("v0.3.1", ["0.4.0.0"]), true);
  assert.equal(storeHasPublished("v0.3.10", ["0.3.9.0"]), false);
  assert.equal(storeHasPublished("v1.0.0", ["0.99.99.0"]), false);
});

test("malformed tags and versions are never treated as published", () => {
  assert.throws(() => storeHasPublished("store-v0.3.1", ["0.3.1.0"]));
  assert.equal(storeHasPublished("v0.3.1", ["0.3.1", "x.y.z.w"]), false);
});
