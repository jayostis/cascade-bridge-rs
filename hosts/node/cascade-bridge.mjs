// The cascade-bridge command on Node, over the package's Node entry, which
// `sh package/build.sh` builds.
import {
  closeSync,
  lstatSync,
  openSync,
  readdirSync,
  readFileSync,
  realpathSync,
  writeFileSync,
} from "node:fs";
import { basename, dirname, join } from "node:path";
import { pathToFileURL } from "node:url";
import { Adapter, describe, test } from "../../package/dist/node.js";

const USAGE = `usage: cascade-bridge test <adapter-dir> [--vocabularies <directory>] [--earl <out.ttl>] [--datasets]
       cascade-bridge convert <adapter-dir> <document> [--envelope <iri>] [--facts <file>] [--vocabularies <directory>] [--out <file>] [--findings <file>] [--format turtle|ntriples]`;

const METADATA = "ro-crate-metadata.json";
const FAILING = ["failed", "inapplicable"];

class Refusal extends Error {}

function reason(error) {
  return error.code ?? error.message;
}

function at(path, f) {
  try {
    return f();
  } catch (error) {
    throw new Refusal(`${path}: ${reason(error)}`);
  }
}

function iriOf(path) {
  return pathToFileURL(at(path, () => realpathSync.native(path))).href;
}

// The IRI the file at `path` will have once it is written, read as it would be then.
function iriOfWritten(path) {
  try {
    return pathToFileURL(realpathSync.native(path)).href;
  } catch {
    return pathToFileURL(join(at(path, () => realpathSync.native(dirname(path))), basename(path))).href;
  }
}

// A directory read into a map, its symbolic links skipped, so the map holds only files inside it.
class Folder {
  constructor(directory) {
    this.path = at(directory, () => realpathSync.native(directory));
    this.iri = `${pathToFileURL(this.path).href}/`;
    this.files = new Map();
  }

  named() {
    return { iri: this.iri, files: this.files };
  }

  read(key) {
    const unread = (said) => new Refusal(`${this.iri}${key}: ${said}`);
    let path = this.path;
    for (const segment of key.split("/")) {
      let decoded;
      try {
        decoded = decodeURIComponent(segment);
      } catch {
        throw unread("not a path");
      }
      path = join(path, decoded);
      let linked;
      try {
        linked = lstatSync(path).isSymbolicLink();
      } catch (error) {
        throw unread(reason(error));
      }
      if (linked) throw unread("a symbolic link, which is not read");
    }
    try {
      return new Uint8Array(readFileSync(path));
    } catch (error) {
      throw unread(reason(error));
    }
  }

  with(keys) {
    for (const key of keys) {
      try {
        this.files.set(key, this.read(key));
      } catch {
        // A file the folder lacks is read again if a load asks for it, and named then.
      }
    }
    return this;
  }

  withEveryFile() {
    const pending = [this.path];
    while (pending.length > 0) {
      const directory = pending.pop();
      for (const entry of at(directory, () => readdirSync(directory, { withFileTypes: true }))) {
        const path = join(directory, entry.name);
        if (entry.isDirectory() && !(directory === this.path && entry.name === ".git")) {
          pending.push(path);
        } else if (entry.isFile()) {
          const key = pathToFileURL(path).href.slice(this.iri.length);
          this.files.set(key, new Uint8Array(at(path, () => readFileSync(path))));
        }
      }
    }
    return this;
  }
}

// What `call` answers once the maps hold every file it asks for, each file a map lacks
// read from its folder; a file the folder lacks too ends the call.
function complete(inputs, call) {
  for (;;) {
    try {
      return call(inputs.adapter.named(), inputs.vocabulary?.named());
    } catch (error) {
      if (error?.name !== "BridgeError" || error.kind !== "missing") throw error;
      const folder = error.map === "adapter" ? inputs.adapter : inputs.vocabulary;
      if (folder === undefined || folder.files.has(error.path)) throw error;
      folder.files.set(error.path, folder.read(error.path));
    }
  }
}

function opened(directory, vocabularies) {
  const adapter = new Folder(directory);
  const vocabulary = vocabularies === undefined ? undefined : new Folder(vocabularies);
  const description = describe(adapter.iri, adapter.read(METADATA));
  vocabulary?.with(description.vocabulary?.files ?? []);
  return [{ adapter, vocabulary }, description];
}

function parse(argv) {
  const [command, ...rest] = argv;
  const next = () => {
    if (rest.length === 0) throw new Refusal(USAGE);
    return rest.shift();
  };
  if (command === "test") {
    const parsed = { command, directory: next(), datasets: false };
    while (rest.length > 0) {
      const flag = rest.shift();
      if (flag === "--vocabularies") parsed.vocabularies = next();
      else if (flag === "--earl") parsed.earl = next();
      else if (flag === "--datasets") parsed.datasets = true;
      else return undefined;
    }
    return parsed;
  }
  if (command === "convert") {
    const parsed = { command, directory: next(), document: next(), format: "turtle" };
    while (rest.length > 0) {
      const flag = rest.shift();
      if (flag === "--envelope") parsed.envelope = next();
      else if (flag === "--facts") parsed.facts = next();
      else if (flag === "--vocabularies") parsed.vocabularies = next();
      else if (flag === "--out") parsed.out = next();
      else if (flag === "--findings") parsed.findings = next();
      else if (flag === "--format") {
        parsed.format = next();
        if (!["turtle", "ntriples"].includes(parsed.format)) return undefined;
      } else return undefined;
    }
    return parsed;
  }
  return undefined;
}

