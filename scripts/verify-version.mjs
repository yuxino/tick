import { readFile } from "node:fs/promises";

const [packageJson, packageLock, tauriConfig, cargoToml, cargoLock] = await Promise.all([
  readJson("package.json"),
  readJson("package-lock.json"),
  readJson("src-tauri/tauri.conf.json"),
  readText("src-tauri/Cargo.toml"),
  readText("src-tauri/Cargo.lock"),
]);

const cargoPackageVersion = cargoToml.match(
  /^\[package\][\s\S]*?^version\s*=\s*"([^"]+)"/m,
)?.[1];
const cargoLockVersion = cargoLock.match(
  /^\[\[package\]\]\s*\nname\s*=\s*"tick"\s*\nversion\s*=\s*"([^"]+)"/m,
)?.[1];
const expected = packageJson.version;
const versions = new Map([
  ["package.json", expected],
  ["package-lock.json", packageLock.version],
  ['package-lock.json packages[""]', packageLock.packages?.[""]?.version],
  ["src-tauri/tauri.conf.json", tauriConfig.version],
  ["src-tauri/Cargo.toml", cargoPackageVersion],
  ["src-tauri/Cargo.lock", cargoLockVersion],
]);

const mismatches = [...versions].filter(([, version]) => version !== expected);
if (!expected || mismatches.length > 0) {
  const details = mismatches
    .map(([source, version]) => `${source}: ${version ?? "missing"}`)
    .join("\n");
  throw new Error(`Tick version sources must all match package.json (${expected ?? "missing"}).\n${details}`);
}

const refName = process.env.GITHUB_REF_NAME;
const isTag =
  process.env.GITHUB_REF_TYPE === "tag" || process.env.GITHUB_REF?.startsWith("refs/tags/");
if (isTag && refName !== `v${expected}`) {
  throw new Error(`Release tag ${refName ?? "missing"} does not match Tick v${expected}.`);
}

console.log(`Tick version verified: ${expected}${isTag ? ` (${refName})` : ""}`);

async function readJson(path) {
  return JSON.parse(await readText(path));
}

async function readText(path) {
  return readFile(new URL(`../${path}`, import.meta.url), "utf8");
}
