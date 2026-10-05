import assert from "node:assert/strict";
import { test } from "node:test";
import { bridge, files } from "./package.js";

const ADAPTER = "https://example.org/adapters/catalog/";
const VOCABULARY = "https://example.org/vocabularies/";
const DOCUMENT = "https://example.org/documents/catalog.xml";
const MAPPING = "mapping/item.rq";

function toLoad() {
  const adapter = files("tiny-adapter");
  for (const path of [...adapter.keys()]) {
    if (path.startsWith("fixtures/")) adapter.delete(path);
  }
  return adapter;
}

const tiny = files("tiny-adapter");
const two = tiny.get("fixtures/in/two.xml");
const vocabulary = { iri: VOCABULARY, files: files("tiny-vocabularies") };

function document(bytes) {
  return { iri: DOCUMENT, bytes };
}

test("loads the tiny adapter from a Map and converts a document", async () => {
  const { Adapter } = await bridge();
  const adapter = Adapter.load({ iri: ADAPTER, files: toLoad() }, vocabulary);
  const { graph, findings } = adapter.convert(document(two));
  assert.ok(graph instanceof Uint8Array, `the graph is ${typeof graph}`);
  assert.ok(findings instanceof Uint8Array, `the findings are ${typeof findings}`);
  assert.match(new TextDecoder().decode(graph), /urn:example:item:1/);
});

test("a document that does not parse throws a BridgeError of kind document", async () => {
  const { Adapter } = await bridge();
  const adapter = Adapter.load({ iri: ADAPTER, files: toLoad() }, vocabulary);
  assert.throws(
    () => adapter.convert(document(new TextEncoder().encode("<catalog><item"))),
    (error) => {
      assert.equal(error.name, "BridgeError");
      assert.equal(error.kind, "document");
      return true;
    },
  );
});

test("a mapping left out throws a BridgeError of kind missing naming the adapter's map and the path", async () => {
  const { Adapter } = await bridge();
  const adapter = toLoad();
  adapter.delete(MAPPING);
  assert.throws(
    () => Adapter.load({ iri: ADAPTER, files: adapter }, vocabulary).convert(document(two)),
    (error) => {
      assert.equal(error.name, "BridgeError");
      assert.equal(error.kind, "missing");
      assert.equal(error.map, "adapter");
      assert.equal(error.path, MAPPING);
      return true;
    },
  );
});

test("a plain object in place of a Map throws a TypeError", async () => {
  const { Adapter } = await bridge();
  const plain = Object.fromEntries(toLoad());
  assert.throws(() => Adapter.load({ iri: ADAPTER, files: plain }, vocabulary), TypeError);
});

test("a freed adapter throws on convert", async () => {
  const { Adapter } = await bridge();
  const adapter = Adapter.load({ iri: ADAPTER, files: toLoad() }, vocabulary);
  adapter.free();
  assert.throws(() => adapter.convert(document(two)));
});
