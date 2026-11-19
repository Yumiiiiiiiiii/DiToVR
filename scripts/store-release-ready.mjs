import {readFile, appendFile} from "node:fs/promises";
import {resolve} from "node:path";
import {fileURLToPath} from "node:url";
import {createStoreClient} from "./publish-store.mjs";
import {checkStoreConnection} from "./check-store-connection.mjs";

// タグ vX.Y.Z のStore版(X.Y.Z.0)が一般公開済みか判定する。読み取り専用で、申請や変更は行わない。
// lastPublishedApplicationSubmission は審査中の申請を含まないため、公開済みの版だけが比較される。
export function storeHasPublished(tag, publishedVersions) {
  const match = /^v(\d+)\.(\d+)\.(\d+)$/.exec(tag ?? "");
  if (!match) throw new Error("Expected a stable vX.Y.Z tag.");
  const wanted = match.slice(1).map(Number);
  return publishedVersions.some(version => {
    const parts = version.split(".").map(Number);
    if (parts.length !== 4 || parts.some(Number.isNaN)) return false;
    for (let i = 0; i < 3; i++) if (parts[i] !== wanted[i]) return parts[i] > wanted[i];
    return true; // 同じ X.Y.Z (リビジョン違いを含む) は公開済みとみなす
  });
}

async function main() {
  const tag = process.env.RELEASE_TAG;
  const identity = JSON.parse(await readFile("store/identity.json", "utf8"));
  const mask = value => console.log(`::add-mask::${value.replaceAll("%", "%25").replaceAll("\r", "%0D").replaceAll("\n", "%0A")}`);
  const client = createStoreClient({tenantId: process.env.AZURE_AD_TENANT_ID, clientId: process.env.AZURE_AD_APPLICATION_CLIENT_ID, clientSecret: process.env.AZURE_AD_APPLICATION_SECRET}, {mask});
  const result = await checkStoreConnection(identity, {client});
  const ready = storeHasPublished(tag, result.versions);
  const message = ready
    ? `Microsoft Store has published ${tag} (x64 ${result.versions.join(", ")}). GitHub release may proceed.`
    : `Microsoft Store has not published ${tag} yet (published x64: ${result.versions.join(", ")}). GitHub release is held.`;
  console.log(message);
  if (process.env.GITHUB_OUTPUT) await appendFile(process.env.GITHUB_OUTPUT, `ready=${ready}\n`);
  if (process.env.GITHUB_STEP_SUMMARY) await appendFile(process.env.GITHUB_STEP_SUMMARY, `## Store publication gate\n\n${message}\n`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { await main(); } catch (error) { console.error(error.message); process.exitCode = 1; }
}
