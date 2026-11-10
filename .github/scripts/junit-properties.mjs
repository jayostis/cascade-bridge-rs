import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import process from "node:process";
import { pathToFileURL } from "node:url";

/**
 * The report the tests page reads, made from nextest's JUnit reports: `node junit-properties.mjs <out.xml> <in.xml>...`,
 * run from the repository's root. Nextest writes a case's module path and its test binary, and no file, line or doc
 * comment, so each case is found in the source by its path: rustfmt's indentation says which `mod` a line is in, and the
 * `///` above a `mod` or a `fn` is what it says of itself, as the vocabulary's tests say it of a class or a function. The
 * reports are merged into one, a case of the files `crates/spec.json` lists as `shaped` that cannot be found is an error,
 * and any other is left as nextest wrote it.
 */

const SPEC = "crates/spec.json";
const LIBRARIES = new Set([
  "lib",
  "rlib",
  "dylib",
  "cdylib",
  "staticlib",
  "proc-macro",
]);
const ATTRIBUTE = /\s([\w:.-]+)="([^"]*)"/g;
const SUITE = /<testsuite\b[^>]*?(?:\/>|>[\s\S]*?<\/testsuite>)/g;
const CASE = /<testcase\b[^>]*?(?:\/>|>[\s\S]*?<\/testcase>)/g;
const RULE = /^@rule[ \t]+(.+)$/gm;
const NAMED = { amp: "&", lt: "<", gt: ">", quot: '"', apos: "'" };

const decoded = (text) =>
  text.replace(/&(?:#(\d+)|#x([\da-f]+)|(\w+));/gi, (whole, decimal, hex, name) =>
    name === undefined
      ? String.fromCodePoint(
          decimal === undefined ? Number.parseInt(hex, 16) : Number(decimal),
        )
      : (NAMED[name] ?? whole),
  );

const escaped = (text) =>
  text
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("\n", "&#10;");

const attributesOf = (source) =>
  new Map(
    [...source.matchAll(ATTRIBUTE)].map(([, name, value]) => [
      name,
      decoded(value),
    ]),
  );

/** Nextest's id of each test binary of a workspace, from `cargo metadata`, and the file it starts in. */
export function binariesOf(metadata) {
  const binaries = new Map();
  for (const { name: crate, targets } of metadata.packages) {
    for (const { name, kind, src_path: source } of targets) {
      const file = source.replaceAll("\\", "/");
      if (kind.some((one) => LIBRARIES.has(one))) binaries.set(crate, file);
      else if (kind.includes("test")) binaries.set(`${crate}::${name}`, file);
      else if (kind.includes("bin")) binaries.set(`${crate}::bin/${name}`, file);
    }
  }
  return binaries;
}

/** What a `mod` or a `fn` says of itself: the first paragraph of its doc is its title, the rest its description. */
export function said(doc, name) {
  const text = doc.join("\n");
  const rules = [...text.matchAll(RULE)].map((found) => (found[1] ?? "").trim());
  const paragraphs = text
    .replace(RULE, "")
    .split("\n\n")
    .map((part) => part.split(/\s+/).filter(Boolean).join(" "))
    .filter((part) => part !== "");
  return {
    title: paragraphs[0] ?? name.replaceAll("_", " "),
    description: paragraphs.slice(1).join("\n\n"),
    rules,
  };
}

/** The first line of the attribute that ends on `line`, a multi-line one included, or `line` when it ends none. */
function attributeStart(lines, line) {
  for (let up = line; up >= 0 && up > line - 20; up -= 1) {
    const text = (lines[up] ?? "").trim();
    if (text.startsWith("#[")) return up;
    if (up !== line && /[;{}]$/.test(text)) return line;
  }
  return line;
}

/**
 * The `///` lines directly above the item on `at`, without their marker; an attribute (of several lines too) and a `//` or
 * `////` comment between them and it are skipped.
 */
function docOf(lines, at) {
  const doc = [];
  for (let line = at - 1; line >= 0; line -= 1) {
    const text = (lines[line] ?? "").trim();
    if (text.startsWith("///") && !text.startsWith("////"))
      doc.unshift(text.slice(3).replace(/^ /, ""));
    else if (text.startsWith("//") || text.startsWith("#[")) continue;
    else if (text.endsWith("]")) {
      const start = attributeStart(lines, line);
      if (start === line) break;
      line = start;
    } else break;
  }
  return doc;
}

const RAW_STRING = /^b?r(#*)"/;

/**
 * Which lines of a file begin inside a string, a raw string or a block comment, where a `}` or a `mod` is text, not code.
 */
function insideText(lines) {
  const inside = new Array(lines.length).fill(false);
  let closing; // what ends the string or comment the scan is in, none in code
  let depth = 0; // block comments nest
  for (let at = 0; at < lines.length; at += 1) {
    inside[at] = closing !== undefined;
    const text = lines[at] ?? "";
    for (let column = 0; column < text.length; column += 1) {
      const rest = text.slice(column);
      if (closing === "*/") {
        if (rest.startsWith("/*")) {
          depth += 1;
          column += 1;
        } else if (rest.startsWith("*/")) {
          depth -= 1;
          column += 1;
          if (depth === 0) closing = undefined;
        }
      } else if (closing === "escaped") {
        if (rest.startsWith("\\")) column += 1;
        else if (rest.startsWith('"')) closing = undefined;
      } else if (closing !== undefined) {
        if (rest.startsWith(closing)) {
          column += closing.length - 1;
          closing = undefined;
        }
      } else if (rest.startsWith("//")) break;
      else if (rest.startsWith("/*")) {
        closing = "*/";
        depth = 1;
        column += 1;
      } else if (RAW_STRING.test(rest) && !/\w/.test(text[column - 1] ?? "")) {
        const raw = RAW_STRING.exec(rest);
        closing = `"${raw?.[1] ?? ""}`;
        column += (raw?.[0].length ?? 1) - 1;
      } else if (rest.startsWith('"')) closing = "escaped";
      else if (rest.startsWith("'")) {
        const literal = /^'(?:\\(?:u\{[^}]*\}|x[\da-fA-F]{2}|.)|[^\\'])'/.exec(rest);
        if (literal !== null) column += literal[0].length - 1;
      }
    }
  }
  return inside;
}

