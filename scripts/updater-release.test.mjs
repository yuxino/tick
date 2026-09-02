import { createHash, generateKeyPairSync, sign } from "node:crypto";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { buildUpdaterManifest, verifyUpdaterManifest } from "./updater-release.mjs";

const tempDirs = [];
const version = "0.1.4";
const baseUrl = `https://github.com/yuxino/tick/releases/download/v${version}/`;
const keyId = Buffer.from("0102030405060708", "hex");
const algorithm = Buffer.from("ED");
const { publicKey: fixturePublicKey, privateKey } = generateKeyPairSync("ed25519");
const rawPublicKey = fixturePublicKey.export({ format: "der", type: "spki" }).subarray(-32);
const publicKey = Buffer.from([
  "untrusted comment: minisign public key: fixture",
  Buffer.concat([algorithm, keyId, rawPublicKey]).toString("base64"),
].join("\n")).toString("base64");

afterEach(async () => {
  await Promise.all(tempDirs.splice(0).map((path) => rm(path, { recursive: true, force: true })));
});

describe("updater release manifest", () => {
  it("builds and verifies one signed asset per supported architecture", async () => {
    const assetDir = await createFixture();
    const manifest = await buildUpdaterManifest({
      version,
      notes: "Bootstrap release",
      pubDate: "2026-09-02T00:00:00Z",
      baseUrl,
      assetDir,
      publicKey,
    });
    const manifestPath = join(assetDir, "latest.json");
    await writeFile(manifestPath, JSON.stringify(manifest));

    const hashes = await verifyUpdaterManifest({ manifestPath, expectedVersion: version, assetDir, baseUrl, publicKey });
    expect(Object.keys(hashes)).toEqual([
      "Tick.app.tar.gz",
      "Tick_0.1.4_x64-setup.exe",
      "Tick_0.1.4_arm64-setup.exe",
    ]);
  });

  it("rejects a malformed signature field", async () => {
    const assetDir = await createFixture();
    await writeFile(join(assetDir, "Tick.app.tar.gz.sig"), "not-a-signature");

    await expect(buildUpdaterManifest({
      version,
      notes: "Bootstrap release",
      pubDate: "2026-09-02T00:00:00Z",
      baseUrl,
      assetDir,
      publicKey,
    })).rejects.toThrow("signature is missing or malformed");
  });

  it("rejects a missing architecture and an unexpected release URL", async () => {
    const assetDir = await createFixture();
    await rm(join(assetDir, "Tick_0.1.4_arm64-setup.exe"));
    await expect(buildUpdaterManifest({
      version,
      notes: "Bootstrap release",
      pubDate: "2026-09-02T00:00:00Z",
      baseUrl,
      assetDir,
      publicKey,
    })).rejects.toThrow("windows-aarch64 updater asset");
    await expect(buildUpdaterManifest({
      version,
      notes: "Bootstrap release",
      pubDate: "2026-09-02T00:00:00Z",
      baseUrl: "https://example.com/releases/",
      assetDir,
      publicKey,
    })).rejects.toThrow("Unexpected updater release base URL");
  });

  it("rejects a structurally valid signature made for different bytes", async () => {
    const assetDir = await createFixture();
    await writeFile(join(assetDir, "Tick.app.tar.gz"), "tampered updater");
    await expect(buildUpdaterManifest({
      version,
      notes: "Bootstrap release",
      pubDate: "2026-09-02T00:00:00Z",
      baseUrl,
      assetDir,
      publicKey,
    })).rejects.toThrow("signature verification failed");
  });
});

async function createFixture() {
  const assetDir = await mkdtemp(join(tmpdir(), "tick-updater-fixture-"));
  tempDirs.push(assetDir);
  const names = [
    "Tick.app.tar.gz",
    "Tick_0.1.4_x64-setup.exe",
    "Tick_0.1.4_arm64-setup.exe",
  ];
  for (const name of names) {
    await writeFile(join(assetDir, name), `fixture:${name}`);
    await writeFile(join(assetDir, `${name}.sig`), await signFixture(join(assetDir, name), name));
  }
  return assetDir;
}

async function signFixture(path, filename) {
  const artifact = await readFile(path);
  const dataSignature = sign(null, createHash("blake2b512").update(artifact).digest(), privateKey);
  const trustedComment = `timestamp:1788307200\tfile:${filename}`;
  const globalSignature = sign(null, Buffer.concat([dataSignature, Buffer.from(trustedComment)]), privateKey);
  return Buffer.from([
    "untrusted comment: signature from minisign secret key",
    Buffer.concat([algorithm, keyId, dataSignature]).toString("base64"),
    `trusted comment: ${trustedComment}`,
    globalSignature.toString("base64"),
  ].join("\n")).toString("base64");
}
