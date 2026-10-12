// One process of the snapshot measurement: `load` loads the adapter, `save` loads it and writes
// its snapshot to a file, `restore` restores from that file and loads only where it is refused.
// It says what it measured as one JSON line, the output of a last convert as hashes.
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { gunzipSync } from "node:zlib";
import { at, complete, Folder, iriOf, METADATA } from "./host.mjs";

const given = JSON.parse(process.argv[2]);
const { Adapter, describe, initSync, restore } = await import(pathToFileURL(join(given.package, "node.js")).href);
const instance = initSync();

const milliseconds = (since) => performance.now() - since;
const hash = (lines) => createHash("sha256").update(lines.join("\n")).digest("hex");
const lines = (bytes, normalise) =>
  new TextDecoder()
    .decode(bytes)
    .split("\n")
    .filter((line) => line !== "")
    .map((line) => (normalise ? line.replace(/_:[A-Za-z0-9_.-]+/g, "_:blank") : line))
    .sort();

const started = performance.now();
const inputs = {
  adapter: new Folder(given.adapter),
  vocabulary: given.vocabularies === undefined ? undefined : new Folder(given.vocabularies),
};
const description = describe(inputs.adapter.iri, inputs.adapter.read(METADATA));
inputs.adapter.with(description.loadFiles);
inputs.vocabulary?.with(description.vocabulary?.files ?? []);
const read = milliseconds(started);

let calls = 0;
const loading = performance.now();
let adapter = given.mode === "restore" ? readSnapshot() : undefined;
const restored = adapter !== undefined;
adapter ??= complete(inputs, (named, vocabulary) => {
  calls += 1;
  return Adapter.load(named, vocabulary);
});
const load = milliseconds(loading);

function readSnapshot() {
  const bytes = new Uint8Array(readFileSync(given.snapshot));
  return restore(bytes, inputs.adapter.named(), inputs.vocabulary?.named());
}

const written = at(given.document, () => readFileSync(given.document));
const document = {
  iri: iriOf(given.document),
  bytes: new Uint8Array(given.document.endsWith(".gz") ? gunzipSync(written) : written),
  envelope: given.envelope === undefined ? undefined : new URL(given.envelope, `${inputs.adapter.iri}${METADATA}`).href,
};

let snapshotBytes;
if (given.mode === "save") {
  const snapshotting = performance.now();
  const snapshot = adapter.snapshot();
  snapshotBytes = { bytes: snapshot.byteLength, milliseconds: milliseconds(snapshotting) };
  writeFileSync(given.snapshot, snapshot);
}

let turtle;
const convert = () => {
  const converting = performance.now();
  const conversion = adapter.convert(document);
  const took = milliseconds(converting);
  turtle ??= { graph: hash(lines(conversion.graph, false)), findings: hash(lines(conversion.findings, false)) };
  return { milliseconds: took, triples: conversion.triples };
};
const first = convert();
const second = convert();
const output = adapter.convert(document, { format: "ntriples" });

process.stdout.write(
  `${JSON.stringify({
    mode: given.mode,
    restored,
    loadCalls: calls,
    read,
    load,
    first,
    second,
    memory: instance.memory.buffer.byteLength,
    grows: instance.memory_grow_calls?.(),
    rss: process.resourceUsage().maxRSS * 1024,
    snapshot: snapshotBytes,
    findings: output.findingCount,
    turtle,
    graphUpToBlankNames: hash(lines(output.graph, true)),
    findingsUpToBlankNames: hash(lines(output.findings, true)),
  })}\n`,
);
