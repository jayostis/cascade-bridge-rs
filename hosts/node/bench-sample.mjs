// One sample of `bench.mjs`: load the adapter, then convert the document twice. It says
// each phase as it starts, then what it measured, one JSON object per line.
//
// `given.skip` is a measurement's load: it names what the sample leaves out ("schema",
// "shapes", "findings"), possibly nothing, and times the load by stage. A sample that
// leaves something out says so on standard error and in its result, and is no validated
// conversion. Without `given.skip` the adapter is loaded as a caller loads it.
// `given.profile` says, for each convert, whether it is profiled (default: the first).
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { gunzipSync } from "node:zlib";
import { at, complete, Folder, iriOf, KINDS, METADATA, Refusal } from "./host.mjs";

const given = JSON.parse(process.argv[2]);
const say = (line) => process.stdout.write(`${JSON.stringify(line)}\n`);
const skip = given.skip ?? [];
if (skip.length > 0) {
  process.stderr.write(`MEASUREMENT ONLY: this sample skips ${skip.join(", ")}; its conversion is not a validated one\n`);
}

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
    adapter = complete(inputs, (named, vocabulary) =>
      given.skip === undefined ? Adapter.load(named, vocabulary) : Adapter.loadForMeasurement(named, vocabulary, skip),
    );
  } catch (error) {
    if (error instanceof Refusal) throw Object.assign(error, { kind: "fileMissingFailure" });
    throw error;
  }
  const load = {
    identifier: description.identifier,
    seconds: (performance.now() - started) / 1000,
    filesRead: count(),
    missingFiles: count() - listed,
    stages: adapter.loadStages(),
  };
  return { adapter, document: documentOf(inputs.adapter), load };
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
    stages: conversion.stages,
  };
}

try {
  say({ phase: "load" });
  const { adapter, document, load } = loaded();
  say({ phase: "convert" });
  const [profileFirst, profileSecond] = given.profile ?? [true, false];
  const first = converted(adapter, document, { profile: profileFirst });
  const second = converted(adapter, document, { profile: profileSecond });
  say({
    sample: { load, converts: [first, second], rss: process.resourceUsage().maxRSS * 1024, skipped: skip },
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
