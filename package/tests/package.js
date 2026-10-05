import assert from "node:assert/strict";
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";

export const repository = fileURLToPath(new URL("../../", import.meta.url));
export const dist = join(repository, "package", "dist");

export async function bridge() {
  const entry = join(dist, "node.js");
  assert.ok(existsSync(entry), `no package entry at ${entry}: run \`sh package/build.sh\` first`);
  return import(new URL("../dist/node.js", import.meta.url));
}

function filesUnder(root, directory, files) {
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      filesUnder(root, path, files);
    } else {
      files.set(relative(root, path).split(sep).join("/"), new Uint8Array(readFileSync(path)));
    }
  }
  return files;
}

export function files(directory) {
  const root = join(repository, "crates", "bridge", "tests", directory);
  return filesUnder(root, root, new Map());
}
