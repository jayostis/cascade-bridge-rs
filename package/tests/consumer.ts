import { Adapter, describe, test } from "../dist/cascade_bridge.js";
import type { BridgeError, Conversion, Document, Named, TestReport } from "../dist/cascade_bridge.js";

const files: Map<string, Uint8Array> = new Map();
const adapter: Named = { iri: "https://example.org/adapters/catalog/", files };
const vocabulary: Named = { iri: "https://example.org/vocabularies/", files: new Map() };

const described = describe(adapter.iri, new Uint8Array());
void described;

const document: Document = {
  iri: "https://example.org/documents/catalog.xml",
  bytes: new Uint8Array(),
  facts: { iri: "https://example.org/facts/catalog.ttl", bytes: new Uint8Array() },
  envelope: "https://example.org/adapters/catalog/ro-crate-metadata.json#envelope-catalog",
};

const loaded: Adapter = Adapter.load(adapter, vocabulary);
const accepted: boolean = loaded.accepts(document);
const conversion: Conversion = loaded.convert(document);
const graph: Uint8Array = conversion.graph;
const findings: Uint8Array = conversion.findings;
loaded.free();
void [accepted, graph, findings];

const report: TestReport = test(adapter, vocabulary);
const earl: Uint8Array = report.earl;
void earl;

try {
  Adapter.load(adapter);
} catch (thrown) {
  const failure = thrown as BridgeError;
  const kind: "document" | "facts" | "adapter" | "vocabulary" | "missing" | "bridge" = failure.kind;
  const map: "adapter" | "vocabulary" | undefined = failure.map;
  const path: string | undefined = failure.path;
  const error: Error = failure;
  void [kind, map, path, error];
}

// @ts-expect-error a plain object is not a Map
Adapter.load({ iri: adapter.iri, files: {} });
