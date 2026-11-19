// The digest of one output of `bench.mjs`: the SHA-256 of the file's triples as canonical
// N-Quads (W3C RDFC-1.0, https://www.w3.org/TR/rdf-canon/), and how many there are.
// `node bench-digest.mjs <file.nt>` prints one JSON object. The file is the Bridge's N-Triples,
// one triple to a line, read here because rdf-canonize's own N-Quads reader takes time that
// grows with the square of the file.
import { createHash } from "node:crypto";
import { createReadStream } from "node:fs";
import { createInterface } from "node:readline";
import canonize from "rdf-canonize";

const ESCAPES = { t: "\t", b: "\b", n: "\n", r: "\r", f: "\f", '"': '"', "'": "'", "\\": "\\" };
const TRIPLE =
  /^(?:<([^>]*)>|_:(\S+))\s+<([^>]*)>\s+(?:<([^>]*)>|_:(\S+?)|"((?:[^"\\]|\\.)*)"(?:\^\^<([^>]*)>|@([A-Za-z0-9-]+))?)\s*\.\s*$/;
const XSD_STRING = "http://www.w3.org/2001/XMLSchema#string";
const LANG_STRING = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";

function escaped(plain) {
  if (!(plain in ESCAPES)) throw new Error(`\\${plain} is no escape of N-Triples`);
  return ESCAPES[plain];
}

function unescaped(text) {
  return text.includes("\\")
    ? text.replace(/\\(?:u([0-9A-Fa-f]{4})|U([0-9A-Fa-f]{8})|(.))/g, (_, short, long, plain) =>
        plain === undefined ? String.fromCodePoint(Number.parseInt(short ?? long, 16)) : escaped(plain),
      )
    : text;
}

function quadOf(line) {
  const found = TRIPLE.exec(line);
  if (found === null) throw new Error(`not an N-Triples line: ${line.slice(0, 200)}`);
  const [, subjectIri, subjectBlank, predicate, objectIri, objectBlank, text, datatype, language] = found;
  const term = (iri, blank) =>
    iri === undefined ? { termType: "BlankNode", value: `_:${blank}` } : { termType: "NamedNode", value: unescaped(iri) };
  const literalDatatype = language !== undefined ? LANG_STRING : datatype === undefined ? XSD_STRING : unescaped(datatype);
  const object =
    text === undefined
      ? term(objectIri, objectBlank)
      : {
          termType: "Literal",
          value: unescaped(text),
          datatype: { termType: "NamedNode", value: literalDatatype },
          ...(language === undefined ? {} : { language: language.toLowerCase() }),
        };
  return {
    subject: term(subjectIri, subjectBlank),
    predicate: { termType: "NamedNode", value: unescaped(predicate) },
    object,
    graph: { termType: "DefaultGraph", value: "" },
  };
}

const dataset = [];
for await (const line of createInterface({ input: createReadStream(process.argv[2], "utf8"), crlfDelay: Infinity })) {
  if (line.trim() !== "" && !line.startsWith("#")) dataset.push(quadOf(line));
}
const canonical =
  dataset.length === 0 ? "" : await canonize.canonize(dataset, { algorithm: "RDFC-1.0", format: "application/n-quads" });
process.stdout.write(
  `${JSON.stringify({
    quads: canonical === "" ? 0 : canonical.trimEnd().split("\n").length,
    sha256: createHash("sha256").update(canonical).digest("hex"),
  })}\n`,
);
