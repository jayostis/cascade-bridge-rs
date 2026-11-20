// The module the package ships has no name section, and the module a profile is read from has one and the same code:
// `node package/wasm-names.mjs <shipped.wasm> <named.wasm>` says where it does not.
import { readFileSync } from "node:fs";
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
  let functionNames = 0;
  while (at < buffer.length) {
    const id = buffer[at++];
    const end = leb() + at;
    if (id === CODE) code = buffer.subarray(at, end);
    if (id !== 0) {
      sections.push(String(id));
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
  return { sections, code, functionNames };
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
  if (!kept.code.equals(made.code))
    found.push("the code differs between the shipped module and the named one: a profile of it would be of other code");
  return found;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const [shipped, named] = process.argv.slice(2);
  if (shipped === undefined || named === undefined) {
    process.stderr.write("usage: node package/wasm-names.mjs <shipped.wasm> <named.wasm>\n");
    process.exit(2);
  }
  const found = faults(readFileSync(shipped), readFileSync(named));
  for (const fault of found) process.stdout.write(`${fault}\n`);
  process.exitCode = found.length === 0 ? 0 : 1;
}
