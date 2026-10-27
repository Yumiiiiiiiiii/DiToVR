import {readFile, appendFile} from "node:fs/promises";
import {resolve} from "node:path";
import {fileURLToPath} from "node:url";
import {createStoreClient, validateApp} from "./publish-store.mjs";

// This check never creates, uploads, commits or deletes a submission.
export async function checkStoreConnection(identity, {client}) {
  await client.authenticate();
  const root = `/applications/${identity.productId}`;
  const app = await client.request(root);
  validateApp(app, identity);
  const id = app.lastPublishedApplicationSubmission.id;
  if (!/^\d+$/.test(id)) throw new Error("Invalid published Store submission ID.");
  const published = await client.request(`${root}/submissions/${id}`);
  if (!["Published", "Release"].includes(published.status)) throw new Error("The latest Store submission is not published yet.");
  if (!published.listings?.["ja-jp"]?.baseListing) throw new Error("The published Japanese Store listing is missing.");
  if (!["Free", "Base"].includes(published.pricing?.priceId) || Object.values(published.pricing.marketSpecificPricings ?? {}).some(price => !["Free", "Base", "NotAvailable"].includes(price))) throw new Error("Automatic submission supports only this free product. Check pricing in Partner Center.");
  const packages = published.applicationPackages;
  if (!Array.isArray(packages) || !packages.length || packages.some(pkg => pkg.fileStatus !== "Uploaded" || !pkg.fileName || !/^\d+\.\d+\.\d+\.\d+$/.test(pkg.version ?? "") || !pkg.architecture || pkg.architecture.toLowerCase() === "neutral")) throw new Error("Published Store packages are incomplete or require manual review.");
  const x64 = packages.filter(pkg => pkg.architecture.toLowerCase() === "x64");
  if (!x64.length) throw new Error("No published x64 Store package exists.");
  return {productId: identity.productId, submissionId: id, versions: x64.map(pkg => pkg.version)};
}

async function main() {
  const identity = JSON.parse(await readFile("store/identity.json", "utf8"));
  const mask = value => console.log(`::add-mask::${value.replaceAll("%", "%25").replaceAll("\r", "%0D").replaceAll("\n", "%0A")}`);
  const client = createStoreClient({tenantId: process.env.AZURE_AD_TENANT_ID, clientId: process.env.AZURE_AD_APPLICATION_CLIENT_ID, clientSecret: process.env.AZURE_AD_APPLICATION_SECRET}, {mask});
  const result = await checkStoreConnection(identity, {client});
  const message = `Microsoft Store connection verified (read only). Product: ${result.productId}; published submission: ${result.submissionId}; x64: ${result.versions.join(", ")}. No update was submitted.`;
  console.log(message);
  if (process.env.GITHUB_STEP_SUMMARY) await appendFile(process.env.GITHUB_STEP_SUMMARY, `## Microsoft Store connection\n\n${message}\n\nAuthentication, product identity and published metadata passed. Submission permission and the next package will be checked when a new version tag is pushed.\n`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { await main(); } catch (error) { console.error(error.message); process.exitCode = 1; }
}
