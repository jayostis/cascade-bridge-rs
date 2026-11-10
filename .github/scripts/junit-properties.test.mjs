import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { before, describe, it } from "node:test";
import { binariesOf, merged, said } from "./junit-properties.mjs";

const LIBRARY = "pub mod naming;\n";

const NAMING = `pub fn naming() {}

#[cfg(test)]
mod tests;
`;

const NAMING_TESTS = `use super::*;

/// Naming a record · naming()
///
/// A record is named by its selector,
/// which may hold a pipe.
///
/// Its name is kept as it was written.
///
/// @rule naming N1
mod naming_a_record {
    use super::*;

    /// when the selector has a pipe
    mod when_it_has_a_pipe {
        use super::*;

        /// escapes it, as \`%7C\`
        #[test]
        #[ignore = "slow"]
        fn escapes_the_pipe() {
            let held = "WHERE {\\n";
        }

        #[test]
        fn keeps_the_base() {}
    }
}
`;

const COMMAND = `mod common;

/// Converting a document · convert
mod converting_a_document {
    /// leaves no file
    mod with_no_out {
        #[test]
        fn writes_to_standard_output() {}
    }
}

#[test]
fn stands_alone() {}
`;

const report = (name, suites) =>
  `<?xml version="1.0" encoding="UTF-8"?>\n<testsuites name="${name}" tests="3" failures="1" errors="0" time="0.5">\n${suites}</testsuites>\n`;
const suite = (name, cases) =>
  `    <testsuite name="${name}" tests="1" disabled="0" errors="0" failures="0">\n${cases}    </testsuite>\n`;
const passing = (classname, name) =>
  `        <testcase name="${name}" classname="${classname}" time="0.001">\n        </testcase>\n`;

/** The properties of the one case a merged report holds, as a map, or its text when it has none. */
const propertiesOf = (xml, name) => {
  const element = new RegExp(`<testcase name="${name}"[^>]*?(?:/>|>[\\s\\S]*?</testcase>)`).exec(xml)?.[0] ?? "";
  return [...element.matchAll(/<property name="([^"]+)" value="([^"]*)"\/>/g)].map(([, key, value]) => [key, value]);
};

