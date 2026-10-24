import {test} from "node:test";
import assert from "node:assert/strict";
import {storeManifest} from "./store-manifest.mjs";
const identity={name:"Example.DiscordtoVR",publisher:"CN=test-publisher",publisherDisplayName:'Name & "Co"'};
test("Store package uses supplied identity, escaped XML, desktop full trust and Store version format",()=>{
  const manifest=storeManifest(identity,"0.2.1");
  assert.match(manifest,/Version="0\.2\.1\.0"/);
  assert.match(manifest,/Name &amp; &quot;Co&quot;/);
  assert.match(manifest,/Windows.FullTrustApplication/);
  assert.match(manifest,/runFullTrust/);
  assert.doesNotMatch(manifest,/allowElevation|autoUpdate|config.json|updater.key/);
});
test("invalid Store identities and versions cannot create a package",()=>{
  for (const version of ["0.0.0","65536.1.0","0.2.1-beta","0.2.1.1","../1"]) assert.throws(()=>storeManifest(identity,version));
  for (const name of ["","../bad","invalid name"]) assert.throws(()=>storeManifest({...identity,name},"0.2.1"));
  assert.throws(()=>storeManifest({...identity,publisher:""},"0.2.1"));
});