function adapterLine(identifier, iri) {
  return `Adapter  ${identifier ?? iri}  (${iri})`;
}

function tally(outcomes) {
  const counts = new Map();
  for (const outcome of outcomes) counts.set(outcome, (counts.get(outcome) ?? 0) + 1);
  return [...counts].map(([outcome, n]) => `${n} ${outcome}`).join(", ");
}

function runTest(parsed) {
  const [inputs, description] = opened(parsed.directory, parsed.vocabularies);
  inputs.adapter.withEveryFile();
  const report = complete(inputs, (adapter, vocabulary) =>
    test(adapter, vocabulary, { datasets: parsed.datasets }),
  );
  const offers = report.bridge.profiles.map((p) => p.split("#")[1] ?? p).join(", ");
  let said = `${adapterLine(description.identifier, description.iri)}\nBridge   ${report.bridge.name} ${report.bridge.version}, offers ${offers}\n\n`;
  const width = Math.max(4, ...report.entries.map(({ name }) => name.length));
  for (const entry of report.entries) {
    said += `  ${entry.outcome.padEnd(12)} ${entry.name.padEnd(width)}  ${entry.seconds.toFixed(2).padStart(6)} s  ${entry.description}\n`;
  }
  said += `\n${tally(report.entries.map(({ outcome }) => outcome))}\n`;
  process.stdout.write(said);
  if (parsed.earl !== undefined) {
    at(parsed.earl, () => writeFileSync(parsed.earl, report.earl));
    process.stdout.write(`EARL     ${parsed.earl}\n`);
  }
  return report.entries.some(({ outcome }) => FAILING.includes(outcome)) ? 1 : 0;
}

function runConvert(parsed) {
  const [inputs, description] = opened(parsed.directory, parsed.vocabularies);
  inputs.adapter.with(description.loadFiles);
  const adapter = complete(inputs, (named, vocabulary) => Adapter.load(named, vocabulary));
  try {
    const bytes = at(parsed.document, () => new Uint8Array(readFileSync(parsed.document)));
    const iri = iriOf(parsed.document);
    let envelope;
    if (parsed.envelope !== undefined) {
      try {
        envelope = new URL(parsed.envelope, `${inputs.adapter.iri}${METADATA}`).href;
      } catch (error) {
        throw new Refusal(`--envelope ${parsed.envelope}: ${error.message}`);
      }
    }
    const facts =
      parsed.facts === undefined
        ? undefined
        : {
            iri: iriOf(parsed.facts),
            bytes: at(parsed.facts, () => new Uint8Array(readFileSync(parsed.facts))),
          };
    const conversion = adapter.convert(
      { iri, bytes, envelope, facts },
      {
        format: parsed.format,
        findingsRelativeTo: parsed.findings === undefined ? undefined : iriOfWritten(parsed.findings),
      },
    );
    if (parsed.findings !== undefined && conversion.unvalidated !== undefined) {
      throw new Refusal(conversion.unvalidated);
    }
    if (conversion.unvalidated !== undefined) {
      process.stderr.write(`cascade-bridge: ${conversion.unvalidated}\n`);
    }
    const detect =
      conversion.detected === undefined
        ? "the adapter names no bridge:detectQuery"
        : conversion.detected
          ? "true"
          : "false: this adapter does not claim this document, converted anyway";
    process.stderr.write(
      `${adapterLine(description.identifier, description.iri)}\nDocument ${iri}  ${conversion.records} record(s), ${conversion.triples} triples, ${conversion.findingCount} finding(s)\nDetect   ${detect}\n`,
    );
    // Before the graph, so a findings file that cannot be written leaves standard output empty.
    if (parsed.findings !== undefined) {
      // Not emptied first, so a run that fails leaves a committed oracle as it was.
      at(parsed.findings, () => closeSync(openSync(parsed.findings, "a")));
      at(parsed.findings, () => writeFileSync(parsed.findings, conversion.findings));
      process.stderr.write(`Findings ${parsed.findings}\n`);
    }
    if (parsed.out !== undefined) {
      at(parsed.out, () => writeFileSync(parsed.out, conversion.graph));
      process.stderr.write(`Graph    ${parsed.out}\n`);
    } else {
      process.stdout.write(conversion.graph);
    }
    return 0;
  } finally {
    adapter.free();
  }
}

// A pipe's reader may go away mid-graph; the caller is owed a status, not an exception.
process.stdout.on("error", (error) => {
  process.stderr.write(`cascade-bridge: standard output: ${error.message}\n`);
  process.exitCode = 2;
});

function said(error) {
  if (error instanceof WebAssembly.RuntimeError) {
    return `a fault in the bridge, whose module trapped: ${error.message}`;
  }
  return error instanceof Error ? error.message : String(error);
}

let parsed;
try {
  parsed = parse(process.argv.slice(2));
} catch {
  parsed = undefined;
}
if (parsed === undefined) {
  process.stderr.write(`${USAGE}\n`);
  process.exitCode = 2;
} else {
  try {
    process.exitCode = parsed.command === "test" ? runTest(parsed) : runConvert(parsed);
  } catch (error) {
    process.stderr.write(`cascade-bridge: ${said(error)}\n`);
    process.exitCode = 2;
  }
}
