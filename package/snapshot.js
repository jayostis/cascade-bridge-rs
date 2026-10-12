import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { Adapter, initSync } from "./cascade_bridge.js";

const MAGIC = [0x43, 0x42, 0x53, 0x4e];
const FORMAT = 1;
const PREFIX = 8;
const PAGE = 65536;

const loads = new WeakMap();
let made = 0;
let wasmDigest;

function sha256(...parts) {
  const hash = createHash("sha256");
  for (const part of parts) hash.update(part);
  return hash.digest("hex");
}

function moduleDigest() {
  wasmDigest ??= sha256(readFileSync(new URL("./cascade_bridge_bg.wasm", import.meta.url)));
  return wasmDigest;
}

function imageDigest(pointer, image) {
  return sha256(`${pointer}\0`, image);
}

function digests(named, paths, what) {
  if (!(named.files instanceof Map)) throw new TypeError(`${what}.files is not a Map`);
  return paths.map((path) => {
    const bytes = named.files.get(path);
    return [path, bytes instanceof Uint8Array ? sha256(bytes) : null];
  });
}

function filesDigest(adapter, vocabulary) {
  const lines = [];
  for (const [map, entries] of [["adapter", adapter], ["vocabulary", vocabulary]]) {
    for (const [path, digest] of [...entries].sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))) {
      lines.push(`${map}\0${path}\0${digest ?? "-"}\n`);
    }
  }
  return sha256(...lines);
}

function remember(adapter, given, vocabulary) {
  const read = adapter.filesRead();
  loads.set(adapter, {
    adapter: { iri: given.iri, entries: digests(given, read.adapter, "adapter") },
    vocabulary:
      vocabulary === undefined || vocabulary === null
        ? null
        : { iri: vocabulary.iri, entries: digests(vocabulary, read.vocabulary, "vocabulary") },
  });
  made += 1;
}

const load = Adapter.load;
Adapter.load = function (adapter, vocabulary) {
  const loaded = load.call(this, adapter, vocabulary);
  remember(loaded, adapter, vocabulary);
  return loaded;
};

Adapter.prototype.snapshot = function snapshot() {
  const state = loads.get(this);
  if (state === undefined || this.__wbg_ptr === 0) {
    throw new Error("only an adapter this module instance made and has not freed can be snapshot");
  }
  if (made !== 1) {
    throw new Error("a module instance that made more than one adapter cannot snapshot one: the image holds them all");
  }
  const { memory } = initSync();
  const image = new Uint8Array(memory.buffer);
  const header = new TextEncoder().encode(
    JSON.stringify({
      format: FORMAT,
      module: moduleDigest(),
      adapter: { iri: state.adapter.iri, files: state.adapter.entries.map(([path]) => path) },
      vocabulary:
        state.vocabulary === null
          ? null
          : { iri: state.vocabulary.iri, files: state.vocabulary.entries.map(([path]) => path) },
      files: filesDigest(state.adapter.entries, state.vocabulary?.entries ?? []),
      image: {
        bytes: image.byteLength,
        sha256: imageDigest(this.__wbg_ptr, image),
        pointer: this.__wbg_ptr,
      },
    }),
  );
  const snapshot = new Uint8Array(PREFIX + header.byteLength + image.byteLength);
  snapshot.set(MAGIC);
  new DataView(snapshot.buffer).setUint32(MAGIC.length, header.byteLength, true);
  snapshot.set(header, PREFIX);
  snapshot.set(image, PREFIX + header.byteLength);
  return snapshot;
};

const list = (value) => Array.isArray(value) && value.every((item) => typeof item === "string");
const named = (value) =>
  value !== null && typeof value === "object" && typeof value.iri === "string" && list(value.files);

function parsed(snapshot) {
  if (snapshot.byteLength < PREFIX || MAGIC.some((byte, at) => snapshot[at] !== byte)) return undefined;
  const length = new DataView(snapshot.buffer, snapshot.byteOffset, snapshot.byteLength).getUint32(MAGIC.length, true);
  if (PREFIX + length > snapshot.byteLength) return undefined;
  let header;
  try {
    header = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(snapshot.subarray(PREFIX, PREFIX + length)));
  } catch {
    return undefined;
  }
  const image = snapshot.subarray(PREFIX + length);
  const ok =
    header?.format === FORMAT &&
    typeof header.module === "string" &&
    named(header.adapter) &&
    (header.vocabulary === null || named(header.vocabulary)) &&
    typeof header.files === "string" &&
    Number.isSafeInteger(header.image?.bytes) &&
    header.image.bytes === image.byteLength &&
    header.image.bytes % PAGE === 0 &&
    typeof header.image.sha256 === "string" &&
    Number.isSafeInteger(header.image.pointer) &&
    header.image.pointer > 0 &&
    header.image.pointer < header.image.bytes;
  return ok ? { header, image } : undefined;
}

export function snapshotName(snapshot) {
  const read = snapshot instanceof Uint8Array ? parsed(snapshot) : undefined;
  if (read === undefined) throw new TypeError("not a snapshot");
  return read.header.files.slice(0, 16);
}

export function restore(snapshot, adapter, vocabulary) {
  if (!(snapshot instanceof Uint8Array)) throw new TypeError("a snapshot is a Uint8Array");
  if (made > 0) {
    throw new Error("this module instance has made an adapter already, and a restore replaces the whole instance memory");
  }
  const read = parsed(snapshot);
  if (read === undefined) return undefined;
  const { header, image } = read;
  const given = vocabulary !== undefined && vocabulary !== null;
  if (header.module !== moduleDigest()) return undefined;
  if (adapter.iri !== header.adapter.iri) return undefined;
  if (given !== (header.vocabulary !== null)) return undefined;
  if (given && vocabulary.iri !== header.vocabulary.iri) return undefined;
  const entries = {
    adapter: digests(adapter, header.adapter.files, "adapter"),
    vocabulary: given ? digests(vocabulary, header.vocabulary.files, "vocabulary") : [],
  };
  if (filesDigest(entries.adapter, entries.vocabulary) !== header.files) return undefined;
  if (imageDigest(header.image.pointer, image) !== header.image.sha256) return undefined;
  const { memory } = initSync();
  const held = memory.buffer.byteLength;
  if (held < image.byteLength) memory.grow((image.byteLength - held) / PAGE);
  new Uint8Array(memory.buffer).set(image);
  const restored = Adapter.__wrap(header.image.pointer);
  loads.set(restored, {
    adapter: { iri: adapter.iri, entries: entries.adapter },
    vocabulary: given ? { iri: vocabulary.iri, entries: entries.vocabulary } : null,
  });
  made += 1;
  return restored;
}
