import assert from "node:assert/strict";
import { cpSync, existsSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, relative, sep } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

export const repository = fileURLToPath(new URL("../../", import.meta.url));
export const dist = join(repository, "package", "dist");

export const ADAPTER = "https://example.org/adapters/catalog/";
export const VOCABULARY = "https://example.org/vocabularies/";
export const DOCUMENT = "https://example.org/documents/catalog.xml";

const copies = [];

export async function instance(rebuilt = (wasm) => wasm) {
  const entry = join(dist, "node.js");
  assert.ok(existsSync(entry), `no package entry at ${entry}: run \`sh package/build.sh\` first`);
  const directory = mkdtempSync(join(tmpdir(), "cascade-bridge-"));
  copies.push(directory);
  cpSync(dist, directory, { recursive: true });
  const wasm = join(directory, "cascade_bridge_bg.wasm");
  writeFileSync(wasm, rebuilt(readFileSync(wasm)));
  return import(pathToFileURL(join(directory, "node.js")).href);
}

export function discardInstances() {
  for (const directory of copies) rmSync(directory, { recursive: true, force: true });
}

export function toLoad() {
  const adapter = files("tiny-adapter");
  for (const path of [...adapter.keys()]) {
    if (path.startsWith("fixtures/")) adapter.delete(path);
  }
  return adapter;
}

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
