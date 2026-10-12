import assert from "node:assert/strict";
import { after, test } from "node:test";
import { ADAPTER, discardInstances, DOCUMENT, files, instance, toLoad, VOCABULARY } from "./package.js";

after(discardInstances);

const JSON_ADAPTER = "https://example.org/tiny-json-adapter/";
const MAPPING = "mapping/item.rq";
const VOCABULARY_FILE = "ontologies/catalog/v1/catalog.ttl";
const NTRIPLES = { format: "ntriples" };

const tiny = files("tiny-adapter");
const unaccounted = { iri: DOCUMENT, bytes: tiny.get("fixtures/in/unaccounted-child.xml") };
const order = { iri: DOCUMENT, bytes: tiny.get("fixtures/in/order.xml") };
const jsonFiles = files("tiny-json-adapter");
const item = { iri: "https://example.org/documents/item.json", bytes: jsonFiles.get("fixtures/in/item.json") };

function given() {
  return {
    adapter: { iri: ADAPTER, files: toLoad() },
    vocabulary: { iri: VOCABULARY, files: files("tiny-vocabularies") },
  };
}

function lines(bytes) {
  return new TextDecoder()
    .decode(bytes)
    .split("\n")
    .filter((line) => line !== "")
    .map((line) => line.replace(/_:[A-Za-z0-9_.-]+/g, "_:blank"))
    .sort();
}

function meaning(conversion) {
  return {
    triples: conversion.triples,
    findingCount: conversion.findingCount,
    graph: lines(conversion.graph),
    findings: lines(conversion.findings),
  };
}

function aByteMore(map, path) {
  const edited = new Map(map);
  edited.set(path, Uint8Array.of(...edited.get(path), 32));
  return edited;
}

async function loaded({ adapter, vocabulary } = given()) {
  return (await instance()).Adapter.load(adapter, vocabulary);
}

const source = await loaded();
const snapshot = source.snapshot();
const alone = (await instance()).Adapter.load({ iri: JSON_ADAPTER, files: jsonFiles });

test("a restored adapter converts a document to the triples and findings a fresh load gives", async () => {
  const fresh = meaning(source.convert(unaccounted, NTRIPLES));
  assert.ok(fresh.triples > 0 && fresh.findingCount > 0, "the document shows no triple or no finding");
  const { restore, describe } = await instance();
  const { adapter, vocabulary } = given();
  describe(ADAPTER, adapter.files.get("ro-crate-metadata.json"));
  assert.deepEqual(meaning(restore(snapshot, adapter, vocabulary).convert(unaccounted, NTRIPLES)), fresh);
});

test("a restored adapter converts a second document, and an adapter loaded after the restore converts", async () => {
  const { Adapter, restore } = await instance();
  const { adapter, vocabulary } = given();
  const restored = restore(snapshot, adapter, vocabulary);
  for (const document of [unaccounted, order]) {
    assert.deepEqual(meaning(restored.convert(document, NTRIPLES)), meaning(source.convert(document, NTRIPLES)));
  }
  const second = Adapter.load({ iri: JSON_ADAPTER, files: jsonFiles });
  assert.deepEqual(meaning(second.convert(item, NTRIPLES)), meaning(alone.convert(item, NTRIPLES)));
  assert.deepEqual(meaning(restored.convert(order, NTRIPLES)), meaning(source.convert(order, NTRIPLES)));
});

test("an instance whose memory is already larger than the image restores it, and loads another adapter after", async () => {
  const { Adapter, restore, initSync } = await instance();
  initSync().memory.grow(Math.ceil(snapshot.byteLength / 65536));
  const { adapter, vocabulary } = given();
  const restored = restore(snapshot, adapter, vocabulary);
  assert.deepEqual(meaning(restored.convert(order, NTRIPLES)), meaning(source.convert(order, NTRIPLES)));
  const second = Adapter.load({ iri: JSON_ADAPTER, files: jsonFiles });
  assert.deepEqual(meaning(second.convert(item, NTRIPLES)), meaning(alone.convert(item, NTRIPLES)));
});

function altered(at) {
  const bytes = snapshot.slice();
  bytes[at] ^= 1;
  return bytes;
}