describe("a crate on disk", () => {
  let options;

  before(() => {
    const root = mkdtempSync(join(tmpdir(), "junit-properties-"));
    for (const [file, text] of [
      ["crates/a/src/lib.rs", LIBRARY],
      ["crates/a/src/naming.rs", NAMING],
      ["crates/a/src/naming/tests.rs", NAMING_TESTS],
      ["crates/a/tests/cli.rs", COMMAND],
      ["crates/a/tests/other.rs", "#[test]\nfn is_not_listed() {}\n"],
    ]) {
      mkdirSync(join(root, file, ".."), { recursive: true });
      writeFileSync(join(root, file), text);
    }
    const target = (name, kind, file) => ({ name, kind, src_path: join(root, "crates/a", file) });
    options = {
      binaries: binariesOf({
        packages: [
          {
            name: "crate-a",
            targets: [
              target("crate_a", ["lib"], "src/lib.rs"),
              target("cli", ["test"], "tests/cli.rs"),
              target("other", ["test"], "tests/other.rs"),
              target("crate-a", ["bin"], "src/main.rs"),
            ],
          },
        ],
      }),
      root,
      shaped: new Set(["crates/a/tests/cli.rs", "crates/a/tests/missing.rs"]),
    };
  });

  describe("binariesOf(metadata)", () => {
    it("names a library by its package, a test by both, and a bin by its kind", () => {
      assert.deepEqual([...options.binaries.keys()], ["crate-a", "crate-a::cli", "crate-a::other", "crate-a::bin/crate-a"]);
    });
  });

  describe("merged(reports, options)", () => {
    it("finds a case below `mod tests` in the file beside its module, and names each level below `tests`", () => {
      const { xml, unplaced } = merged(
        [report("nextest-run", suite("crate-a", passing("crate-a", "naming::tests::naming_a_record::when_it_has_a_pipe::escapes_the_pipe")))],
        options,
      );
      assert.deepEqual(unplaced, []);
      assert.deepEqual(propertiesOf(xml, "naming::tests::naming_a_record::when_it_has_a_pipe::escapes_the_pipe"), [
        ["file", "crates/a/src/naming/tests.rs"],
        ["line", "21"],
        ["title", "escapes it, as `%7C`"],
        ["describe:1:line", "11"],
        ["describe:1:title", "Naming a record · naming()"],
        ["describe:1:description", "A record is named by its selector, which may hold a pipe.&#10;&#10;Its name is kept as it was written."],
        ["describe:1:rule", "naming N1"],
        ["describe:2:line", "15"],
        ["describe:2:title", "when the selector has a pipe"],
      ]);
    });

    it("gives a case with no doc its name in words, and none of a description", () => {
      const { xml } = merged(
        [report("nextest-run", suite("crate-a", passing("crate-a", "naming::tests::naming_a_record::when_it_has_a_pipe::keeps_the_base")))],
        options,
      );
      assert.deepEqual(propertiesOf(xml, "naming::tests::naming_a_record::when_it_has_a_pipe::keeps_the_base").slice(0, 3), [
        ["file", "crates/a/src/naming/tests.rs"],
        ["line", "26"],
        ["title", "keeps the base"],
      ]);
    });

    it("finds a case of an integration test, and one with no module", () => {
      const { xml } = merged(
        [
          report(
            "nextest-run",
            suite(
              "crate-a::cli",
              passing("crate-a::cli", "converting_a_document::with_no_out::writes_to_standard_output") + passing("crate-a::cli", "stands_alone"),
            ),
          ),
        ],
        options,
      );
      assert.deepEqual(propertiesOf(xml, "converting_a_document::with_no_out::writes_to_standard_output"), [
        ["file", "crates/a/tests/cli.rs"],
        ["line", "8"],
        ["title", "writes to standard output"],
        ["describe:1:line", "4"],
        ["describe:1:title", "Converting a document · convert"],
        ["describe:2:line", "6"],
        ["describe:2:title", "leaves no file"],
      ]);
      assert.deepEqual(propertiesOf(xml, "stands_alone").slice(0, 2), [
        ["file", "crates/a/tests/cli.rs"],
        ["line", "13"],
      ]);
    });

    it("opens a self-closed case, and keeps a failure where it was", () => {
      const failed = `        <testcase name="stands_alone" classname="crate-a::cli" time="0.001">\n            <failure type="x">a &lt;testcase&gt; failed</failure>\n        </testcase>\n`;
      const closed = `        <testcase name="converting_a_document::with_no_out::writes_to_standard_output" classname="crate-a::cli" time="0.001"/>\n`;
      const { xml } = merged([report("nextest-run", suite("crate-a::cli", failed + closed))], options);
      assert.match(xml, /<testcase name="stands_alone"[^>]*><properties>.*<\/properties>\n            <failure type="x">a &lt;testcase&gt; failed<\/failure>/);
      assert.match(xml, /time="0.001"><properties>.*<\/properties><\/testcase>/);
    });

    it("merges every suite of two reports under one root named for nextest", () => {
      const first = report("nextest-run", suite("crate-a", passing("crate-a", "naming::tests::naming_a_record::when_it_has_a_pipe::keeps_the_base")));
      const second = report("nextest-run", suite("crate-a::cli", passing("crate-a::cli", "stands_alone")));
      const { xml } = merged([first, second], options);
      assert.equal(xml.match(/<testsuites /g)?.length, 1);
      assert.equal(xml.match(/<testsuite /g)?.length, 2);
      assert.match(xml, /^<\?xml[^>]*\?>\n<testsuites name="nextest-run" tests="6" failures="2" errors="0" time="1.000">/);
    });

    it("leaves a case of a file the spec does not list as it was written, and names one of a listed file it cannot find", () => {
      const { xml, unplaced } = merged(
        [
          report(
            "nextest-run",
            suite(
              "crate-a::other",
              passing("crate-a::other", "is_not_listed") + passing("crate-a::other", "is_not_in_the_source") + passing("elsewhere", "has_no_binary"),
            ) +
              suite("crate-a::cli", passing("crate-a::cli", "converting_a_document::with_no_out::is_gone") + passing("crate-a::cli", "no_such_module::leaf")),
          ),
        ],
        options,
      );
      assert.deepEqual(unplaced, ["crate-a::cli converting_a_document::with_no_out::is_gone", "crate-a::cli no_such_module::leaf"]);
      assert.deepEqual(propertiesOf(xml, "is_not_in_the_source"), []);
      assert.deepEqual(propertiesOf(xml, "has_no_binary"), []);
    });

    it("refuses a report that is not nextest's", () => {
      assert.throws(() => merged([report("pytest tests", "")], options), /not nextest's/);
    });
  });
});

describe("said(doc, name)", () => {
  it("takes the first paragraph for the title and the rest for the description, and a rule out of both", () => {
    assert.deepEqual(said(["Title", "goes on.", "", "A  second.", "@rule matcher M20", "", "A third."], "a_name"), {
      title: "Title goes on.",
      description: "A second.\n\nA third.",
      rules: ["matcher M20"],
    });
  });

  it("gives a name in words where there is no doc", () => {
    assert.deepEqual(said([], "writes_the_graph"), { title: "writes the graph", description: "", rules: [] });
  });
});
