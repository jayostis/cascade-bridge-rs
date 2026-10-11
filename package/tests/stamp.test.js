import assert from "node:assert/strict";
import { test } from "node:test";
import { stamp, workspaceVersion } from "../stamp.mjs";

const built = {
  name: "cascade-bridge-rs",
  version: `${workspaceVersion()}-commit-${"a".repeat(40)}`,
  cascadeBridge: { commit: "a".repeat(40), dirty: false, sources: "sha256-aa", profile: "wasm" },
};

test("stamping a built package for another commit changes its version and commit and keeps sources, dirty and profile", () => {
  const commit = "b".repeat(40);
  const stamped = JSON.parse(stamp(JSON.stringify(built), { commit }));
  assert.equal(stamped.version, `${workspaceVersion()}-commit-${commit}`);
  assert.deepEqual(stamped.cascadeBridge, { ...built.cascadeBridge, commit });
});

test("stamping a template names every field it is given", () => {
  const stamped = JSON.parse(
    stamp(JSON.stringify({ name: "cascade-bridge-rs", version: "0.0.0" }), {
      commit: "c".repeat(40),
      dirty: true,
      sources: "sha256-cc",
      profile: "wasm-fast",
    }),
  );
  assert.deepEqual(stamped.cascadeBridge, {
    commit: "c".repeat(40),
    dirty: true,
    sources: "sha256-cc",
    profile: "wasm-fast",
  });
});

test("a stamped manifest is two-space JSON with a final newline, as build.sh's freshness check reads it", () => {
  const text = stamp(JSON.stringify(built), { commit: "d".repeat(40) });
  assert.match(text, /\n {4}"dirty": false,\n/);
  assert.match(text, /\n {4}"profile": "wasm"\n/);
  assert.ok(text.endsWith("}\n"));
});
