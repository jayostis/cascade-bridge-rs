// node hosts/node/check-imports.mjs <allowlist>
//
// Fails naming each import of the module the node host loads that the
// allowlist, one import per line, leaves out. std::fs compiles on
// wasm32-unknown-unknown and fails only when called, and a dependency reaching
// the host for a clock, a timezone or a file arrives as a new import, so this
// is where one is first seen. A browser host loads the same module and must
// supply the same list.
import { readFileSync } from "node:fs";

// wasm-bindgen suffixes each name with a hash of its crate and signature,
// which moves with any dependency's version and says nothing about what the
// host must supply.
const HASH = /_[0-9a-f]{16}$/;

const [allowlist] = process.argv.slice(2);
if (allowlist === undefined) {
  process.stderr.write("usage: node hosts/node/check-imports.mjs <allowlist>\n");
  process.exit(2);
}
const allowed = new Set(
  readFileSync(allowlist, "utf8").split("\n").map((line) => line.trim()).filter((line) => line !== ""),
);
const module = new WebAssembly.Module(
  readFileSync(new URL("./pkg/cascade_bridge_wasm_bg.wasm", import.meta.url)),
);
const unlisted = [
  ...new Set(WebAssembly.Module.imports(module).map(({ name }) => name.replace(HASH, ""))),
].filter((name) => !allowed.has(name));
for (const name of unlisted) {
  process.stdout.write(`not on the allowlist: ${name}\n`);
}
process.exitCode = unlisted.length === 0 ? 0 : 1;
