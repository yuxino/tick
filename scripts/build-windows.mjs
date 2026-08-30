import { existsSync } from "node:fs";
import { homedir } from "node:os";
import { join, parse, resolve } from "node:path";
import { spawnSync } from "node:child_process";

const encodedSeparator = "\u001f";
const workspace = resolve(import.meta.dirname, "..");
const cargoHome = resolve(
  process.env.CARGO_HOME || join(homedir(), ".cargo"),
);

if (process.env.RUSTFLAGS && !process.env.CARGO_ENCODED_RUSTFLAGS) {
  throw new Error(
    "RUSTFLAGS is set. Convert it to CARGO_ENCODED_RUSTFLAGS before using the privacy-preserving Windows build.",
  );
}

const remaps = new Map([
  [workspace, "tick-src"],
  [cargoHome, "cargo-home"],
  [homedir(), "build-home"],
]);
const remapFlags = [...remaps]
  .filter(([source]) => source && source !== parse(source).root)
  .sort(([left], [right]) => right.length - left.length)
  .map(([source, target]) => `--remap-path-prefix=${source}=${target}`);

const existingFlags = process.env.CARGO_ENCODED_RUSTFLAGS
  ? process.env.CARGO_ENCODED_RUSTFLAGS.split(encodedSeparator).filter(Boolean)
  : [];
const env = {
  ...process.env,
  CARGO_ENCODED_RUSTFLAGS: [...existingFlags, ...remapFlags].join(
    encodedSeparator,
  ),
};

const tauri = join(workspace, "node_modules", "@tauri-apps", "cli", "tauri.js");
if (!existsSync(tauri)) {
  throw new Error(`Tauri CLI not found at ${tauri}; run npm ci first.`);
}

const result = spawnSync(
  process.execPath,
  [tauri, "build", ...process.argv.slice(2)],
  {
    cwd: workspace,
    env,
    shell: false,
    stdio: "inherit",
    windowsHide: true,
  },
);

if (result.error) {
  throw result.error;
}
process.exit(result.status ?? 1);
