// The bench on the Node host: converts each document with a built package, one fresh
// process per sample, and writes what it measured as the result file of
// cascade-bridge-spec's bench/result.schema.json.
import { execFileSync, spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import os from "node:os";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { gunzipSync } from "node:zlib";
import { at, iriOf, Refusal } from "./host.mjs";

const USAGE = `usage: node hosts/node/bench.mjs --inputs <bench/manifest.json> [--only <glob>[,<glob>...]] [--pull-requests] [options]
       node hosts/node/bench.mjs <adapter-dir> <document>... [--envelope <iri>] [options]
options: [--vocabularies <directory>] [--samples <n>] [--timeout <seconds>] [--package <directory>]
         [--executor compiled|spareval|both] [--out <file.json>] [--profile-samples <n>] [--other-load <text>] [--glossary <file.json>] [--digest-only]
         [--baseline <package directory>] [--baseline-adapter <adapter directory>] [--baseline-out <file.json>]
         [--allow-fallbacks]`;

const HERE = dirname(fileURLToPath(import.meta.url));
const SAMPLE = join(HERE, "bench-sample.mjs");
const DIGEST = join(HERE, "bench-digest.mjs");
const SECONDS = 6;
// The most the module accepts: the timeout, not the budget, is what stops a convert.
const MEMORY_BUDGET_MIB = 3072;

function parse(argv) {
  const parsed = { documents: [], samples: 5, profileSamples: 1, package: join(HERE, "../../package/dist") };
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
    else if (argument === "--profile-samples") parsed.profileSamples = number(argument, next(), true);
    else if (argument === "--other-load") parsed.otherLoad = next();
    else if (argument === "--glossary") parsed.glossary = next();
    else if (argument === "--digest-only") parsed.digestOnly = true;
    else if (argument === "--baseline") parsed.baseline = next();
    else if (argument === "--baseline-adapter") parsed.baselineAdapter = next();
    else if (argument === "--baseline-out") parsed.baselineOut = next();
    else if (argument === "--inputs") parsed.manifest = next();
    else if (argument === "--only") parsed.only = next();
    else if (argument === "--pull-requests") parsed.pullRequests = true;
    else if (argument === "--allow-fallbacks") parsed.allowFallbacks = true;
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
  if (parsed.baselineOut !== undefined && parsed.baseline === undefined && parsed.baselineAdapter === undefined)
    throw new Refusal(USAGE);
  return parsed;
}

function digested(path) {
  const read = at(path, () => readFileSync(path));
  const bytes = path.endsWith(".gz") ? gunzipSync(read) : read;
  return { sha256: createHash("sha256").update(bytes).digest("hex"), bytes: bytes.byteLength };
}

// A comma-separated list of globs, as one expression.
function globbed(globs) {
  const patterns = globs.split(",").map((glob) =>
    glob
      .replace(/[.+^${}()|[\]\\]/g, "\\$&")
      .replaceAll("*", ".*")
      .replaceAll("?", "."),
  );
  return new RegExp(`^(?:${patterns.join("|")})$`);
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

function environment(side, parsed) {
  const env = {
    os: osName(),
    runner: process.env.RUNNER_ENVIRONMENT ?? "local",
    cpus: os.availableParallelism(),
    node: process.versions.node,
    host: "node",
    profile: side.built.profile ?? null,
  };
  const cpu = os.cpus()[0]?.model.trim();
  if (cpu) env.cpu = cpu;
  if (parsed.otherLoad !== undefined) env.otherLoad = parsed.otherLoad;
  return env;
}

// A sample's measures, or how it ended without them: refused, or killed at the timeout
// in the phase it last said it started. A profile sample's first convert is profiled, and
// where it has an output directory its second writes the outputs there.
function sampled(side, input, parsed, mode, outputDirectory) {
  const given = {
    mode,
    outputDirectory,
    package: side.directory,
    adapter: side.adapter,
    vocabularies: parsed.vocabularies,
    document: input.path,
    envelope: input.envelope,
    executor: parsed.executor,
    memoryBudgetMiB: MEMORY_BUDGET_MIB,
    timeBudgetSeconds: input.timeout,
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

// The canonical digest of one output file, in a process of its own so that its memory is nobody's measure.
function digestOf(path) {
  return new Promise((done, failed) => {
    const child = spawn(process.execPath, ["--max-old-space-size=12288", DIGEST, path], {
      stdio: ["ignore", "pipe", "pipe"],
    });
    let out = "";
    let err = "";
    child.stdout.on("data", (chunk) => (out += chunk));
    child.stderr.on("data", (chunk) => (err += chunk));
    child.on("error", failed);
    child.on("close", (code) =>
      code === 0 ? done(JSON.parse(out)) : failed(new Error(`the digest of ${path} exited ${code}: ${err.trim()}`)),
    );
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
  const digits = unit === "s" ? SECONDS : unit === "{count}/s" ? 3 : 0;
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

// Each execution for a record that the general evaluator answered: every one of a query the
// compiler did not take, and each one a compiled plan gave up on. The detect query runs once.
function fallbacksOf(queries) {
  return (queries ?? [])
    .filter(({ stage }) => stage !== "detect")
    .reduce((count, { fallback, executions, giveUps }) => count + (fallback === undefined ? giveUps : executions), 0);
}

function kebab(text) {
  return text.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`);
}

// What a row's name means, from the glossary: its name without the input or adapter in front, or that with
// its last segment as `*`; a stage by its own name. A name the glossary lacks refuses the run.
function meaning(name, glossary) {
  const tail = name.split("/").slice(1);
  const [head, ...rest] = tail;
  const found =
    head === "stage" && rest.length === 1
      ? glossary.stages[rest[0]]
      : (glossary.rows[tail.join("/")] ?? (rest.length > 0 ? glossary.rows[`${head}/*`] : undefined));
  if (found === undefined) throw new Refusal(`${name}: the glossary has no sentence for it`);
  return found;
}

// Each row's `extra`: its meaning, the input's description where it has one, and what the row said of itself.
function explained(results, glossary, inputs) {
  if (glossary === undefined) return results;
  const described = new Map(inputs.map(({ id }) => [id, glossary.inputs[id]]));
  return results.map((entry) => {
    const [first, ...tail] = entry.name.split("/");
    const input = ["convert/first", "records"].includes(tail.join("/")) ? described.get(first) : undefined;
    const extra = [meaning(entry.name, glossary), input, entry.extra].filter((part) => part !== undefined).join(" ");
    return { ...entry, extra };
  });
}

// Always written, whatever the manifest's `measures` say: an input with no digest cannot be
// checked for equivalence, and the stages are what a speed-up is looked for in.
const ALWAYS = ["output", "stage", "records-per-second"];

function digestRows(input, side) {
  const digests = side.digests.get(input.id);
  return digests === undefined
    ? []
    : ["graph", "findings"].map((kind) => ({
        ...one(`${input.id}/output/${kind}`, "{count}", digests[kind].quads),
        sha256: digests[kind].sha256,
      }));
}

function inputResults(input, side) {
  const outcomes = side.outcomes.get(input.id);
  const failed = outcomes.find((outcome) => outcome.sample === undefined);
  if (failed?.timeout !== undefined) return [one(`${input.id}/timeout/${failed.timeout}`, "s", failed.seconds)];
  if (failed !== undefined)
    return [{ ...one(`${input.id}/refused/${failed.refused}`, "{count}", 1), extra: failed.message }];
  if (side.digestOnly) return digestRows(input, side);
  const samples = outcomes.map(({ sample }) => sample);
  const profiled = side.profiles.get(input.id).flatMap(({ sample }) => (sample === undefined ? [] : [sample]));
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
  if (profiled.length > 0) {
    results.push(
      spread(
        `${input.id}/convert/profiled`,
        "s",
        "clock",
        profiled.map(({ converts }) => converts[0].seconds),
      ),
    );
  }
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
    spread(
      `${input.id}/records-per-second`,
      "{count}/s",
      "clock",
      samples.map(({ converts: [first] }) => first.records / first.seconds),
    ),
  );
  const first = samples[0].converts[0];
  const profile = profiled[0]?.converts[0];
  results.push(
    one(`${input.id}/records`, "{count}", first.records),
    one(`${input.id}/triples`, "{count}", first.triples),
    one(`${input.id}/findings`, "{count}", first.findings),
  );
  if (profile !== undefined) {
    results.push(
      one(
        `${input.id}/index-built`,
        "{count}",
        profile.stages?.some(({ stage }) => stage === "index") ? 1 : 0,
      ),
    );
  }
  results.push(
    one(`${input.id}/output-bytes/graph`, "By", first.graphBytes),
    one(`${input.id}/output-bytes/findings`, "By", first.findingsBytes),
  );
  if (profile?.queries !== undefined) {
    results.push(
      one(`${input.id}/fallbacks`, "{count}", fallbacksOf(profile.queries)),
    );
    const names = queryNames(profile.queries, side.adapterIri);
    for (const query of profile.queries) {
      const seconds = profiled.map(
        ({ converts }) => converts[0].queries.find(({ iri }) => iri === query.iri)?.seconds ?? 0,
      );
      results.push(spread(`${input.id}/query-time/${names.get(query.iri)}`, "s", "clock", seconds));
    }
    for (const query of profile.queries) {
      results.push(
        one(`${input.id}/query-rows/${names.get(query.iri)}`, "{count}", query.rows, `executions: ${query.executions}`),
      );
    }
  }
  if (profile?.stages !== undefined) {
    for (const { stage } of profile.stages) {
      const seconds = profiled.map(
        ({ converts }) => converts[0].stages.find((timed) => timed.stage === stage)?.seconds ?? 0,
      );
      results.push(spread(`${input.id}/stage/${kebab(stage)}`, "s", "clock", seconds));
    }
    const unaccounted = profiled.map(({ converts: [convert] }) =>
      Math.max(0, convert.seconds - convert.stages.reduce((sum, { seconds }) => sum + seconds, 0)),
    );
    results.push(spread(`${input.id}/stage/unaccounted`, "s", "clock", unaccounted));
  }
  results.push(...digestRows(input, side));
  return input.measures === undefined
    ? results
    : results.filter(({ name }) => {
        const measure = name.split("/")[1];
        return ALWAYS.includes(measure) || input.measures.includes(measure);
      });
}

function loadResults(side, inputs) {
  const loads = inputs
    .flatMap(({ id }) => side.outcomes.get(id))
    .filter(({ sample }) => sample !== undefined)
    .map(({ sample }) => sample);
  if (loads.length === 0) return [];
  const adapter = segment(loads[0].load.identifier ?? "adapter");
  const each = (pick) => loads.map(pick);
  return [
    spread(`${adapter}/load`, "s", "clock", each(({ load }) => load.seconds)),
    spread(`${adapter}/load/read-files`, "s", "clock", each(({ load }) => load.readFilesSeconds)),
    spread(`${adapter}/load/prepare`, "s", "clock", each(({ load }) => load.prepareSeconds)),
    spread(`${adapter}/load/files-read`, "{count}", "cost", each(({ load }) => load.filesRead)),
    spread(`${adapter}/load/missing-files`, "{count}", "cost", each(({ load }) => load.missingFiles)),
    spread(`${adapter}/boot/read-wasm`, "s", "clock", each(({ boot }) => boot.readWasmSeconds)),
    spread(`${adapter}/boot/compile-wasm`, "s", "clock", each(({ boot }) => boot.compileSeconds)),
    spread(`${adapter}/boot/instantiate-wasm`, "s", "clock", each(({ boot }) => boot.instantiateSeconds)),
  ];
}

function resultFile(side, inputs, commits, parsed, glossary) {
  const engine =
    side.built.commit === undefined
      ? {}
      : { [side.manifest.name]: `${side.built.commit}${side.built.dirty ? "-dirty" : ""}` };
  return {
    env: environment(side, parsed),
    commit: { ...checkoutCommit(side.adapter), ...commits, ...engine },
    results: explained(
      [...inputs.flatMap((input) => inputResults(input, side)), ...(side.digestOnly ? [] : loadResults(side, inputs))],
      glossary,
      inputs,
    ),
  };
}

function shown(entry) {
  if (entry === undefined) return "";
  if (entry.sha256 !== undefined) return `${entry.value.toLocaleString("en")} (sha256 ${entry.sha256.slice(0, 12)})`;
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
  if (before.sha256 !== undefined || after.sha256 !== undefined)
    return before.sha256 === after.sha256 ? "same output" : "NOT EQUIVALENT";
  if (after.kind === "clock") return before.value === 0 ? "" : `${(after.value / before.value).toFixed(2)}×`;
  const difference = after.value - before.value;
  if (difference === 0) return "=";
  const percent = before.value === 0 ? "" : ` (${((difference / before.value) * 100).toFixed(1)}%)`;
  return `${difference > 0 ? "+" : "−"}${Math.abs(difference).toLocaleString("en")}${percent}`;
}

function table(inputs, files) {
  const [head] = files.slice(-1);
  const base = files.length > 1 ? files[0] : undefined;
  const names = [...new Set(files.flatMap((file) => file.results.map(({ name }) => name)))];
  const find = (file, name) => file?.results.find((entry) => entry.name === name);
  let said = `Measured on ${Object.entries(head.commit)
    .map(([repository, sha]) => `${repository} ${sha}`)
    .join(", ")}`;
  if (base !== undefined)
    said += `, against ${Object.entries(base.commit)
      .filter(([repository, sha]) => head.commit[repository] !== sha)
      .map(([repository, sha]) => `${repository} ${sha}`)
      .join(", ")}`;
  said += `; ${head.env.os}, ${head.env.cpu ?? "unknown CPU"}, ${head.env.cpus} CPUs, wasm (${head.env.profile}) under Node ${head.env.node}.\n`;
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

function failure(outcome) {
  return outcome.sample !== undefined ? undefined : (outcome.refused ?? `timed out in ${outcome.timeout}`);
}

// The digests of the outputs a sample wrote to `directory`, which are then removed.
async function digestOutputs(side, input, directory) {
  const [graph, findings] = await Promise.all(["graph", "findings"].map((kind) => digestOf(join(directory, `${kind}.nt`))));
  side.digests.set(input.id, { graph, findings });
  process.stderr.write(`${input.id} output ${graph.quads} + ${findings.quads} quads, graph sha256 ${graph.sha256}
`);
  rmSync(join(directory, "graph.nt"), { force: true });
  rmSync(join(directory, "findings.nt"), { force: true });
}

async function run(parsed) {
  let inputs;
  if (parsed.manifest === undefined) inputs = parsed.documents.map((path) => inputOf(path, parsed));
  else ({ adapter: parsed.adapter, inputs } = listed(parsed));
  for (const adapter of [parsed.adapter, parsed.baselineAdapter]) {
    if (adapter !== undefined) at(adapter, () => statSync(join(adapter, "ro-crate-metadata.json")));
  }
  if (parsed.vocabularies !== undefined) at(parsed.vocabularies, () => statSync(parsed.vocabularies));
  const ids = inputs.map(({ id }) => id);
  const twice = ids.find((id, index) => ids.indexOf(id) !== index);
  if (twice !== undefined) throw new Refusal(`two documents are both named ${twice}; rename one`);
  const baselined = parsed.baseline !== undefined || parsed.baselineAdapter !== undefined;
  const made = (directory, adapter) => ({
    ...packageOf(directory),
    adapter,
    adapterIri: `${iriOf(adapter)}/`,
    outcomes: new Map(ids.map((id) => [id, []])),
    profiles: new Map(ids.map((id) => [id, []])),
    digests: new Map(),
    digestOnly: parsed.digestOnly === true,
  });
  const sides = [
    ...(baselined ? [made(parsed.baseline ?? parsed.package, parsed.baselineAdapter ?? parsed.adapter)] : []),
    made(parsed.package, parsed.adapter),
  ];
  const scratch = mkdtempSync(join(os.tmpdir(), "bench-"));
  try {
    for (const input of inputs) {
      if (parsed.digestOnly) {
        for (const side of sides) {
          const directory = join(scratch, `${sides.indexOf(side)}-${input.id}`);
          mkdirSync(directory);
          const outcome = await sampled(side, input, parsed, "digest", directory);
          if (outcome.timeout !== undefined) outcome.seconds = input.timeout;
          side.outcomes.get(input.id).push(outcome);
          const ended = failure(outcome) ?? `${outcome.sample.converts[0].seconds.toFixed(2)} s`;
          process.stderr.write(`${input.id} digest ${side.manifest.name}: ${ended}
`);
          if (outcome.sample !== undefined) await digestOutputs(side, input, directory);
        }
        continue;
      }
      for (let n = 1; n <= parsed.samples; n++) {
        for (const side of sides) {
          const outcomes = side.outcomes.get(input.id);
          if (outcomes.some(({ sample }) => sample === undefined)) continue;
          const outcome = await sampled(side, input, parsed, "timing");
          if (outcome.timeout !== undefined) outcome.seconds = input.timeout;
          outcomes.push(outcome);
          const ended =
            outcome.sample !== undefined
              ? `${outcome.sample.converts.map(({ seconds }) => `${seconds.toFixed(2)} s`).join(", ")}`
              : failure(outcome);
          process.stderr.write(
            `${input.id} sample ${n}/${parsed.samples} ${side.manifest.name} ${side.built.commit?.slice(0, 7) ?? side.directory}: ${ended}\n`,
          );
        }
      }
      for (const side of sides) {
        if (side.outcomes.get(input.id).some(({ sample }) => sample === undefined)) continue;
        const directory = join(scratch, `${sides.indexOf(side)}-${input.id}`);
        mkdirSync(directory);
        for (let n = 1; n <= parsed.profileSamples; n++) {
          const outcome = await sampled(side, input, parsed, "profile", n === 1 ? directory : undefined);
          side.profiles.get(input.id).push(outcome);
          const ended = failure(outcome);
          process.stderr.write(
            `${input.id} profile ${n}/${parsed.profileSamples} ${side.manifest.name}: ${
              ended ?? `${outcome.sample.converts[0].seconds.toFixed(2)} s profiled`
            }\n`,
          );
          if (n === 1 && ended === undefined) await digestOutputs(side, input, directory);
        }
      }
    }
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
  const commits = parsed.vocabularies === undefined ? {} : checkoutCommit(parsed.vocabularies);
  const glossary =
    parsed.glossary === undefined ? undefined : JSON.parse(at(parsed.glossary, () => readFileSync(parsed.glossary, "utf8")));
  if (glossary !== undefined && parsed.manifest !== undefined) {
    for (const { id } of inputs) {
      if (glossary.inputs[id] === undefined) process.stderr.write(`bench: ${parsed.glossary} has no description of the input ${id}
`);
    }
  }
  const files = sides.map((side) => resultFile(side, inputs, commits, parsed, glossary));
  const writes = [
    [parsed.baselineOut, baselined ? files[0] : undefined],
    [parsed.out, files.at(-1)],
  ];
  for (const [path, file] of writes) {
    if (path !== undefined && file !== undefined)
      at(path, () => writeFileSync(path, `${JSON.stringify(file, null, 2)}\n`));
  }
  process.stdout.write(table(inputs, files));
  const head = sides.at(-1);
  const fellBack =
    parsed.executor === "spareval" || parsed.allowFallbacks
      ? []
      : inputs
          .map(({ id }) => [id, fallbacksOf(head.profiles.get(id).find(({ sample }) => sample)?.sample.converts[0].queries)])
          .filter(([, count]) => count > 0);
  for (const [id, count] of fellBack)
    process.stderr.write(`bench: ${id}/fallbacks: ${count} record executions fell back to the general evaluator (--allow-fallbacks to measure anyway)\n`);
  return fellBack.length === 0 ? 0 : 1;
}

try {
  process.exitCode = await run(parse(process.argv.slice(2)));
} catch (error) {
  process.stderr.write(`bench: ${error instanceof Error ? error.message : String(error)}\n`);
  process.exitCode = 2;
}