const refused = {
  "an adapter file changed by a byte": ({ adapter, vocabulary }) => [
    snapshot,
    { iri: ADAPTER, files: aByteMore(adapter.files, MAPPING) },
    vocabulary,
  ],
  "a vocabulary file changed by a byte": ({ adapter, vocabulary }) => [
    snapshot,
    adapter,
    { iri: VOCABULARY, files: aByteMore(vocabulary.files, VOCABULARY_FILE) },
  ],
  "a file the load read left out": ({ adapter, vocabulary }) => {
    const left = new Map(adapter.files);
    left.delete(MAPPING);
    return [snapshot, { iri: ADAPTER, files: left }, vocabulary];
  },
  "another adapter's files": ({ vocabulary }) => [snapshot, { iri: ADAPTER, files: jsonFiles }, vocabulary],
  "no vocabulary where the snapshot has one": ({ adapter }) => [snapshot, adapter, undefined],
  "another adapter IRI": ({ adapter, vocabulary }) => [snapshot, { ...adapter, iri: `${ADAPTER}again/` }, vocabulary],
  "another vocabulary IRI": ({ adapter, vocabulary }) => [snapshot, adapter, { ...vocabulary, iri: `${VOCABULARY}again/` }],
  "empty bytes": ({ adapter, vocabulary }) => [new Uint8Array(0), adapter, vocabulary],
  "bytes that are no snapshot": ({ adapter, vocabulary }) => [new Uint8Array(4096).fill(7), adapter, vocabulary],
  "a snapshot cut inside its header": ({ adapter, vocabulary }) => [snapshot.subarray(0, 100), adapter, vocabulary],
  "a snapshot cut inside its image": ({ adapter, vocabulary }) => [
    snapshot.subarray(0, snapshot.byteLength - 65536),
    adapter,
    vocabulary,
  ],
  "an image with a byte changed": ({ adapter, vocabulary }) => [altered(snapshot.byteLength - 1000), adapter, vocabulary],
  "a header with a byte changed": ({ adapter, vocabulary }) => [altered(200), adapter, vocabulary],
  "a first byte changed": ({ adapter, vocabulary }) => [altered(0), adapter, vocabulary],
};

test("restore returns undefined for each snapshot it must refuse, and the refusals leave the next restore whole", async () => {
  const { restore } = await instance();
  for (const [name, refusal] of Object.entries(refused)) {
    assert.equal(restore(...refusal(given())), undefined, name);
  }
  const { adapter, vocabulary } = given();
  assert.deepEqual(
    meaning(restore(snapshot, adapter, vocabulary).convert(order, NTRIPLES)),
    meaning(source.convert(order, NTRIPLES)),
  );
});

test("restore returns undefined for a snapshot of another build of the module", async () => {
  const customSection = Uint8Array.of(0, 2, 1, 0x78);
  const { restore } = await instance((wasm) => Buffer.concat([wasm, customSection]));
  const { adapter, vocabulary } = given();
  assert.equal(restore(snapshot, adapter, vocabulary), undefined);
});

test("a snapshot depends on the files the load read, so maps with more or fewer files restore it", async () => {
  const wide = { adapter: { iri: ADAPTER, files: files("tiny-adapter") }, vocabulary: given().vocabulary };
  const widened = (await loaded(wide)).snapshot();
  const { snapshotName } = await instance();
  assert.match(snapshotName(widened), /^[0-9a-f]{16}$/);
  assert.equal(snapshotName(widened), snapshotName(snapshot));
  const fewer = given();
  assert.notEqual((await instance()).restore(widened, fewer.adapter, fewer.vocabulary), undefined);
  assert.notEqual((await instance()).restore(snapshot, wide.adapter, wide.vocabulary), undefined);
});

test("restore throws in a module instance that has loaded an adapter, or restored one", async () => {
  const { adapter, vocabulary } = given();
  const loading = await instance();
  loading.Adapter.load(adapter, vocabulary);
  assert.throws(() => loading.restore(snapshot, adapter, vocabulary), { name: "Error" });
  const restoring = await instance();
  restoring.restore(snapshot, adapter, vocabulary);
  assert.throws(() => restoring.restore(snapshot, adapter, vocabulary), { name: "Error" });
});

test("snapshot throws for an adapter freed, and for any in a module instance that made another", async () => {
  const { adapter, vocabulary } = given();
  const freed = await loaded();
  freed.free();
  assert.throws(() => freed.snapshot(), { name: "Error" });
  const { Adapter } = await instance();
  const first = Adapter.load(adapter, vocabulary);
  Adapter.load({ iri: JSON_ADAPTER, files: jsonFiles });
  assert.throws(() => first.snapshot(), { name: "Error" });
});
