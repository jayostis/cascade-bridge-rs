// The module the package ships has no name section, and the module a profile is read from has one and every other section the same:
// `node package/wasm-names.mjs <shipped.wasm> <named.wasm>` says where it does not.
// `node package/wasm-names.mjs drop <custom section> <in.wasm> <out.wasm>` writes a module without one custom section.
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const FUNCTION_NAMES = 1;
const CODE = 10;

export function inspect(bytes) {
  const buffer = Buffer.from(bytes);
  let at = 8;
  const leb = () => {
    let value = 0;
    let shift = 0;
    let byte;
    do {
      byte = buffer[at++];
      value |= (byte & 127) << shift;
      shift += 7;
    } while (byte & 128);
    return value >>> 0;
  };
  const sections = [];
  let code = Buffer.alloc(0);
  const bodies = new Map();
  let functionNames = 0;
  while (at < buffer.length) {
    const id = buffer[at++];
    const end = leb() + at;
    if (id === CODE) code = buffer.subarray(at, end);
    if (id !== 0) {
      sections.push(String(id));
      bodies.set(id, buffer.subarray(at, end));
    } else {
      const name = buffer.subarray(at + 1, at + 1 + buffer[at]).toString();
      sections.push(name);
      if (name === "name") {
        at += 1 + buffer[at];
        while (at < end) {
          const kind = buffer[at++];
          const next = leb() + at;
          if (kind === FUNCTION_NAMES) functionNames = leb();
          at = next;
        }
      }
    }
    at = end;
  }
  return { sections, code, bodies, functionNames };
}

export function withoutSection(bytes, dropped) {
  const buffer = Buffer.from(bytes);
  const kept = [buffer.subarray(0, 8)];
  let at = 8;
  while (at < buffer.length) {
    const start = at;
    const id = buffer[at++];
    let size = 0;
    let shift = 0;
    let byte;
    do {
      byte = buffer[at++];
      size |= (byte & 127) << shift;
      shift += 7;
    } while (byte & 128);
    const end = at + size;
    const named = id === 0 && buffer.subarray(at + 1, at + 1 + buffer[at]).toString() === dropped;
    if (!named) kept.push(buffer.subarray(start, end));
    at = end;
  }
  return Buffer.concat(kept);
}

export function faults(shipped, named) {
  const kept = inspect(shipped);
  const made = inspect(named);
  const found = [];
  if (kept.functionNames > 0)
    found.push(`the shipped module keeps its names (${kept.functionNames} functions): the package would weigh more for every reader`);
  if (made.functionNames === 0)
    found.push("the named module holds no function names: a profile of it would show wasm-function[N]");
  for (const debug of made.sections.filter((section) => section.startsWith(".debug")))
    found.push(`the named module holds ${debug}: it would weigh as much again and name nothing more`);
  for (const id of new Set([...kept.bodies.keys(), ...made.bodies.keys()])) {
    const a = kept.bodies.get(id);
    const b = made.bodies.get(id);
    if (a === undefined || b === undefined || !a.equals(b))
      found.push(`${id === CODE ? "the code" : `section ${id}`} differs between the shipped module (${a?.length ?? "absent"} bytes) and the named one (${b?.length ?? "absent"}): a profile of it would be of other code`);
  }
  return found;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const [shipped, named, ...rest] = process.argv.slice(2);
  if (shipped === "drop" && rest.length === 2) {
    const [input, output] = rest;
    writeFileSync(output, withoutSection(readFileSync(input), named));
    process.exit(0);
  }
  if (shipped === undefined || named === undefined || shipped === "drop") {
    process.stderr.write("usage: node package/wasm-names.mjs <shipped.wasm> <named.wasm> | drop <custom section> <in.wasm> <out.wasm>\n");
    process.exit(2);
  }
  const found = faults(readFileSync(shipped), readFileSync(named));
  for (const fault of found) process.stdout.write(`${fault}\n`);
  process.exitCode = found.length === 0 ? 0 : 1;
}