const textOf = new WeakMap();
const insideTextOf = (lines) => {
  if (!textOf.has(lines)) textOf.set(lines, insideText(lines));
  return textOf.get(lines) ?? [];
};

const modRule = (depth, name) =>
  new RegExp(`^ {${4 * depth}}(?:pub(?:\\([^)]*\\))? )?mod ${name}(?: \\{|;)`);
const fnRule = (depth, name) =>
  new RegExp(
    `^ {${4 * depth}}(?:pub(?:\\([^)]*\\))? )?(?:async )?(?:unsafe )?fn ${name}\\b`,
  );

/**
 * The `mod`s and the `fn` a case's path names, each with its file, its 1-based line and its doc, outermost first; none when
 * the source does not hold it. rustfmt indents a `mod`'s items by four spaces and closes it at the `mod`'s own indent, so
 * the lines between are found without counting braces, passing over lines that begin inside a string or a block comment.
 * `mod x;` goes on in `x.rs` or `x/mod.rs` beside the file, under the file's own name unless it is the binary's first.
 * `visited` is given every file the walk read, which is how a case it could not place is tied to a shaped file.
 */
export function placed(binary, path, linesOf, visited = []) {
  let file = binary;
  let directory = dirname(binary);
  let lines = linesOf(file);
  let from = 0;
  let to = lines?.length ?? 0;
  let depth = 0;
  const found = [];
  visited.push(file);
  for (const [at, name] of path.entries()) {
    if (lines === undefined || !/^\w+$/.test(name)) return undefined;
    const text = insideTextOf(lines);
    const last = at === path.length - 1;
    const rule = last ? fnRule(depth, name) : modRule(depth, name);
    let line = from;
    while (line < to && (text[line] || !rule.test(lines[line] ?? ""))) line += 1;
    if (line === to) return undefined;
    found.push({ name, file, line: line + 1, doc: docOf(lines, line) });
    if (last) return found;
    directory = join(directory, name);
    if ((lines[line] ?? "").endsWith(";")) {
      file = [`${directory}.rs`, join(directory, "mod.rs")].find((one) =>
        existsSync(one),
      );
      if (file === undefined) return undefined;
      visited.push(file);
      lines = linesOf(file);
      from = 0;
      to = lines?.length ?? 0;
      depth = 0;
    } else {
      const closing = `${" ".repeat(4 * depth)}}`;
      from = line + 1;
      to = from;
      while (to < lines.length && (text[to] || !(lines[to] ?? "").startsWith(closing))) to += 1;
      depth += 1;
    }
  }
  return undefined;
}

