// A browser host loads the module the Node entry loads, so it must supply every import on the allowlist.
import { readFileSync } from "node:fs";

// wasm-bindgen suffixes each name with a hash that moves with any dependency's version.
const HASH = /_[0-9a-f]{16}$/;

const [allowlist, wasm] = process.argv.slice(2);
if (allowlist === undefined || wasm === undefined) {
  process.stderr.write("usage: node package/check-imports.mjs <allowlist> <module.wasm>\n");
  process.exit(2);
}
const allowed = new Set(
  readFileSync(allowlist, "utf8").split("\n").map((line) => line.trim()).filter((line) => line !== ""),
);
const module = new WebAssembly.Module(readFileSync(wasm));
const unlisted = [
  ...new Set(WebAssembly.Module.imports(module).map(({ name }) => name.replace(HASH, ""))),
].filter((name) => !allowed.has(name));
for (const name of unlisted) {
  process.stdout.write(`not on the allowlist: ${name}\n`);
}
process.exitCode = unlisted.length === 0 ? 0 : 1;
