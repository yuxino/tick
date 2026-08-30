import assert from "node:assert/strict";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { scanBinary } from "./scan-binary-paths.mjs";

async function withFixture(contents, callback) {
  const directory = await mkdtemp(join(tmpdir(), "tick-path-scan-"));
  const fixture = join(directory, "fixture.bin");
  try {
    await writeFile(fixture, contents);
    await callback(fixture);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

test("detects ASCII macOS and Linux user homes", async () => {
  await withFixture(
    Buffer.from("x/Users/alice/.cargo/source y/home/runner/work/tick"),
    async (fixture) => {
      const matches = await scanBinary(fixture);
      assert.deepEqual(
        matches.map(({ label }) => label),
        ["macOS home", "Linux home"],
      );
    },
  );
});

test("detects Windows user homes in UTF-16LE", async () => {
  await withFixture(
    Buffer.from("C:\\Users\\runneradmin\\.cargo\\registry", "utf16le"),
    async (fixture) => {
      const matches = await scanBinary(fixture);
      assert.ok(matches.some(({ label }) => label === "Windows home"));
    },
  );
});

test("accepts remapped diagnostics and runtime placeholders", async () => {
  await withFixture(
    Buffer.from(
      "cargo-home/registry/src/tauri/src/app.rs %APPDATA%\\tick C:\\Users\\you\\scripts /rustc/hash/library",
    ),
    async (fixture) => {
      assert.deepEqual(await scanBinary(fixture), []);
    },
  );
});
