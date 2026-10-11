// What the Node host's scripts share: folders read into maps, and the IRIs they are read at.
import { lstatSync, readdirSync, readFileSync, realpathSync } from "node:fs";
import { join } from "node:path";

export const METADATA = "ro-crate-metadata.json";

export const KINDS = {
  document: "documentFailure",
  facts: "factsFailure",
  adapter: "adapterFailure",
  vocabulary: "vocabularyFailure",
  missing: "fileMissingFailure",
  bridge: "bridgeFailure",
};

export class Refusal extends Error {}

export function reason(error) {
  return error.code ?? error.message;
}

export function at(path, f) {
  try {
    return f();
  } catch (error) {
    throw new Refusal(`${path}: ${reason(error)}`);
  }
}

function encoded(text) {
  let out = "";
  for (const byte of new TextEncoder().encode(text)) {
    const kept = byte < 0x80 && /[A-Za-z0-9\-._~/:]/.test(String.fromCharCode(byte));
    out += kept ? String.fromCharCode(byte) : `%${byte.toString(16).toUpperCase().padStart(2, "0")}`;
  }
  return out;
}

// Spelled as the native host spells it, so both hosts name a path alike.
export function fileIri(path) {
  const text = path.replaceAll("\\", "/");
  const unc = text.startsWith("//?/UNC/")
    ? text.slice("//?/UNC/".length)
    : text.startsWith("//") && !text.startsWith("//?/")
      ? text.slice(2)
      : undefined;
  let server = "";
  let rest = text.startsWith("//?/") ? text.slice("//?/".length) : text;
  if (unc !== undefined) {
    const slash = unc.indexOf("/");
    [server, rest] = slash < 0 ? [unc, ""] : [unc.slice(0, slash), unc.slice(slash + 1)];
  }
  return `file://${encoded(server)}${rest.startsWith("/") ? "" : "/"}${encoded(rest)}`;
}

export function iriOf(path) {
  return fileIri(at(path, () => realpathSync.native(path)));
}

// A directory read into a map, its symbolic links skipped, so the map holds only files inside it.
export class Folder {
  constructor(directory) {
    this.path = at(directory, () => realpathSync.native(directory));
    this.iri = `${fileIri(this.path)}/`;
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
      if (decoded === "." || decoded === ".." || /[/\\\0]/.test(decoded)) {
        throw unread("not inside the folder");
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
          const key = fileIri(path).slice(this.iri.length);
          this.files.set(key, new Uint8Array(at(path, () => readFileSync(path))));
        }
      }
    }
    return this;
  }
}

// What `call` answers once the maps hold every file it asks for, each file a map lacks
// read from its folder; a file the folder lacks too ends the call.
export function complete(inputs, call) {
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
