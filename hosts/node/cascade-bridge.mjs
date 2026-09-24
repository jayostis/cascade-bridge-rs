// The cascade-bridge command on Node, over the library built for wasm32-unknown-unknown.
// `sh hosts/node/setup.sh` builds the module it loads.
import { closeSync, openSync, readFileSync, realpathSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { basename, dirname, isAbsolute, join, relative, resolve, sep } from "node:path";

const USAGE = `usage: cascade-bridge test <adapter-dir> [--vocabularies <directory>] [--earl <out.ttl>] [--datasets]
       cascade-bridge convert <adapter-dir> <document.xml> [--vocabularies <directory>] [--out <file>] [--findings <file>] [--format turtle|ntriples]`;

if (globalThis.crypto?.getRandomValues === undefined) {
  process.stderr.write(
    `cascade-bridge: Node ${process.versions.node} has no globalThis.crypto; Node 19 or later is needed\n`,
  );
  process.exit(2);
}

const bridge = createRequire(import.meta.url)("./pkg/cascade_bridge_wasm.js");

// The bytes the native command's resolver leaves unencoded, so both hosts name
// a file by the same IRI.
const UNENCODED = /[A-Za-z0-9\-._~/:]/;

function encoded(text) {
  let out = "";
  for (const byte of new TextEncoder().encode(text)) {
    const c = String.fromCharCode(byte);
    out += UNENCODED.test(c) ? c : `%${byte.toString(16).toUpperCase().padStart(2, "0")}`;
  }
  return out;
}

function pathToFileIri(path) {
  const text = path.replaceAll("\\", "/");
  if (text.startsWith("//")) {
    const [server, ...rest] = text.slice(2).split("/");
    return `file://${encoded(server)}/${encoded(rest.join("/"))}`;
  }
  return `file://${text.startsWith("/") ? "" : "/"}${encoded(text)}`;
}

// A message would name again the file its IRI already names.
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

// The server a file IRI names, empty for this machine.
function authority(iri) {
  const rest = iri.slice("file://".length);
  const server = rest.split("/")[0];
  return server === "localhost" ? "" : server;
}

function fileIriToPath(iri) {
  if (!iri.startsWith("file://")) return undefined;
  const rest = iri.slice("file://".length);
  const cut = rest.indexOf("/");
  if (cut === -1) return undefined;
  try {
    const server = authority(iri);
    const decoded = decodeURIComponent(rest.slice(cut));
    if (server !== "") return `//${decodeURIComponent(server)}${decoded}`;
    return /^\/[A-Za-z]:/.test(decoded) ? decoded.slice(1) : decoded;
  } catch {
    return undefined;
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
    this.iri = `${pathToFileIri(this.path)}/`;
  }

  // What the module is handed, or the reason it reports after the IRI.
  read(iri, what) {
    const bare = iri.split("#")[0];
    const path = bare.startsWith("file://") && authority(bare) === authority(this.iri)
      ? fileIriToPath(bare)
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

function host(a) {
  const adapter = new Directory(a.directory);
  const vocabularies = a.vocabularies === undefined ? undefined : new Directory(a.vocabularies);
  return {
    root: adapter.iri,
    vocabularies: vocabularies?.iri,
    files: {
      read: (iri) => adapter.read(iri, "the adapter"),
      readVocabulary: (iri) => vocabularies.read(iri, "the vocabularies"),
    },
  };
}

function parse(argv) {
  const next = () => argv.shift();
  const a = { command: next(), directory: next() };
  if (a.directory === undefined) return undefined;
  switch (a.command) {
    case "test":
      a.datasets = false;
      for (let flag; (flag = next()) !== undefined; ) {
        if (flag === "--datasets") a.datasets = true;
        else if (flag === "--vocabularies" || flag === "--earl") a[flag.slice(2)] = next();
        else return undefined;
      }
      return a;
    case "convert":
      a.document = next();
      a.format = "turtle";
      if (a.document === undefined) return undefined;
      for (let flag; (flag = next()) !== undefined; ) {
        if (!["--vocabularies", "--out", "--findings", "--format"].includes(flag)) return undefined;
        a[flag.slice(2)] = next();
      }
      return a;
    default:
      return undefined;
  }
}

// A flag given no value leaves its key present and undefined.
function complete(a) {
  return Object.values(a).every((value) => value !== undefined)
    && (a.format === undefined || ["turtle", "ntriples"].includes(a.format));
}

function write(path, text) {
  try {
    writeFileSync(path, text);
  } catch (e) {
    throw new Error(`${path}: ${e.message}`);
  }
}

function test(a) {
  const { root, vocabularies, files } = host(a);
  const run = bridge.test(root, vocabularies, files, a.datasets);
  process.stdout.write(`${run.summary}\n`);
  if (a.earl !== undefined) {
    write(a.earl, run.earl);
    process.stdout.write(`EARL     ${a.earl}\n`);
  }
  return run.holds ? 0 : 1;
}

function convert(a) {
  const { root, vocabularies, files } = host(a);
  let document;
  try {
    document = readFileSync(a.document);
  } catch (e) {
    throw new Error(`${a.document}: ${e.message}`);
  }
  const iri = pathToFileIri(canonical(a.document));
  const converted = bridge.convert(
    root,
    vocabularies,
    files,
    iri,
    document,
    a.format,
    a.findings !== undefined,
  );
  const graph = converted.graph();
  process.stderr.write(`${converted.summary}\n`);
  // Before the graph, so a findings file that cannot be written leaves standard output
  // empty. Created first, since its own IRI names the document; not emptied, so a run
  // that fails leaves a committed oracle as it was.
  if (a.findings !== undefined) {
    try {
      closeSync(openSync(a.findings, "a"));
    } catch (e) {
      throw new Error(`${a.findings}: ${e.message}`);
    }
    write(a.findings, converted.findings(pathToFileIri(canonical(a.findings))));
    process.stderr.write(`Findings ${a.findings}\n`);
  }
  if (a.out !== undefined) {
    write(a.out, graph);
    process.stderr.write(`Graph    ${a.out}\n`);
  } else {
    process.stdout.write(graph);
  }
  return 0;
}

function main(argv) {
  const a = parse(argv);
  if (a === undefined || !complete(a)) {
    process.stderr.write(`${USAGE}\n`);
    return 2;
  }
  try {
    return a.command === "test" ? test(a) : convert(a);
  } catch (e) {
    process.stderr.write(`cascade-bridge: ${e instanceof Error ? e.message : e}\n`);
    return 2;
  }
}

// A pipe's reader may go away mid-graph; the caller is owed a status, not an exception.
process.stdout.on("error", (e) => {
  process.stderr.write(`cascade-bridge: standard output: ${e.message}\n`);
  process.exitCode = 2;
});

process.exitCode = main(process.argv.slice(2));
