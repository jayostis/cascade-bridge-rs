// One sample of `bench.mjs`: boot the module, load the adapter, then convert the document twice.
// It says each phase as it starts, then what it measured, one JSON object per line.
//
// `given.mode` is "timing" (default), whose two converts are unprofiled, so their seconds are
// those of a caller that asked for no profile, "profile", whose first convert is profiled and
// whose second writes `graph.nt` and `findings.nt` to `given.outputDirectory` as N-Triples, for
// the digest, or "digest", whose one convert writes them. A profiled convert is slower than an
// unprofiled one, so the two are never mixed.
//
// `given.skip` is a measurement's load: it names what the sample leaves out ("schema",
// "shapes", "findings"), possibly nothing, and times the load by stage. A sample that
// leaves something out says so on standard error and in its result, and is no validated
// conversion. Without `given.skip` the adapter is loaded as a caller loads it.
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { gunzipSync } from "node:zlib";
import { at, complete, Folder, iriOf, KINDS, METADATA, Refusal } from "./host.mjs";

const given = JSON.parse(process.argv[2]);
const say = (line) => process.stdout.write(`${JSON.stringify(line)}\n`);
const seconds = (from, to) => (to - from) / 1000;
const skip = given.skip ?? [];
if (skip.length > 0) {
  process.stderr.write(`MEASUREMENT ONLY: this sample skips ${skip.join(", ")}; its conversion is not a validated one\n`);
}

const glue = await import(pathToFileURL(join(given.package, "cascade_bridge.js")).href);
const { Adapter, describe } = glue;

// What package/node.js does, in the steps it is made of: the file is read, compiled, then instantiated.
const readStarted = performance.now();
const wasm = readFileSync(join(given.package, "cascade_bridge_bg.wasm"));
const compileStarted = performance.now();
const compiled = new WebAssembly.Module(wasm);
const instantiateStarted = performance.now();
const instance = glue.initSync({ module: compiled });
const booting = {
  readWasmSeconds: seconds(readStarted, compileStarted),
  compileSeconds: seconds(compileStarted, instantiateStarted),
  instantiateSeconds: seconds(instantiateStarted, performance.now()),
};
const grows = typeof instance.memory_grow_calls === "function" ? () => instance.memory_grow_calls() : () => undefined;
const held = () => instance.memory.buffer.byteLength;
const since = (before, now) => (before === undefined ? undefined : now - before);

function documentOf(adapter) {
  const read = at(given.document, () => readFileSync(given.document));
  return {
    iri: iriOf(given.document),
    bytes: new Uint8Array(given.document.endsWith(".gz") ? gunzipSync(read) : read),
    envelope: given.envelope === undefined ? undefined : new URL(given.envelope, `${adapter.iri}${METADATA}`).href,
  };
}

// The load, as it always was: the adapter's files read, then described and prepared. The time
// spent reading files is told apart from the rest.
function loaded() {
  const started = performance.now();
  let reading = 0;
  const inputs = {
    adapter: new Folder(given.adapter),
    vocabulary: given.vocabularies === undefined ? undefined : new Folder(given.vocabularies),
  };
  for (const folder of [inputs.adapter, inputs.vocabulary]) {
    if (folder === undefined) continue;
    const read = folder.read.bind(folder);
    folder.read = (key) => {
      const before = performance.now();
      try {
        return read(key);
      } finally {
        reading += performance.now() - before;
      }
    };
  }
  const description = describe(inputs.adapter.iri, inputs.adapter.read(METADATA));
  inputs.adapter.with(description.loadFiles);
  inputs.vocabulary?.with(description.vocabulary?.files ?? []);
  const count = () => inputs.adapter.files.size + (inputs.vocabulary?.files.size ?? 0);
  const listed = count();
  let adapter;
  try {
    adapter = complete(inputs, (named, vocabulary) =>
      given.skip === undefined ? Adapter.load(named, vocabulary) : Adapter.loadForMeasurement(named, vocabulary, skip),
    );
  } catch (error) {
    if (error instanceof Refusal) throw Object.assign(error, { kind: "fileMissingFailure" });
    throw error;
  }
  const total = performance.now() - started;
  const load = {
    identifier: description.identifier,
    seconds: total / 1000,
    readFilesSeconds: reading / 1000,
    prepareSeconds: (total - reading) / 1000,
    filesRead: count(),
    missingFiles: count() - listed,
    stages: adapter.loadStages(),
  };
  return { adapter, document: documentOf(inputs.adapter), load };
}

// What a convert measured. Its output is kept only where `written` will want it.
function converted(adapter, document, options, written) {
  const before = { at: performance.now(), grows: grows() };
  const conversion = adapter.convert(document, {
    findings: true,
    census: true,
    executor: given.executor,
    memoryBudgetMiB: given.memoryBudgetMiB,
    timeBudgetSeconds: given.timeBudgetSeconds,
    ...options,
  });
  const measures = {
    seconds: (performance.now() - before.at) / 1000,
    grows: since(before.grows, grows()),
    memory: held(),
    records: conversion.records,
    triples: conversion.triples,
    findings: conversion.findingCount,
    graphBytes: conversion.graph.byteLength,
    findingsBytes: conversion.findings?.byteLength ?? 0,
    queries: conversion.queries,
    stages: conversion.stages,
  };
  if (written) {
    writeFileSync(join(given.outputDirectory, "graph.nt"), conversion.graph);
    writeFileSync(join(given.outputDirectory, "findings.nt"), conversion.findings ?? new Uint8Array());
  }
  return measures;
}

try {
  say({ phase: "load" });
  const { adapter, document, load } = loaded();
  say({ phase: "convert" });
  const profiling = given.mode === "profile";
  const digesting = given.mode === "digest";
  const first = converted(adapter, document, digesting ? { format: "ntriples" } : { profile: profiling }, digesting);
  const converts = [first];
  if (!digesting) {
    say({ phase: "convert again" });
    converts.push(converted(adapter, document, profiling ? { format: "ntriples" } : {}, profiling));
  }
  say({
    sample: {
      boot: booting,
      load,
      converts,
      rss: process.resourceUsage().maxRSS * 1024,
      skipped: skip,
    },
  });
} catch (error) {
  const kind =
    error?.name === "BridgeError"
      ? KINDS[error.kind]
      : error instanceof WebAssembly.RuntimeError
        ? "bridgeFailure"
        : error?.kind;
  if (kind === undefined) throw error;
  say({ refused: kind, message: error.message });
}
