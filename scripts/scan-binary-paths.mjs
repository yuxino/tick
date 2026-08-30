import { readFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";

const homePathPatterns = [
  { label: "macOS home", regex: /\/Users\/[^/\0]{1,128}\//giu },
  { label: "Linux home", regex: /\/home\/[^/\0]{1,128}\//giu },
  {
    label: "Windows home",
    regex: /(?:\\\\\?\\)?[A-Z]:[\\/]Users[\\/](?!(?:you|user|<user>)[\\/])[^\\/\0]{1,128}[\\/]/giu,
  },
];

function findMatches(text, encoding) {
  const matches = [];
  for (const { label, regex } of homePathPatterns) {
    regex.lastIndex = 0;
    for (const match of text.matchAll(regex)) {
      matches.push({ encoding, label, offset: match.index });
    }
  }
  return matches;
}

export async function scanBinary(filePath) {
  const bytes = await readFile(filePath);
  return [
    ...findMatches(bytes.toString("latin1"), "single-byte"),
    ...findMatches(bytes.toString("utf16le"), "UTF-16LE/even"),
    ...findMatches(bytes.subarray(1).toString("utf16le"), "UTF-16LE/odd"),
  ];
}

async function main(filePaths) {
  if (filePaths.length === 0) {
    throw new Error("Usage: node scripts/scan-binary-paths.mjs <binary> [...]");
  }

  let failed = false;
  for (const filePath of filePaths) {
    const matches = await scanBinary(filePath);
    if (matches.length === 0) {
      console.log(`No embedded user-home paths: ${filePath}`);
      continue;
    }

    failed = true;
    const summaries = new Map();
    for (const match of matches) {
      const key = `${match.label}/${match.encoding}`;
      const summary = summaries.get(key) || { ...match, count: 0 };
      summary.count += 1;
      summaries.set(key, summary);
    }
    for (const match of summaries.values()) {
      console.error(
        `Embedded ${match.label} path detected ${match.count} time(s) in ${filePath} (${match.encoding}, first offset ${match.offset}).`,
      );
    }
  }

  if (failed) {
    process.exitCode = 1;
  }
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  await main(process.argv.slice(2));
}
