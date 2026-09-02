import { createHash, createPublicKey, verify as verifyCryptoSignature } from "node:crypto";
import { readFile, readdir, stat, writeFile } from "node:fs/promises";
import { basename, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

export const UPDATER_PLATFORMS = [
  "darwin-aarch64",
  "windows-x86_64",
  "windows-aarch64",
];

const assetMatchers = {
  "darwin-aarch64": (name) => name.endsWith(".app.tar.gz"),
  "windows-x86_64": (name) => name.endsWith("_x64-setup.exe"),
  "windows-aarch64": (name) => name.endsWith("_arm64-setup.exe"),
};

export async function buildUpdaterManifest({ version, notes, pubDate, baseUrl, assetDir, publicKey }) {
  assertVersion(version);
  const normalizedBaseUrl = assertReleaseBaseUrl(baseUrl, version);
  const files = await readdir(assetDir);
  const platforms = {};

  for (const platform of UPDATER_PLATFORMS) {
    const matches = files.filter((name) => assetMatchers[platform](name));
    if (matches.length !== 1) {
      throw new Error(`Expected exactly one ${platform} updater asset, found ${matches.length}.`);
    }

    const filename = matches[0];
    const signature = (await readFile(join(assetDir, `${filename}.sig`), "utf8")).trim();
    assertSignature(signature, platform);
    await verifyMinisignSignature({ artifactPath: join(assetDir, filename), signature, publicKey });
    platforms[platform] = {
      signature,
      url: new URL(encodeURIComponent(filename), normalizedBaseUrl).toString(),
    };
  }

  return {
    version,
    notes: notes.trim(),
    pub_date: new Date(pubDate).toISOString(),
    platforms,
  };
}

export async function verifyUpdaterManifest({ manifestPath, expectedVersion, assetDir, baseUrl, publicKey }) {
  const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
  assertVersion(expectedVersion);
  if (manifest.version !== expectedVersion) {
    throw new Error(`Updater manifest version ${manifest.version ?? "missing"} does not match ${expectedVersion}.`);
  }

  const normalizedBaseUrl = assertReleaseBaseUrl(baseUrl, expectedVersion);
  const platformKeys = Object.keys(manifest.platforms ?? {}).sort();
  const expectedKeys = [...UPDATER_PLATFORMS].sort();
  if (JSON.stringify(platformKeys) !== JSON.stringify(expectedKeys)) {
    throw new Error(`Updater platforms must be exactly: ${UPDATER_PLATFORMS.join(", ")}.`);
  }

  const seenUrls = new Set();
  const hashes = {};
  for (const platform of UPDATER_PLATFORMS) {
    const entry = manifest.platforms[platform];
    assertSignature(entry.signature, platform);
    const url = new URL(entry.url);
    if (url.protocol !== "https:" || !entry.url.startsWith(normalizedBaseUrl)) {
      throw new Error(`${platform} updater URL is outside the expected HTTPS release path.`);
    }
    if (seenUrls.has(entry.url)) {
      throw new Error(`Updater URL is reused by multiple platforms: ${entry.url}`);
    }
    seenUrls.add(entry.url);

    const filename = decodeURIComponent(basename(url.pathname));
    if (!assetMatchers[platform](filename)) {
      throw new Error(`${platform} updater URL has the wrong architecture-specific filename.`);
    }
    const assetPath = resolve(assetDir, filename);
    const assetStats = await stat(assetPath);
    if (!assetStats.isFile() || assetStats.size === 0) {
      throw new Error(`${platform} updater asset is empty.`);
    }
    const signature = (await readFile(`${assetPath}.sig`, "utf8")).trim();
    if (signature !== entry.signature) {
      throw new Error(`${platform} manifest signature does not match its .sig asset.`);
    }
    await verifyMinisignSignature({ artifactPath: assetPath, signature, publicKey });
    hashes[filename] = createHash("sha256").update(await readFile(assetPath)).digest("hex");
  }

  return hashes;
}

export async function verifyMinisignSignature({ artifactPath, signature, publicKey }) {
  const publicKeyText = decodeOuterBase64(publicKey, "updater public key");
  const publicKeyLines = publicKeyText.trim().split(/\r?\n/);
  if (publicKeyLines.length !== 2 || !publicKeyLines[0].startsWith("untrusted comment: ")) {
    throw new Error("Updater public key is not a complete Minisign public key.");
  }
  const publicKeyBytes = decodeBase64Exact(publicKeyLines[1], 42, "updater public key");
  const signatureText = decodeOuterBase64(signature, "updater signature");
  const signatureLines = signatureText.trim().split(/\r?\n/);
  const signatureBytes = decodeBase64Exact(signatureLines[1], 74, "updater signature");
  const globalSignature = decodeBase64Exact(signatureLines[3], 64, "updater global signature");

  const keyId = publicKeyBytes.subarray(2, 10);
  if (!keyId.equals(signatureBytes.subarray(2, 10))) {
    throw new Error("Updater signature key ID does not match the configured public key.");
  }
  const algorithm = signatureBytes.subarray(0, 2).toString("ascii");
  if (algorithm !== "ED" && algorithm !== "Ed") {
    throw new Error(`Unsupported Minisign signature algorithm: ${algorithm}`);
  }

  const rawPublicKey = publicKeyBytes.subarray(10, 42);
  const spkiPrefix = Buffer.from("302a300506032b6570032100", "hex");
  const key = createPublicKey({ key: Buffer.concat([spkiPrefix, rawPublicKey]), format: "der", type: "spki" });
  const artifact = await readFile(artifactPath);
  const signedData = algorithm === "ED" ? createHash("blake2b512").update(artifact).digest() : artifact;
  const dataSignature = signatureBytes.subarray(10, 74);
  if (!verifyCryptoSignature(null, signedData, key, dataSignature)) {
    throw new Error(`Updater signature verification failed for ${basename(artifactPath)}.`);
  }

  const trustedComment = signatureLines[2].slice("trusted comment: ".length);
  const globalData = Buffer.concat([dataSignature, Buffer.from(trustedComment)]);
  if (!verifyCryptoSignature(null, globalData, key, globalSignature)) {
    throw new Error(`Updater trusted-comment signature verification failed for ${basename(artifactPath)}.`);
  }
}

function assertVersion(version) {
  if (!/^\d+\.\d+\.\d+$/.test(version)) {
    throw new Error(`Updater version must be a patch SemVer without a v prefix: ${version}`);
  }
}

function assertReleaseBaseUrl(baseUrl, version) {
  const normalized = baseUrl.endsWith("/") ? baseUrl : `${baseUrl}/`;
  const url = new URL(normalized);
  if (
    url.protocol !== "https:" ||
    url.hostname !== "github.com" ||
    url.pathname !== `/yuxino/tick/releases/download/v${version}/`
  ) {
    throw new Error(`Unexpected updater release base URL: ${normalized}`);
  }
  return normalized;
}

function assertSignature(signature, platform) {
  if (!/^[A-Za-z0-9+/=]+$/.test(signature) || signature.length < 80) {
    throw new Error(`${platform} updater signature is missing or malformed.`);
  }
  const decoded = Buffer.from(signature, "base64").toString("utf8");
  const lines = decoded.trim().split(/\r?\n/);
  if (
    lines.length !== 4 ||
    !lines[0].startsWith("untrusted comment: ") ||
    !lines[2].startsWith("trusted comment: ") ||
    Buffer.from(lines[1] ?? "", "base64").length !== 74 ||
    Buffer.from(lines[3] ?? "", "base64").length !== 64
  ) {
    throw new Error(`${platform} updater signature is not a complete Minisign signature.`);
  }
}

function decodeOuterBase64(value, label) {
  if (!/^[A-Za-z0-9+/=]+$/.test(value)) {
    throw new Error(`${label} is not valid Base64.`);
  }
  return Buffer.from(value, "base64").toString("utf8");
}

function decodeBase64Exact(value, expectedLength, label) {
  const decoded = Buffer.from(value ?? "", "base64");
  if (decoded.length !== expectedLength) {
    throw new Error(`${label} has an invalid encoded length.`);
  }
  return decoded;
}

async function main() {
  const [command, ...args] = process.argv.slice(2);
  const options = parseArgs(args);
  const version = required(options, "version");
  const assetDir = resolve(required(options, "asset-dir"));
  const baseUrl = `https://github.com/yuxino/tick/releases/download/v${version}/`;
  const manifestPath = resolve(options.output ?? join(assetDir, "latest.json"));
  const tauriConfig = JSON.parse(await readFile(resolve("src-tauri/tauri.conf.json"), "utf8"));
  const publicKey = tauriConfig.plugins?.updater?.pubkey;
  if (!publicKey) throw new Error("Updater public key is missing from tauri.conf.json.");

  if (command === "build") {
    const notes = await readFile(resolve(required(options, "notes-file")), "utf8");
    const manifest = await buildUpdaterManifest({
      version,
      notes,
      pubDate: options["pub-date"] ?? new Date().toISOString(),
      baseUrl,
      assetDir,
      publicKey,
    });
    await writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, "utf8");
  } else if (command !== "verify") {
    throw new Error("Usage: updater-release.mjs <build|verify> --version X.Y.Z --asset-dir DIR [--notes-file FILE] [--output FILE]");
  }

  const hashes = await verifyUpdaterManifest({ manifestPath, expectedVersion: version, assetDir, baseUrl, publicKey });
  console.log(`Updater manifest verified for v${version}: ${Object.keys(hashes).join(", ")}`);
}

function parseArgs(args) {
  const result = {};
  for (let index = 0; index < args.length; index += 2) {
    const key = args[index];
    const value = args[index + 1];
    if (!key?.startsWith("--") || value === undefined) {
      throw new Error(`Invalid updater release argument: ${key ?? "missing"}`);
    }
    result[key.slice(2)] = value;
  }
  return result;
}

function required(options, key) {
  const value = options[key];
  if (!value) throw new Error(`Missing --${key}.`);
  return value;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : error);
    process.exitCode = 1;
  });
}
