import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";
import { dist, repository } from "./package.js";

function manifest() {
  const path = join(dist, "package.json");
  assert.ok(existsSync(path), `no package at ${path}: run \`sh package/build.sh\` first`);
  return JSON.parse(readFileSync(path, "utf8"));
}

function run(command, args, cwd) {
  return execFileSync(command, args, { cwd, encoding: "utf8", shell: process.platform === "win32" });
}

test("the package's version is 0.1.0-commit- and the commit it was built from", () => {
  const head = run("git", ["rev-parse", "HEAD"], repository).trim();
  assert.equal(manifest().version, `0.1.0-commit-${head}`);
});

test("the package's sources are what package/sources.mjs prints", () => {
  const sources = manifest().cascadeBridge?.sources;
  const printed = run(process.execPath, [join("package", "sources.mjs")], repository).trim();
  assert.equal(sources, printed);
});

test("the package needs Node 22 or later", () => {
  assert.equal(manifest().engines?.node, ">=22");
});

test("the package's exports have the types, node and default conditions", () => {
  const { exports } = manifest();
  const conditions = Object.keys(exports?.["."] ?? exports ?? {});
  for (const condition of ["types", "node", "default"]) {
    assert.ok(conditions.includes(condition), `${condition} is not among ${conditions}`);
  }
});

test("the package packs exactly its manifest, licence, Node entry, module, declarations and wasm", () => {
  manifest();
  const [packed] = JSON.parse(run("npm", ["pack", "--dry-run", "--json"], dist));
  assert.deepEqual(
    packed.files.map(({ path }) => path).sort(),
    [
      "LICENSE",
      "cascade_bridge.d.ts",
      "cascade_bridge.js",
      "cascade_bridge_bg.wasm",
      "node.js",
      "package.json",
    ],
  );
});
