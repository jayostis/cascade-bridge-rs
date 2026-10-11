// The one place a built package's manifest is written, for a build and for a reuse of another
// commit's build. Kept fields: a restamp names only the commit, and sources, dirty and profile stay.
//   node package/stamp.mjs version
//   node package/stamp.mjs <template> <out> <commit> [<dirty> <sources> <profile>]
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const CARGO = new URL("../Cargo.toml", import.meta.url);

export function workspaceVersion() {
  const lines = readFileSync(CARGO, "utf8").split("\n");
  const start = lines.indexOf("[workspace.package]");
  for (const line of start < 0 ? [] : lines.slice(start + 1)) {
    if (line.startsWith("[")) break;
    const version = line.match(/^version = "(.*)"$/)?.[1];
    if (version) return version;
  }
  throw new Error("Cargo.toml has no [workspace.package] version");
}

export function stamp(template, { commit, dirty, sources, profile }) {
  const manifest = JSON.parse(template);
  const kept = manifest.cascadeBridge ?? {};
  manifest.version = `${workspaceVersion()}-commit-${commit}`;
  manifest.cascadeBridge = {
    commit,
    dirty: dirty ?? kept.dirty,
    sources: sources ?? kept.sources,
    profile: profile ?? kept.profile,
  };
  return `${JSON.stringify(manifest, null, 2)}\n`;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const [first, out, commit, dirty, sources, profile] = process.argv.slice(2);
  if (first === "version") {
    process.stdout.write(`${workspaceVersion()}\n`);
  } else if (first && out && commit) {
    const stamped = stamp(readFileSync(first, "utf8"), {
      commit,
      dirty: dirty === undefined ? undefined : dirty === "true",
      sources,
      profile,
    });
    writeFileSync(out, stamped);
  } else {
    process.stderr.write("usage: node package/stamp.mjs version | <template> <out> <commit> [<dirty> <sources> <profile>]\n");
    process.exit(2);
  }
}
