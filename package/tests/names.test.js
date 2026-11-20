import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import { faults, inspect, withoutSection } from "../wasm-names.mjs";

const script = fileURLToPath(new URL("../wasm-names.mjs", import.meta.url));

const text = (string) => [...Buffer.from(string)];
const section = (id, bytes) => [id, bytes.length, ...bytes];
const custom = (name, bytes) => section(0, [name.length, ...text(name), ...bytes]);

// One function, taking and returning nothing, whose body is `body` after its empty list of locals.
function module({ names = [], body = [0x0b], extra = [] } = {}) {
  const code = section(10, [1, body.length + 1, 0, ...body]);
  const naming = names.length === 0
    ? []
    : custom("name", [1, 1 + 1 + names[0].length + 1, 1, 0, names[0].length, ...text(names[0])]);
  return Uint8Array.from([
    0x00, 0x61, 0x73, 0x6d, 1, 0, 0, 0,
    ...section(1, [1, 0x60, 0, 0]),
    ...section(3, [1, 0]),
    ...code,
    ...naming,
    ...extra,
  ]);
}

test("a module's function names are counted from its name section, and its code is its code section", () => {
  const named = inspect(module({ names: ["_ZN6bridge4lift17h0123456789abcdefE"] }));
  assert.equal(named.functionNames, 1);
  assert.deepEqual([...named.code], [1, 2, 0, 0x0b]);
  assert.deepEqual(named.sections, ["1", "3", "10", "name"]);
});

test("a module with no name section has no function names", () => {
  assert.equal(inspect(module()).functionNames, 0);
});

test("a shipped module without names beside a named one with the same code has no fault", () => {
  assert.deepEqual(faults(module(), module({ names: ["lift"] })), []);
});

test("a shipped module that keeps its names is a fault, as the package would weigh more for every reader", () => {
  const found = faults(module({ names: ["lift"] }), module({ names: ["lift"] }));
  assert.equal(found.length, 1);
  assert.match(found[0], /shipped module keeps its names/);
});

test("a named module with no names is a fault, as a profile of it would show none", () => {
  const found = faults(module(), module());
  assert.equal(found.length, 1);
  assert.match(found[0], /named module holds no function names/);
});

test("a named module that holds the debug sections of its source is a fault, as it would weigh as much again", () => {
  const found = faults(module(), module({ names: ["lift"], extra: custom(".debug_info", [0]) }));
  assert.equal(found.length, 1);
  assert.match(found[0], /\.debug_info/);
});

test("a named module whose code differs from the shipped module's is a fault, as a profile of it would be of other code", () => {
  const found = faults(module(), module({ names: ["lift"], body: [0x01, 0x0b] }));
  assert.equal(found.length, 1);
  assert.match(found[0], /code differs/);
});

test("a named module whose other non-custom sections differ from the shipped module's is a fault, though its code is the same", () => {
  const found = faults(module(), module({ names: ["lift"], extra: section(6, [0]) }));
  assert.equal(found.length, 1);
  assert.match(found[0], /section 6 differs/);
});

test("the command exits 0 on a pair with no fault and 1, saying what, on one with", () => {
  const folder = mkdtempSync(join(tmpdir(), "names-"));
  const write = (name, bytes) => {
    writeFileSync(join(folder, name), bytes);
    return join(folder, name);
  };
  const shipped = write("shipped.wasm", module());
  const named = write("named.wasm", module({ names: ["lift"] }));
  execFileSync(process.execPath, [script, shipped, named]);
  const failed = spawnSync(process.execPath, [script, named, named], { encoding: "utf8" });
  assert.equal(failed.status, 1);
  assert.match(failed.stdout, /shipped module keeps its names/);
});

test("dropping a custom section leaves the other sections byte for byte, and a module without it as it was", () => {
  const features = custom("target_features", [1, 0x2b, 4, ...text("simd")]);
  const named = module({ names: ["lift"], extra: features });
  const dropped = withoutSection(named, "target_features");
  assert.deepEqual(inspect(dropped).sections, ["1", "3", "10", "name"]);
  assert.deepEqual(Buffer.from(dropped), Buffer.from(module({ names: ["lift"] })));
  assert.deepEqual(Buffer.from(withoutSection(module(), "target_features")), Buffer.from(module()));
});

test("the command's drop writes the module without the section it names", () => {
  const folder = mkdtempSync(join(tmpdir(), "names-"));
  const input = join(folder, "in.wasm");
  const output = join(folder, "out.wasm");
  writeFileSync(input, module({ names: ["lift"], extra: custom("target_features", [0]) }));
  execFileSync(process.execPath, [script, "drop", "target_features", input, output]);
  assert.deepEqual(inspect(readFileSync(output)).sections, ["1", "3", "10", "name"]);
});