/** The properties of a case from what `placed` found: its own, and each level's below the `mod tests` its path may hold. */
export function propertiesOf(found, root) {
  const property = (name, value) =>
    `<property name="${name}" value="${escaped(value)}"/>`;
  const about = (one, prefix, withFile) => {
    const { title, description, rules } = said(one.doc, one.name);
    return [
      ...(withFile ? [property("file", relative(root, one.file).replaceAll("\\", "/"))] : []),
      property(`${prefix}line`, String(one.line)),
      property(`${prefix}title`, title),
      ...(description === "" ? [] : [property(`${prefix}description`, description)]),
      ...rules.map((rule) => property(`${prefix}rule`, rule)),
    ];
  };
  const leaf = found.at(-1);
  const modules = found.slice(0, -1);
  const levels = modules.slice(modules.findIndex((one) => one.name === "tests") + 1);
  return [
    ...about(leaf, "", true),
    ...levels.flatMap((one, at) => about(one, `describe:${at + 1}:`, false)),
  ];
}

/**
 * One report from several: every `testsuite` of each under a `testsuites` named `nextest-run`, which is how the page tells
 * the runner, each case of them given the properties of where it is in the source. `unplaced` are the cases of a `shaped`
 * file that the source does not hold.
 */
export function merged(reports, { binaries, root, shaped }) {
  const files = new Map();
  const linesOf = (file) => {
    if (!files.has(file))
      files.set(
        file,
        existsSync(file) ? readFileSync(file, "utf8").split(/\r?\n/) : undefined,
      );
    return files.get(file);
  };
  const unplaced = [];
  let tests = 0;
  let failures = 0;
  let errors = 0;
  let time = 0;
  const suites = [];
  for (const report of reports) {
    const rootTag = /<testsuites\b[^>]*>/.exec(report);
    const attribute = attributesOf(rootTag?.[0] ?? "");
    if (attribute.get("name") !== "nextest-run")
      throw new Error(
        `a report is not nextest's: its root is named ${attribute.get("name") ?? "nothing"}, not nextest-run`,
      );
    tests += Number(attribute.get("tests") ?? 0);
    failures += Number(attribute.get("failures") ?? 0);
    errors += Number(attribute.get("errors") ?? 0);
    time += Number(attribute.get("time") ?? 0);
    for (const suite of report.match(SUITE) ?? [])
      suites.push(
        suite.replace(CASE, (element) => {
          const [, source = "", closed = ""] =
            /^<testcase\b([^>]*?)(\/?)>/.exec(element) ?? [];
          const attributes = attributesOf(source);
          const classname = attributes.get("classname") ?? "";
          const name = attributes.get("name") ?? "";
          const binary = binaries.get(classname);
          const visited = [];
          const found =
            binary === undefined
              ? undefined
              : placed(binary, name.split("::"), linesOf, visited);
          if (found === undefined) {
            if (visited.some((file) => shaped.has(relative(root, file).replaceAll("\\", "/"))))
              unplaced.push(`${classname} ${name}`);
            return element;
          }
          const properties = `<properties>${propertiesOf(found, root).join("")}</properties>`;
          const opening = `<testcase${source}>`;
          return closed === "/"
            ? `${opening}${properties}</testcase>`
            : `${opening}${properties}${element.slice(opening.length)}`;
        }),
      );
  }
  const xml = [
    '<?xml version="1.0" encoding="UTF-8"?>',
    `<testsuites name="nextest-run" tests="${tests}" failures="${failures}" errors="${errors}" time="${time.toFixed(3)}">`,
    ...suites,
    "</testsuites>",
    "",
  ].join("\n");
  return { xml, unplaced };
}

function run([out, ...inputs]) {
  if (out === undefined || inputs.length === 0) {
    process.stderr.write("usage: node junit-properties.mjs <out.xml> <in.xml>...\n");
    return 2;
  }
  const root = process.cwd();
  const metadata = JSON.parse(
    execFileSync("cargo", ["metadata", "--no-deps", "--format-version", "1"], {
      encoding: "utf8",
      maxBuffer: 1 << 28,
    }),
  );
  const spec = existsSync(SPEC) ? JSON.parse(readFileSync(SPEC, "utf8")) : {};
  const shaped = new Set((spec.shaped ?? []).map((file) => join(dirname(SPEC), file).replaceAll("\\", "/")));
  const reports = inputs.map((input) => readFileSync(input, "utf8"));
  const { xml, unplaced } = merged(reports, {
    binaries: binariesOf(metadata),
    root,
    shaped,
  });
  writeFileSync(out, xml);
  for (const one of unplaced)
    process.stderr.write(`not found in the source, though its file is shaped: ${one}\n`);
  return unplaced.length === 0 ? 0 : 1;
}

if (process.argv[1] !== undefined && import.meta.url === pathToFileURL(process.argv[1]).href)
  process.exitCode = run(process.argv.slice(2));
