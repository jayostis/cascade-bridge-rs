// The bench on the Node host: converts each document with a built package, one fresh
// process per sample, and writes what it measured as the result file of
// cascade-bridge-spec's bench/result.schema.json.
import { execFileSync, spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync, statSync, writeFileSync } from "node:fs";
import os from "node:os";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { gunzipSync } from "node:zlib";
import { at, iriOf, Refusal } from "./host.mjs";

const USAGE = `usage: node hosts/node/bench.mjs --inputs <bench/manifest.json> [--only <glob>] [--pull-requests] [options]
       node hosts/node/bench.mjs <adapter-dir> <document>... [--envelope <iri>] [options]
options: [--vocabularies <directory>] [--samples <n>] [--timeout <seconds>] [--package <directory>]
         [--executor compiled|spareval|both] [--out <file.json>] [--baseline <directory> [--baseline-out <file.json>]]`;

const HERE = dirname(fileURLToPath(import.meta.url));
const SAMPLE = join(HERE, "bench-sample.mjs");
const SECONDS = 6;

function parse(argv) {
  const parsed = { documents: [], samples: 5, package: join(HERE, "../../package/dist") };
  const rest = [...argv];
  const next = () => {
    if (rest.length === 0) throw new Refusal(USAGE);
    return rest.shift();
  };
  const number = (flag, text, whole) => {
    const value = Number(text);
    if (!(value > 0) || (whole && !Number.isInteger(value)))
      throw new Refusal(`${flag} ${text}: not a ${whole ? "whole " : ""}number above 0`);
    return value;
  };
  while (rest.length > 0) {
    const argument = rest.shift();
    if (argument === "--vocabularies") parsed.vocabularies = next();
    else if (argument === "--envelope") parsed.envelope = next();
    else if (argument === "--samples") parsed.samples = number(argument, next(), true);
    else if (argument === "--timeout") parsed.timeout = number(argument, next(), false);
    else if (argument === "--package") parsed.package = next();
    else if (argument === "--executor") {
      parsed.executor = next();
      if (!["compiled", "spareval", "both"].includes(parsed.executor)) throw new Refusal(USAGE);
    } else if (argument === "--out") parsed.out = next();
    else if (argument === "--baseline") parsed.baseline = next();
    else if (argument === "--baseline-out") parsed.baselineOut = next();
    else if (argument === "--inputs") parsed.manifest = next();
    else if (argument === "--only") parsed.only = next();
    else if (argument === "--pull-requests") parsed.pullRequests = true;
    else if (argument.startsWith("--")) throw new Refusal(USAGE);
    else if (parsed.adapter === undefined) parsed.adapter = argument;
    else parsed.documents.push(argument);
  }
  const fromManifest = parsed.manifest !== undefined;
  if (fromManifest === (parsed.adapter !== undefined)) throw new Refusal(USAGE);
  if (!fromManifest && (parsed.documents.length === 0 || parsed.only !== undefined || parsed.pullRequests)) {
    throw new Refusal(USAGE);
  }
  if (fromManifest && parsed.envelope !== undefined) throw new Refusal(USAGE);
  if (parsed.baselineOut !== undefined && parsed.baseline === undefined) throw new Refusal(USAGE);
  return parsed;
}

function digested(path) {
  const read = at(path, () => readFileSync(path));
  const bytes = path.endsWith(".gz") ? gunzipSync(read) : read;
  return { sha256: createHash("sha256").update(bytes).digest("hex"), bytes: bytes.byteLength };
}

function globbed(glob) {
  const pattern = glob
    .replace(/[.+^${}()|[\]\\]/g, "\\$&")
    .replaceAll("*", ".*")
    .replaceAll("?", ".");
  return new RegExp(`^${pattern}$`);
}

