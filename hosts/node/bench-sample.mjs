// One sample of `bench.mjs`, in a fresh process so its first convert starts from a cold
// module: load the adapter, then convert the document twice. It says each phase as it
// starts, then what it measured, one JSON object per line.
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { gunzipSync } from "node:zlib";
import { at, complete, Folder, iriOf, KINDS, METADATA, Refusal } from "./host.mjs";

const given = JSON.parse(process.argv[2]);
const say = (line) => process.stdout.write(`${JSON.stringify(line)}\n`);

const { Adapter, describe, initSync } = await import(pathToFileURL(join(given.package, "node.js")).href);
const instance = initSync();
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

function loaded() {
  const started = performance.now();
  const inputs = {
    adapter: new Folder(given.adapter),
    vocabulary: given.vocabularies === undefined ? undefined : new Folder(given.vocabularies),
  };
  const description = describe(inputs.adapter.iri, inputs.adapter.read(METADATA));
  inputs.adapter.with(description.loadFiles);
  inputs.vocabulary?.with(description.vocabulary?.files ?? []);
  const count = () => inputs.adapter.files.size + (inputs.vocabulary?.files.size ?? 0);
  const listed = count();
  let adapter;
  try {
    adapter = complete(inputs, (named, vocabulary) => Adapter.load(named, vocabulary));
  } catch (error) {
    // complete refuses only a file the load asked for that its folder lacks.
    if (error instanceof Refusal) throw Object.assign(error, { kind: "fileMissingFailure" });
    throw error;
  }
  return {
    adapter,
    document: documentOf(inputs.adapter),
    load: {
      identifier: description.identifier,
      seconds: (performance.now() - started) / 1000,
      filesRead: count(),
      missingFiles: count() - listed,
    },
  };
}

function converted(adapter, document, options) {
  const before = { at: performance.now(), grows: grows() };
  const conversion = adapter.convert(document, options);
  return {
    seconds: (performance.now() - before.at) / 1000,
    grows: since(before.grows, grows()),
    memory: held(),
    records: conversion.records,
    triples: conversion.triples,
    findings: conversion.findingCount,
    graphBytes: conversion.graph.byteLength,
    findingsBytes: conversion.findings.byteLength,
    queries: conversion.queries,
  };
}

try {
  say({ phase: "load" });
  const { adapter, document, load } = loaded();
  say({ phase: "convert" });
  const first = converted(adapter, document, { profile: true });
  const second = converted(adapter, document);
  say({ sample: { load, converts: [first, second], rss: process.resourceUsage().maxRSS * 1024 } });
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
