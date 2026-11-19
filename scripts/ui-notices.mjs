import {readFileSync,writeFileSync,readdirSync,existsSync} from "node:fs";
import {resolve,join} from "node:path";
import {fileURLToPath} from "node:url";

// Windows CIのcheckoutはCRLFになり得るため、改行コードの違いでは不一致にしない。
const lf=value=>value.split("\r\n").join("\n");
const root=fileURLToPath(new URL("../",import.meta.url));
const lock=JSON.parse(readFileSync(join(root,"package-lock.json"),"utf8"));
const notices=[];
for(const [path,entry] of Object.entries(lock.packages)) {
  if(!path || entry.dev || !path.startsWith("node_modules/"))continue;
  const directory=resolve(root,path);
  if(!existsSync(directory))throw new Error(`Install dependencies before generating notices: ${path}`);
  const files=readdirSync(directory).filter(name=>/^licen[cs]e(?:\.|$)/i.test(name)).sort();
  if(!files.length)throw new Error(`No license file found for ${path}`);
  const name=path.slice(path.lastIndexOf("node_modules/")+13);
  notices.push(`${name} ${entry.version}\n${files.map(file=>lf(readFileSync(join(directory,file),"utf8")).trim()).join("\n\n")}`);
}
const text="DiToVR - third-party UI notices\n\nThese libraries are used as part of the DiToVR application.\nAnimate UI 2.4.2 uses MIT + Commons Clause; its components may not be sold or redistributed as standalone components.\n\n"+notices.join("\n\n"+"=".repeat(72)+"\n\n")+"\n";
const output=join(root,"THIRD_PARTY_UI_NOTICES.txt");
if(process.argv.includes("--check")) {
  if(!existsSync(output)||lf(readFileSync(output,"utf8"))!==text)throw new Error("UI license notices need regeneration: node scripts/ui-notices.mjs");
} else writeFileSync(output,text);
console.log(`UI license notices: ${notices.length} packages`);
