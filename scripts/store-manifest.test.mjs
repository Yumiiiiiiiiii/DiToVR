import {test} from "node:test";
import assert from "node:assert/strict";
import {storeManifest} from "./store-manifest.mjs";
import {mkdtempSync,mkdirSync,writeFileSync,readFileSync,readdirSync,rmSync} from "node:fs";
import {tmpdir} from "node:os";
import {join,dirname} from "node:path";
import {fileURLToPath} from "node:url";
import {execFileSync} from "node:child_process";
const identity={name:"Example.DiToVR",publisher:"CN=test-publisher",publisherDisplayName:'Name & "Co"'};
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

test("Store staging includes audio licenses and only the allowed application files",()=>{
  const directory=mkdtempSync(join(tmpdir(),"ditovr-store-audio-test-"));
  try {
    const exe=join(directory,"DiToVR.exe"), icons=join(directory,"icons"), stage=join(directory,"stage");
    const binary=Buffer.alloc(128);
    binary.writeUInt32LE(64,0x3c);
    binary.write("PE\0\0",64,"ascii");
    binary.writeUInt16LE(0x8664,68);
    writeFileSync(exe,binary);
    writeFileSync(join(directory,"config.json"),"must not be packaged");
    mkdirSync(icons);
    for (const file of ["StoreLogo.png","Square150x150Logo.png","Square44x44Logo.png"]) writeFileSync(join(icons,file),"test image");
    execFileSync(process.execPath,[fileURLToPath(new URL("./store-manifest.mjs",import.meta.url)),stage,exe,icons],{stdio:"pipe"});
    assert.deepEqual(readdirSync(stage).sort(),["AppxManifest.xml","Assets","DiToVR.exe","LICENSE.txt","THIRD_PARTY_AUDIO_NOTICES.txt"].sort());
    const root=fileURLToPath(new URL("../",import.meta.url));
    assert.equal(readFileSync(join(stage,"THIRD_PARTY_AUDIO_NOTICES.txt"),"utf8"),readFileSync(join(root,"THIRD_PARTY_AUDIO_NOTICES.txt"),"utf8"));
    const config=JSON.parse(readFileSync(join(root,"src-tauri/tauri.conf.json"),"utf8"));
    assert.equal(config.bundle.resources["../THIRD_PARTY_AUDIO_NOTICES.txt"],"THIRD_PARTY_AUDIO_NOTICES.txt");
  } finally {
    assert.equal(dirname(directory),tmpdir());
    rmSync(directory,{recursive:true,force:true});
  }
});
