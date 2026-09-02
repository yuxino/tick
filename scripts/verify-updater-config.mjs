import { readFile } from "node:fs/promises";

const [packageJson, tauriConfig, capability, cargoToml] = await Promise.all([
  readJson("package.json"),
  readJson("src-tauri/tauri.conf.json"),
  readJson("src-tauri/capabilities/default.json"),
  readText("src-tauri/Cargo.toml"),
]);

const expectedEndpoint = "https://github.com/yuxino/tick/releases/latest/download/latest.json";
const updater = tauriConfig.plugins?.updater;
if (tauriConfig.bundle?.createUpdaterArtifacts !== true) {
  throw new Error("Tauri must create v2 updater artifacts.");
}
if (JSON.stringify(updater?.endpoints) !== JSON.stringify([expectedEndpoint])) {
  throw new Error("Updater endpoints must contain only Tick's HTTPS latest.json URL.");
}

const publicKeyText = Buffer.from(updater?.pubkey ?? "", "base64").toString("utf8");
if (!publicKeyText.startsWith("untrusted comment: minisign public key:") || publicKeyText.trim().split(/\r?\n/).length !== 2) {
  throw new Error("Updater public key is missing or malformed.");
}

const permissions = capability.permissions ?? [];
for (const permission of ["updater:default", "process:allow-restart"]) {
  if (!permissions.includes(permission)) {
    throw new Error(`Missing updater capability: ${permission}`);
  }
}
const opener = permissions.find((permission) => typeof permission === "object" && permission.identifier === "opener:allow-open-url");
if (JSON.stringify(opener?.allow) !== JSON.stringify([{ url: "https://github.com/yuxino/tick/releases" }])) {
  throw new Error("Recovery URL capability must be restricted to Tick Releases.");
}

for (const name of ["opener", "process", "updater"]) {
  if (!packageJson.dependencies?.[`@tauri-apps/plugin-${name}`]) {
    throw new Error(`Missing JavaScript plugin dependency: ${name}`);
  }
  if (!new RegExp(`^tauri-plugin-${name}\\s*=`, "m").test(cargoToml)) {
    throw new Error(`Missing Rust plugin dependency: ${name}`);
  }
}

await readText(`docs/release-notes/v${packageJson.version}.md`);
console.log(`Updater configuration verified for Tick v${packageJson.version}.`);

async function readJson(path) {
  return JSON.parse(await readText(path));
}

async function readText(path) {
  return readFile(new URL(`../${path}`, import.meta.url), "utf8");
}
