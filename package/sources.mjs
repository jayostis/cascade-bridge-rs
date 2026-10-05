// The sources a package was built from, as one hash: run in a checkout, it prints
// what that checkout's build records as cascadeBridge.sources.
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const SOURCES = ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "crates/", "package/"];

const checkout = fileURLToPath(new URL("..", import.meta.url));
const listed = execFileSync(
  "git",
  ["ls-files", "-z", "--cached", "--others", "--exclude-standard", "--", ...SOURCES],
  { cwd: checkout, encoding: "buffer" },
);
const paths = [...new Set(listed.toString("utf8").split("\0").filter((path) => path !== ""))]
  .sort((a, b) => Buffer.compare(Buffer.from(a), Buffer.from(b)));
const all = createHash("sha256");
for (const path of paths) {
  let bytes;
  try {
    bytes = readFileSync(new URL(path, new URL("..", import.meta.url)));
  } catch (error) {
    if (error.code === "ENOENT") continue;
    throw error;
  }
  all.update(`${path}\0${createHash("sha256").update(bytes).digest("hex")}\n`);
}
process.stdout.write(`sha256-${all.digest("hex")}\n`);
