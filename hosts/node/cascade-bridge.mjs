// The cascade-bridge command on Node, over the library built for wasm32-unknown-unknown.
// `sh hosts/node/setup.sh` builds the module it loads.
import { closeSync, openSync, readFileSync, realpathSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { basename, dirname, isAbsolute, join, relative, resolve, sep } from "node:path";

if (globalThis.crypto?.getRandomValues === undefined) {
  process.stderr.write(
    `cascade-bridge: Node ${process.versions.node} has no globalThis.crypto; Node 19 or later is needed\n`,
  );
  process.exit(2);
}

const bridge = createRequire(import.meta.url)("./pkg/cascade_bridge_wasm.js");

function reason(e, path) {
  return e.code ?? String(e.message ?? e).replaceAll(path, "");
}

function canonical(path) {
  try {
    return realpathSync.native(path);
  } catch (e) {
    throw new Error(`${path}: ${reason(e, path)}`);
  }
}

// A missing file resolves through its nearest existing ancestor, so it is still judged
// against the boundary.
function reached(path) {
  const tail = [];
  for (let head = resolve(path); ; head = dirname(head)) {
    try {
      return join(realpathSync.native(head), ...tail);
    } catch {
      if (dirname(head) === head) return undefined;
      tail.unshift(basename(head));
    }
  }
}

// The boundary is decided on the path a filesystem reaches, never on the IRI's spelling.
class Directory {
  constructor(path) {
    this.path = canonical(path);
    this.iri = `${bridge.path_to_file_iri(this.path)}/`;
  }

  read(iri, what) {
    const bare = iri.split("#")[0];
    const path = bare.startsWith("file://") && bridge.authority(bare) === bridge.authority(this.iri)
      ? bridge.file_iri_to_path(bare)
      : undefined;
    const at = path === undefined ? undefined : reached(path);
    const within = at === undefined ? undefined : relative(this.path, at);
    const outside = within === ".." || within?.startsWith(`..${sep}`);
    if (within === undefined || outside || isAbsolute(within)) {
      throw `not inside ${what}`;
    }
    try {
      return readFileSync(at);
    } catch (e) {
      throw reason(e, at);
    }
  }
}

// What throws here reaches the module as the reason it prints.
function said(f) {
  return (...a) => {
    try {
      return f(...a);
    } catch (e) {
      throw e instanceof Error ? e.message : e;
    }
  };
}

function at(path, f) {
  try {
    return f();
  } catch (e) {
    throw `${path}: ${e.message}`;
  }
}

let adapter;
let vocabularies;

const host = {
  adapter: said((path) => {
    adapter = new Directory(path);
    return adapter.iri;
  }),
  vocabularies: said((path) => {
    vocabularies = new Directory(path);
    return vocabularies.iri;
  }),
  read: (iri) => adapter.read(iri, "the adapter"),
  readVocabulary: (iri) => vocabularies.read(iri, "the vocabularies"),
  readFile: (path) => at(path, () => readFileSync(path)),
  fileIri: said((path) => bridge.path_to_file_iri(canonical(path))),
  create: (path) => at(path, () => closeSync(openSync(path, "a"))),
  write: (path, text) => at(path, () => writeFileSync(path, text)),
  out: (text) => {
    process.stdout.write(text);
  },
  err: (text) => {
    process.stderr.write(text);
  },
};

// A pipe's reader may go away mid-graph; the caller is owed a status, not an exception.
process.stdout.on("error", (e) => {
  process.stderr.write(`cascade-bridge: standard output: ${e.message}\n`);
  process.exitCode = 2;
});

try {
  process.exitCode = bridge.run(process.argv.slice(2), host);
} catch (e) {
  process.stderr.write(`cascade-bridge: ${e instanceof Error ? e.message : e}\n`);
  process.exitCode = 2;
}