// The adapter's directory is the manifest's grandparent.
function listed(parsed) {
  const root = dirname(dirname(resolve(parsed.manifest)));
  const manifest = JSON.parse(at(parsed.manifest, () => readFileSync(parsed.manifest, "utf8")));
  const only = parsed.only === undefined ? undefined : globbed(parsed.only);
  const inputs = manifest.inputs
    .filter((input) => only === undefined || only.test(input.id))
    .filter((input) => !parsed.pullRequests || input.onPullRequests)
    .map((input) => {
      const path = join(root, ...input.path.split("/"));
      const found = digested(path);
      if (found.sha256 !== input.sha256 || found.bytes !== input.bytes) {
        throw new Refusal(
          `${path}: ${found.bytes} bytes of sha256 ${found.sha256}, where ${parsed.manifest} names ${input.bytes} bytes of sha256 ${input.sha256}`,
        );
      }
      return {
        path,
        id: input.id,
        ...found,
        envelope: input.envelope,
        measures: input.measures,
        timeout: parsed.timeout ?? input.timeoutSeconds,
      };
    });
  if (inputs.length === 0) throw new Refusal(`${parsed.manifest}: no input is chosen`);
  return { adapter: root, inputs };
}

function segment(text) {
  const made = text
    .toLowerCase()
    .replace(/[^a-z0-9._-]/g, "-")
    .replace(/^[^a-z0-9]+/, "");
  if (made === "") throw new Refusal(`${text}: names no segment of a result's name`);
  return made;
}

function inputOf(path, parsed) {
  const name = basename(path).replace(/\.gz$/, "");
  return {
    path,
    id: segment(name.includes(".") ? name.slice(0, name.lastIndexOf(".")) : name),
    ...digested(path),
    envelope: parsed.envelope,
    timeout: parsed.timeout ?? 300,
  };
}

function packageOf(directory) {
  const manifest = JSON.parse(at(directory, () => readFileSync(join(directory, "package.json"), "utf8")));
  at(directory, () => statSync(join(directory, "node.js")));
  return { directory: resolve(directory), manifest, built: manifest.cascadeBridge ?? {} };
}

