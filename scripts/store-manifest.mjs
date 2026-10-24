import {readFileSync, writeFileSync, mkdirSync, copyFileSync, existsSync} from "node:fs";
import {resolve, join, dirname} from "node:path";
import {fileURLToPath} from "node:url";
import {validateVersion} from "./release-manifest.mjs";

const xml=value=>String(value).replaceAll("&","&amp;").replaceAll('"',"&quot;").replaceAll("<","&lt;").replaceAll(">","&gt;").replaceAll("'","&apos;");
export function storeManifest(identity, version) {
  if (!/^[A-Za-z0-9][A-Za-z0-9.-]{2,49}$/.test(identity.name??"") || !/^CN=\S.+$/.test(identity.publisher??"") || !identity.publisherDisplayName?.trim()) throw new Error("Use the exact Partner Center package identity.");
  if (!/^\d+\.\d+\.\d+$/.test(version) || version.split(".").some(v=>Number(v)>65535) || version.split(".").every(v=>Number(v)===0)) throw new Error("MSIX requires a stable three-part app version, each part <= 65535.");
  return `<?xml version="1.0" encoding="utf-8"?>
<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
 xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10"
 xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities"
 IgnorableNamespaces="uap rescap">
 <Identity Name="${xml(identity.name)}" Publisher="${xml(identity.publisher)}" Version="${version}.0" ProcessorArchitecture="x64" />
 <Properties>
  <DisplayName>Discord to VR</DisplayName>
  <PublisherDisplayName>${xml(identity.publisherDisplayName)}</PublisherDisplayName>
  <Description>Discord notifications in VR with XSOverlay</Description>
  <Logo>Assets\\StoreLogo.png</Logo>
 </Properties>
 <Resources><Resource Language="ja-jp" /></Resources>
 <Dependencies><TargetDeviceFamily Name="Windows.Desktop" MinVersion="10.0.19041.0" MaxVersionTested="10.0.26100.0" /></Dependencies>
 <Applications>
  <Application Id="DiscordToVR" Executable="discord_To_VR.exe" EntryPoint="Windows.FullTrustApplication">
   <uap:VisualElements DisplayName="Discord to VR" Description="Discord notifications in VR with XSOverlay"
    BackgroundColor="transparent" Square150x150Logo="Assets\\Square150x150Logo.png" Square44x44Logo="Assets\\Square44x44Logo.png" />
  </Application>
 </Applications>
 <Capabilities><rescap:Capability Name="runFullTrust" /></Capabilities>
</Package>
`;
}

if (process.argv[1] && resolve(process.argv[1])===fileURLToPath(import.meta.url)) {
  const [stage, exe, icons] = process.argv.slice(2);
  if (!stage||!exe||!icons) throw new Error("Usage: node scripts/store-manifest.mjs STAGE EXE ICONS");
  const root=fileURLToPath(new URL("../",import.meta.url));
  const identity=JSON.parse(readFileSync(join(root,"store/identity.json"),"utf8"));
  const config=JSON.parse(readFileSync(join(root,"src-tauri/tauri.conf.json"),"utf8"));
  const version=validateVersion(config,readFileSync(join(root,"src-tauri/Cargo.toml"),"utf8"),JSON.parse(readFileSync(join(root,"package.json"),"utf8")));
  const binary=readFileSync(exe);
  const peOffset=binary.length>=64?binary.readUInt32LE(0x3c):0;
  if (peOffset<64 || peOffset+6>binary.length || binary.toString("ascii",peOffset,peOffset+4)!=="PE\0\0" || binary.readUInt16LE(peOffset+4)!==0x8664) throw new Error("The Store package requires an x64 Windows executable.");
  // Explicit allowlist: never copy a build directory, user config, or signing key.
  mkdirSync(join(stage,"Assets"),{recursive:true});
  writeFileSync(join(stage,"AppxManifest.xml"),storeManifest(identity,version));
  copyFileSync(exe,join(stage,"discord_To_VR.exe"));
  // GNU builds dynamically link Microsoft's WebView2 loader. MSVC may link it statically.
  const loader=join(dirname(exe),"WebView2Loader.dll");
  if (existsSync(loader)) copyFileSync(loader,join(stage,"WebView2Loader.dll"));
  copyFileSync(join(root,"LICENSE"),join(stage,"LICENSE.txt"));
  for (const file of ["StoreLogo.png","Square150x150Logo.png","Square44x44Logo.png"]) copyFileSync(join(icons,file),join(stage,"Assets",file));
}