function git(directory, ...args) {
  try {
    return execFileSync("git", ["-C", directory, ...args], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
  } catch {
    return undefined;
  }
}

function checkoutCommit(directory) {
  const top = git(directory, "rev-parse", "--show-toplevel");
  const head = git(directory, "rev-parse", "HEAD");
  if (top === undefined || head === undefined) return {};
  const dirty = (git(directory, "status", "--porcelain") ?? "") !== "";
  return { [basename(top)]: `${head}${dirty ? "-dirty" : ""}` };
}

function osName() {
  if (process.platform === "win32") return `${os.version()} ${os.release()}`;
  if (process.platform === "linux") {
    try {
      const pretty = /^PRETTY_NAME="?(.*?)"?$/m.exec(readFileSync("/etc/os-release", "utf8"));
      if (pretty) return pretty[1];
    } catch {}
  }
  return `${os.type()} ${os.release()}`;
}

function environment(side) {
  return {
    os: osName(),
    runner: process.env.RUNNER_ENVIRONMENT ?? "local",
    cpus: os.availableParallelism(),
    node: process.versions.node,
    host: "node",
    profile: side.built.profile ?? null,
  };
}

// A sample's measures, or how it ended without them: refused, or killed at the timeout
// in the phase it last said it started.
function sampled(side, input, parsed) {
  const given = {
    package: side.directory,
    adapter: parsed.adapter,
    vocabularies: parsed.vocabularies,
    document: input.path,
    envelope: input.envelope,
    executor: parsed.executor,
  };
  return new Promise((done, failed) => {
    const child = spawn(process.execPath, [SAMPLE, JSON.stringify(given)], { stdio: ["ignore", "pipe", "pipe"] });
    let out = "";
    let err = "";
    let timedOut = false;
    child.stdout.on("data", (chunk) => (out += chunk));
    child.stderr.on("data", (chunk) => (err += chunk));
    const timer = setTimeout(() => {
      timedOut = true;
      child.kill();
    }, input.timeout * 1000);
    child.on("error", failed);
    child.on("close", (code) => {
      clearTimeout(timer);
      const said = out
        .split("\n")
        .filter((line) => line.startsWith("{"))
        .map((line) => JSON.parse(line));
      const last = said.at(-1);
      if (timedOut) done({ timeout: said.findLast((line) => line.phase !== undefined)?.phase ?? "load" });
      else if (last?.sample !== undefined || last?.refused !== undefined) done(last);
      else failed(new Error(`a sample of ${input.path} on ${side.directory} exited ${code}: ${err.trim()}`));
    });
  });
}

function median(values) {
  const sorted = [...values].sort((a, b) => a - b);
  const middle = Math.floor(sorted.length / 2);
  return sorted.length % 2 === 1 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2;
}

function decimal(value, digits) {
  return Number(value.toFixed(digits));
}

function written(value, digits) {
  return digits === 0 ? String(Math.round(value)) : String(decimal(value, digits));
}

function spread(name, unit, kind, values, extra) {
  const digits = unit === "s" ? SECONDS : 0;
  const entry = {
    name,
    unit,
    value: decimal(median(values), digits),
    kind,
    samples: values.length,
    range: `${written(Math.min(...values), digits)}-${written(Math.max(...values), digits)}`,
  };
  return extra === undefined ? entry : { ...entry, extra };
}

function one(name, unit, value, extra) {
  const entry = { name, unit, value, kind: "output" };
  return extra === undefined ? entry : { ...entry, extra };
}

// Each query by its file name, or by its path where two share one.
function queryNames(queries, adapterIri) {
  const paths = queries.map(({ iri }) => (iri.startsWith(adapterIri) ? iri.slice(adapterIri.length) : iri));
  const names = paths.map((path) => segment(path.slice(path.lastIndexOf("/") + 1)));
  return new Map(
    queries.map(({ iri }, index) => [
      iri,
      names.filter((name) => name === names[index]).length > 1
        ? segment(paths[index].replaceAll("/", "."))
        : names[index],
    ]),
  );
}

function inputResults(input, outcomes, adapterIri) {
  const failed = outcomes.find((outcome) => outcome.sample === undefined);
  if (failed?.timeout !== undefined) return [one(`${input.id}/timeout/${failed.timeout}`, "s", failed.seconds)];
  if (failed !== undefined)
    return [{ ...one(`${input.id}/refused/${failed.refused}`, "{count}", 1), extra: failed.message }];
  const samples = outcomes.map(({ sample }) => sample);
  const results = [];
  const variants = ["first", "second"];
  variants.forEach((variant, index) => {
    const converts = samples.map(({ converts }) => converts[index]);
    results.push(
      spread(
        `${input.id}/convert/${variant}`,
        "s",
        "clock",
        converts.map(({ seconds }) => seconds),
        index === 0 ? `sha256 ${input.sha256}, ${input.bytes} bytes` : undefined,
      ),
    );
  });
  if (samples[0].converts[0].grows !== undefined) {
    variants.forEach((variant, index) => {
      const grows = samples.map(({ converts }) => converts[index].grows);
      results.push(spread(`${input.id}/memory.grow/${variant}`, "{count}", "cost", grows));
    });
  }
  variants.forEach((variant, index) => {
    const held = samples.map(({ converts }) => converts[index].memory);
    results.push(spread(`${input.id}/wasm-memory/${variant}`, "By", "cost", held));
  });
  results.push(
    spread(
      `${input.id}/rss`,
      "By",
      "clock",
      samples.map(({ rss }) => rss),
    ),
  );
  const first = samples[0].converts[0];
  results.push(
    one(`${input.id}/records`, "{count}", first.records),
    one(`${input.id}/triples`, "{count}", first.triples),
    one(`${input.id}/findings`, "{count}", first.findings),
    one(
      `${input.id}/index-built`,
      "{count}",
      first.stages?.some(({ stage }) => stage === "index") ? 1 : 0,
      "only the compiled executor builds an index",
    ),
    one(`${input.id}/output-bytes/graph`, "By", first.graphBytes, "turtle"),
    one(`${input.id}/output-bytes/findings`, "By", first.findingsBytes, "turtle"),
  );
  if (first.queries !== undefined) {
    const names = queryNames(first.queries, adapterIri);
    for (const query of first.queries) {
      const seconds = samples.map(
        ({ converts }) => converts[0].queries.find(({ iri }) => iri === query.iri)?.seconds ?? 0,
      );
      results.push(spread(`${input.id}/query-time/${names.get(query.iri)}`, "s", "clock", seconds));
    }
    for (const query of first.queries) {
      results.push(
        one(`${input.id}/query-rows/${names.get(query.iri)}`, "{count}", query.rows, `${query.executions} executions`),
      );
    }
  }
  return input.measures === undefined
    ? results
    : results.filter(({ name }) => input.measures.includes(name.split("/")[1]));
}

function loadResults(outcomes) {
  const loads = outcomes.filter(({ sample }) => sample !== undefined).map(({ sample }) => sample.load);
  if (loads.length === 0) return [];
  const adapter = segment(loads[0].identifier ?? "adapter");
  return [
    spread(
      `${adapter}/load`,
      "s",
      "clock",
      loads.map(({ seconds }) => seconds),
    ),
    spread(
      `${adapter}/load/files-read`,
      "{count}",
      "cost",
      loads.map(({ filesRead }) => filesRead),
    ),
    spread(
      `${adapter}/load/missing-files`,
      "{count}",
      "cost",
      loads.map(({ missingFiles }) => missingFiles),
    ),
  ];
}

function resultFile(side, inputs, adapterIri, commits) {
  const all = inputs.flatMap((input) => side.outcomes.get(input.id));
  const engine =
    side.built.commit === undefined
      ? {}
      : { [side.manifest.name]: `${side.built.commit}${side.built.dirty ? "-dirty" : ""}` };
  return {
    env: environment(side),
    commit: { ...commits, ...engine },
    results: [
      ...inputs.flatMap((input) => inputResults(input, side.outcomes.get(input.id), adapterIri)),
      ...loadResults(all),
    ],
  };
}

function shown(entry) {
  if (entry === undefined) return "";
  const value = entry.unit === "s" ? `${decimal(entry.value, 3)} s` : entry.value.toLocaleString("en");
  if (entry.range === undefined) return value;
  const [low, high] = entry.range.split("-").map(Number);
  return entry.unit === "s"
    ? `${value} (${decimal(low, 3)}–${decimal(high, 3)})`
    : low === high
      ? value
      : `${value} (${low.toLocaleString("en")}–${high.toLocaleString("en")})`;
}

function change(before, after) {
  if (before === undefined || after === undefined || before.unit !== after.unit) return "";
  if (after.kind === "clock") return before.value === 0 ? "" : `${(after.value / before.value).toFixed(2)}×`;
  const difference = after.value - before.value;
  if (difference === 0) return "=";
  const percent = before.value === 0 ? "" : ` (${((difference / before.value) * 100).toFixed(1)}%)`;
  return `${difference > 0 ? "+" : "−"}${Math.abs(difference).toLocaleString("en")}${percent}`;
}

function table(inputs, files, sides) {
  const [head] = files.slice(-1);
  const base = files.length > 1 ? files[0] : undefined;
  const names = [...new Set(files.flatMap((file) => file.results.map(({ name }) => name)))];
  const find = (file, name) => file?.results.find((entry) => entry.name === name);
  let said = `Measured on ${Object.entries(head.commit)
    .map(([repository, sha]) => `${repository} ${sha}`)
    .join(", ")}`;
  if (base !== undefined)
    said += `, against ${sides[0].manifest.name} ${base.commit[sides[0].manifest.name] ?? sides[0].directory}`;
  said += `; ${head.env.os}, ${head.env.cpus} CPUs, wasm (${head.env.profile}) under Node ${head.env.node}.\n`;
  for (const input of inputs)
    said += `Input ${input.id}: ${input.path}, sha256 ${input.sha256}, ${input.bytes.toLocaleString("en")} bytes.\n`;
  said += "\n";
  if (base === undefined) {
    said += "| name | kind | this |\n|---|---|---|\n";
    for (const name of names) said += `| ${name} | ${find(head, name)?.kind ?? ""} | ${shown(find(head, name))} |\n`;
  } else {
    said += "| name | kind | baseline | this | change |\n|---|---|---|---|---|\n";
    for (const name of names) {
      const [before, after] = [find(base, name), find(head, name)];
      said += `| ${name} | ${(after ?? before).kind} | ${shown(before)} | ${shown(after)} | ${change(before, after)} |\n`;
    }
  }
  return said;
}

async function run(parsed) {
  let inputs;
  if (parsed.manifest === undefined) inputs = parsed.documents.map((path) => inputOf(path, parsed));
  else ({ adapter: parsed.adapter, inputs } = listed(parsed));
  at(parsed.adapter, () => statSync(join(parsed.adapter, "ro-crate-metadata.json")));
  if (parsed.vocabularies !== undefined) at(parsed.vocabularies, () => statSync(parsed.vocabularies));
  const ids = inputs.map(({ id }) => id);
  const twice = ids.find((id, index) => ids.indexOf(id) !== index);
  if (twice !== undefined) throw new Refusal(`two documents are both named ${twice}; rename one`);
  const sides = [...(parsed.baseline === undefined ? [] : [parsed.baseline]), parsed.package].map((directory) => ({
    ...packageOf(directory),
    outcomes: new Map(ids.map((id) => [id, []])),
  }));
  for (const input of inputs) {
    for (let n = 1; n <= parsed.samples; n++) {
      for (const side of sides) {
        const outcomes = side.outcomes.get(input.id);
        if (outcomes.some(({ sample }) => sample === undefined)) continue;
        const outcome = await sampled(side, input, parsed);
        if (outcome.timeout !== undefined) outcome.seconds = input.timeout;
        outcomes.push(outcome);
        const ended =
          outcome.sample !== undefined
            ? `${outcome.sample.converts.map(({ seconds }) => `${seconds.toFixed(2)} s`).join(", ")}`
            : (outcome.refused ?? `timed out in ${outcome.timeout}`);
        process.stderr.write(
          `${input.id} sample ${n}/${parsed.samples} ${side.manifest.name} ${side.built.commit?.slice(0, 7) ?? side.directory}: ${ended}\n`,
        );
      }
    }
  }
  const adapterIri = `${iriOf(parsed.adapter)}/`;
  const commits = {
    ...checkoutCommit(parsed.adapter),
    ...(parsed.vocabularies === undefined ? {} : checkoutCommit(parsed.vocabularies)),
  };
  const files = sides.map((side) => resultFile(side, inputs, adapterIri, commits));
  const writes = [
    [parsed.baselineOut, parsed.baseline === undefined ? undefined : files[0]],
    [parsed.out, files.at(-1)],
  ];
  for (const [path, file] of writes) {
    if (path !== undefined && file !== undefined)
      at(path, () => writeFileSync(path, `${JSON.stringify(file, null, 2)}\n`));
  }
  process.stdout.write(table(inputs, files, sides));
  return 0;
}

try {
  process.exitCode = await run(parse(process.argv.slice(2)));
} catch (error) {
  process.stderr.write(`bench: ${error instanceof Error ? error.message : String(error)}\n`);
  process.exitCode = 2;
}
