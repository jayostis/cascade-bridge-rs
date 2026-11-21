window.BENCHMARK_DATA = {
  "lastUpdate": 1795234302343,
  "repoUrl": "https://github.com/the-cascade-protocol/cascade-bridge-rs",
  "entries": {
    "Bridge performance": [
      {
        "commit": {
          "author": {
            "email": "jayostis@gmail.com",
            "name": "jayostis",
            "username": "jayostis"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "23bbe7009eee5d035e5668f85f20b764e8b49e76",
          "message": "Merge pull request #153 from jayostis/feat/perf-files\n\nfeat(bench): stage rows, output digests, boot split, adapter A/B and a perf workflow",
          "timestamp": "2026-11-19T12:58:33-08:00",
          "tree_id": "48a6df88e21918dcef793183beee17c06fb718a5",
          "url": "https://github.com/jayostis/cascade-bridge-rs/commit/23bbe7009eee5d035e5668f85f20b764e8b49e76"
        },
        "date": 1795122019922,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "ccda-entries-x10/convert/first",
            "value": 0.434056,
            "range": "0.393567-0.441827",
            "unit": "s",
            "extra": "Seconds the first convert of a fresh process takes, timed without the step-by-step breakdown. A caller that opens an adapter and converts once also waits for the engine to start (the boot rows) and the adapter to load (the load rows), which this does not count. sha256 77454794ba3cf02cf329e249f7a4a1874ca58cd0761d4f5622163ea9e85efa0e, 138124 bytes"
          },
          {
            "name": "ccda-entries-x10/convert/second",
            "value": 0.200146,
            "range": "0.187327-0.215684",
            "unit": "s",
            "extra": "Seconds a second convert of the same document takes in the same process, timed without the step-by-step breakdown: the warm cost."
          },
          {
            "name": "ccda-entries-x10/convert/profiled",
            "value": 0.336117,
            "range": "0.336117-0.336117",
            "unit": "s",
            "extra": "Seconds of the one convert run with the step-by-step breakdown on, from which the stage and query rows are read. The breakdown makes it slower than a plain convert."
          },
          {
            "name": "ccda-entries-x10/wasm-memory/first",
            "value": 52625408,
            "range": "52625408-52625408",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the first convert is done."
          },
          {
            "name": "ccda-entries-x10/wasm-memory/second",
            "value": 52625408,
            "range": "52625408-52625408",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the second convert is done."
          },
          {
            "name": "ccda-entries-x10/rss",
            "value": 353087488,
            "range": "344805376-372166656",
            "unit": "By",
            "extra": "Peak memory the whole process held in RAM, in bytes: the engine's start, the adapter's load and both converts all count."
          },
          {
            "name": "ccda-entries-x10/records",
            "value": 1,
            "unit": "{count}",
            "extra": "Source records the Bridge found in the document and converted."
          },
          {
            "name": "ccda-entries-x10/triples",
            "value": 1926,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph."
          },
          {
            "name": "ccda-entries-x10/findings",
            "value": 93,
            "unit": "{count}",
            "extra": "Findings the conversion wrote."
          },
          {
            "name": "ccda-entries-x10/query-time/document-table.rq",
            "value": 0.07111,
            "range": "0.07111-0.07111",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/allergy.rq",
            "value": 0.000231,
            "range": "0.000231-0.000231",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/condition.rq",
            "value": 0.000209,
            "range": "0.000209-0.000209",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/document-organizations.rq",
            "value": 0.000936,
            "range": "0.000936-0.000936",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/immunization.rq",
            "value": 0.000465,
            "range": "0.000465-0.000465",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/medication.rq",
            "value": 0.008955,
            "range": "0.008955-0.008955",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/note-or-document.rq",
            "value": 0.001451,
            "range": "0.001451-0.001451",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/patient.rq",
            "value": 0.000218,
            "range": "0.000218-0.000218",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/procedure.rq",
            "value": 0.000496,
            "range": "0.000496-0.000496",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/record.rq",
            "value": 0.004474,
            "range": "0.004474-0.004474",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/report.rq",
            "value": 0.003056,
            "range": "0.003056-0.003056",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/result-identifiers.rq",
            "value": 0.001773,
            "range": "0.001773-0.001773",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/result-quantities.rq",
            "value": 0.004268,
            "range": "0.004268-0.004268",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/result.rq",
            "value": 0.014737,
            "range": "0.014737-0.014737",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/allergy-findings.rq",
            "value": 0.000336,
            "range": "0.000336-0.000336",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/condition-findings.rq",
            "value": 0.000627,
            "range": "0.000627-0.000627",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/document-findings.rq",
            "value": 0.000478,
            "range": "0.000478-0.000478",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/entry-findings.rq",
            "value": 0.002122,
            "range": "0.002122-0.002122",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/entry-statement-findings.rq",
            "value": 0.003008,
            "range": "0.003008-0.003008",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/medication-findings.rq",
            "value": 0.001911,
            "range": "0.001911-0.001911",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/name-findings.rq",
            "value": 0.000976,
            "range": "0.000976-0.000976",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/narrative-findings.rq",
            "value": 0.002221,
            "range": "0.002221-0.002221",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/organization-findings.rq",
            "value": 0.000462,
            "range": "0.000462-0.000462",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/patient-findings.rq",
            "value": 0.000086,
            "range": "0.000086-0.000086",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/reference-findings.rq",
            "value": 0.002773,
            "range": "0.002773-0.002773",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/report-and-note-findings.rq",
            "value": 0.001014,
            "range": "0.001014-0.001014",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/result-findings.rq",
            "value": 0.013863,
            "range": "0.013863-0.013863",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/section-findings.rq",
            "value": 0.000558,
            "range": "0.000558-0.000558",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/detect.rq",
            "value": 0.006226,
            "range": "0.006226-0.006226",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/stage/document-table",
            "value": 0.073081,
            "range": "0.073081-0.073081",
            "unit": "s",
            "extra": "Running the adapter's document-table query on every record of the document, to link a record to any other record, in the document or not. (Specification stage: link records.)"
          },
          {
            "name": "ccda-entries-x10/stage/lift",
            "value": 0.04059,
            "range": "0.04059-0.04059",
            "unit": "s",
            "extra": "Reading the source document into the triples the adapter's queries can ask about, one record at a time. (Specification stage: read and chunk.)"
          },
          {
            "name": "ccda-entries-x10/stage/source-validation",
            "value": 0.017818,
            "range": "0.017818-0.017818",
            "unit": "s",
            "extra": "Checking each source record, and the document as a whole where the adapter gives a document schema, against the adapter's schema (XSD or JSON Schema). A failure is a finding, never a refusal. (Specification stage: check, validate.)"
          },
          {
            "name": "ccda-entries-x10/stage/accounting",
            "value": 0.000869,
            "range": "0.000869-0.000869",
            "unit": "s",
            "extra": "Working out, against the adapter's accounting file, what became of each part of each source record."
          },
          {
            "name": "ccda-entries-x10/stage/dataset",
            "value": 0.000052,
            "range": "0.000052-0.000052",
            "unit": "s",
            "extra": "Putting the facts about a record into the store its queries run over."
          },
          {
            "name": "ccda-entries-x10/stage/index",
            "value": 0.015041,
            "range": "0.015041-0.015041",
            "unit": "s",
            "extra": "Building the lookup structure that compiled queries read, from a record's triples. A query the general evaluator answers needs none."
          },
          {
            "name": "ccda-entries-x10/stage/mapping",
            "value": 0.041609,
            "range": "0.041609-0.041609",
            "unit": "s",
            "extra": "Running the adapter's mapping queries on each record, without the time of building their lookup structure. (Specification stage: transform.)"
          },
          {
            "name": "ccda-entries-x10/stage/records",
            "value": 0.020282,
            "range": "0.020282-0.020282",
            "unit": "s",
            "extra": "Turning what the mapping produced into each record's graph: naming its versions, keeping one where the mapping wrote several, and writing their arrivals. (Specification stage: name versions, write arrivals.)"
          },
          {
            "name": "ccda-entries-x10/stage/output-validation",
            "value": 0.057774,
            "range": "0.057774-0.057774",
            "unit": "s",
            "extra": "Checking each record's output against the vocabulary's SHACL shapes."
          },
          {
            "name": "ccda-entries-x10/stage/findings",
            "value": 0.03372,
            "range": "0.03372-0.03372",
            "unit": "s",
            "extra": "Running the adapter's findings queries to find what the source record got wrong or left out. (Specification stage: findings.)"
          },
          {
            "name": "ccda-entries-x10/stage/unresolved-addresses",
            "value": 0.010305,
            "range": "0.010305-0.010305",
            "unit": "s",
            "extra": "Finding the references to records the document does not hold, which keep their computed names."
          },
          {
            "name": "ccda-entries-x10/stage/detect",
            "value": 0.006269,
            "range": "0.006269-0.006269",
            "unit": "s",
            "extra": "Running the adapter's detect query, which reports whether the adapter reads this document. The Bridge converts every record whatever the answer; it never routes."
          },
          {
            "name": "ccda-entries-x10/stage/summary",
            "value": 0.000732,
            "range": "0.000732-0.000732",
            "unit": "s",
            "extra": "Counting the graph's distinct triples and the findings once every record is done."
          },
          {
            "name": "ccda-entries-x10/stage/serialising",
            "value": 0.006542,
            "range": "0.006542-0.006542",
            "unit": "s",
            "extra": "Writing the graph and the findings out as Turtle or N-Triples."
          },
          {
            "name": "ccda-entries-x10/stage/unaccounted",
            "value": 0.011433,
            "range": "0.011433-0.011433",
            "unit": "s",
            "extra": "Seconds of the profiled convert that none of the stages accounts for, such as writing the document and the import into the graph."
          },
          {
            "name": "ccda-entries-x10/output/graph",
            "value": 1926,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph, with a SHA-256 fingerprint of the output that is the same whenever the content is the same, whatever order the engine wrote it in and whatever it named its blank nodes. sha256 ad5fa27c4fe8b29ebed8c8892c3bf3df74f202365f5e14dca5a7981d130dfc16"
          },
          {
            "name": "ccda-entries-x10/output/findings",
            "value": 1210,
            "unit": "{count}",
            "extra": "Distinct triples in the findings, with a SHA-256 fingerprint of them that is the same whenever the content is the same, whatever order the engine wrote them in and whatever it named its blank nodes. sha256 95da111564c5677296616ba1bec9f3340f1788949bbde806f616a5ca56b4ba0a"
          },
          {
            "name": "ccda/load",
            "value": 0.678689,
            "range": "0.640608-0.78057",
            "unit": "s",
            "extra": "Seconds to read the adapter's and the vocabulary's files, describe the adapter and prepare it, with the engine already started."
          },
          {
            "name": "ccda/load/read-files",
            "value": 0.005059,
            "range": "0.005046-0.006599",
            "unit": "s",
            "extra": "Seconds of the load spent reading files from disk."
          },
          {
            "name": "ccda/load/prepare",
            "value": 0.67209,
            "range": "0.635549-0.775525",
            "unit": "s",
            "extra": "Seconds of the load spent describing the adapter and preparing it: parsing and compiling its queries, schemas and shapes."
          },
          {
            "name": "ccda/boot/read-wasm",
            "value": 0.003688,
            "range": "0.003631-0.003694",
            "unit": "s",
            "extra": "Seconds to read the engine's compiled code from disk."
          },
          {
            "name": "ccda/boot/compile-wasm",
            "value": 0.016248,
            "range": "0.016123-0.016353",
            "unit": "s",
            "extra": "Seconds to turn the engine's compiled code into something the machine can run."
          },
          {
            "name": "ccda/boot/instantiate-wasm",
            "value": 0.001042,
            "range": "0.000957-0.002164",
            "unit": "s",
            "extra": "Seconds to start the compiled code, which runs its start-up work."
          },
          {
            "name": "fhir-r4-lab-pull-3000/convert/first",
            "value": 4.810136,
            "range": "4.799477-4.8353",
            "unit": "s",
            "extra": "Seconds the first convert of a fresh process takes, timed without the step-by-step breakdown. A caller that opens an adapter and converts once also waits for the engine to start (the boot rows) and the adapter to load (the load rows), which this does not count. sha256 69b4daac45fd32449b8c290be9a7551566b537a6171218e8125169e94ca5882b, 2760035 bytes"
          },
          {
            "name": "fhir-r4-lab-pull-3000/convert/second",
            "value": 4.506259,
            "range": "4.501731-4.595525",
            "unit": "s",
            "extra": "Seconds a second convert of the same document takes in the same process, timed without the step-by-step breakdown: the warm cost."
          },
          {
            "name": "fhir-r4-lab-pull-3000/convert/profiled",
            "value": 4.892735,
            "range": "4.892735-4.892735",
            "unit": "s",
            "extra": "Seconds of the one convert run with the step-by-step breakdown on, from which the stage and query rows are read. The breakdown makes it slower than a plain convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/wasm-memory/first",
            "value": 136511488,
            "range": "136511488-136511488",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the first convert is done."
          },
          {
            "name": "fhir-r4-lab-pull-3000/wasm-memory/second",
            "value": 136511488,
            "range": "136511488-136511488",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the second convert is done."
          },
          {
            "name": "fhir-r4-lab-pull-3000/rss",
            "value": 415424512,
            "range": "369061888-441966592",
            "unit": "By",
            "extra": "Peak memory the whole process held in RAM, in bytes: the engine's start, the adapter's load and both converts all count."
          },
          {
            "name": "fhir-r4-lab-pull-3000/records",
            "value": 3004,
            "unit": "{count}",
            "extra": "Source records the Bridge found in the document and converted."
          },
          {
            "name": "fhir-r4-lab-pull-3000/triples",
            "value": 63025,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph."
          },
          {
            "name": "fhir-r4-lab-pull-3000/findings",
            "value": 3006,
            "unit": "{count}",
            "extra": "Findings the conversion wrote."
          },
          {
            "name": "fhir-r4-lab-pull-3000/fallbacks",
            "value": 0,
            "unit": "{count}",
            "extra": "Record executions the general evaluator answered because a query was not compiled or a compiled plan gave up. The detect query, which runs once, is not counted."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/coding-system-table.rq",
            "value": 0.049146,
            "range": "0.049146-0.049146",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/document-table.rq",
            "value": 0.007971,
            "range": "0.007971-0.007971",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/lab-result-table.rq",
            "value": 0.015229,
            "range": "0.015229-0.015229",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/medication-table.rq",
            "value": 0.00847,
            "range": "0.00847-0.00847",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/note-or-document-table.rq",
            "value": 0.004576,
            "range": "0.004576-0.004576",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/organization-table.rq",
            "value": 0.01561,
            "range": "0.01561-0.01561",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/variant-interpretation-table.rq",
            "value": 0.006891,
            "range": "0.006891-0.006891",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/variant-table.rq",
            "value": 0.007763,
            "range": "0.007763-0.007763",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/absent-reasons.rq",
            "value": 0.039774,
            "range": "0.039774-0.039774",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/allergy-intolerance.rq",
            "value": 0.005793,
            "range": "0.005793-0.005793",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/attachments.rq",
            "value": 0.007446,
            "range": "0.007446-0.007446",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/condition.rq",
            "value": 0.00548,
            "range": "0.00548-0.00548",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/immunization.rq",
            "value": 0.00537,
            "range": "0.00537-0.00537",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/lab-result-codes.rq",
            "value": 0.071913,
            "range": "0.071913-0.071913",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/lab-result-identifiers.rq",
            "value": 0.01603,
            "range": "0.01603-0.01603",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/lab-result-performer.rq",
            "value": 0.002576,
            "range": "0.002576-0.002576",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/lab-result-quantities.rq",
            "value": 0.078741,
            "range": "0.078741-0.078741",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/lab-result.rq",
            "value": 0.081995,
            "range": "0.081995-0.081995",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/medication-dispense-prescription.rq",
            "value": 0.007615,
            "range": "0.007615-0.007615",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/medication-dispense.rq",
            "value": 0.005107,
            "range": "0.005107-0.005107",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/medication-supply-and-dosages.rq",
            "value": 0.006896,
            "range": "0.006896-0.006896",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/medication.rq",
            "value": 0.006507,
            "range": "0.006507-0.006507",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/narrative-text.rq",
            "value": 0.021144,
            "range": "0.021144-0.021144",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/note-or-document.rq",
            "value": 0.005312,
            "range": "0.005312-0.005312",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/organization-from-server.rq",
            "value": 0.003419,
            "range": "0.003419-0.003419",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/organization.rq",
            "value": 0.008719,
            "range": "0.008719-0.008719",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/patient.rq",
            "value": 0.005569,
            "range": "0.005569-0.005569",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/procedure.rq",
            "value": 0.005231,
            "range": "0.005231-0.005231",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/record.rq",
            "value": 0.200996,
            "range": "0.200996-0.200996",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/report-and-note-performer.rq",
            "value": 0.002451,
            "range": "0.002451-0.002451",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/report-results.rq",
            "value": 0.007553,
            "range": "0.007553-0.007553",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/report.rq",
            "value": 0.005445,
            "range": "0.005445-0.005445",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/variant-interpretation.rq",
            "value": 0.007405,
            "range": "0.007405-0.007405",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/variant.rq",
            "value": 0.006722,
            "range": "0.006722-0.006722",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/absent-reason-findings.rq",
            "value": 0.024019,
            "range": "0.024019-0.024019",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/category-text-findings.rq",
            "value": 0.007618,
            "range": "0.007618-0.007618",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/code-findings.rq",
            "value": 0.005708,
            "range": "0.005708-0.005708",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/date-findings.rq",
            "value": 0.029375,
            "range": "0.029375-0.029375",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/document-version-findings.rq",
            "value": 0.00095,
            "range": "0.00095-0.00095",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/lab-result-findings.rq",
            "value": 0.051582,
            "range": "0.051582-0.051582",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/medication-dispense-findings.rq",
            "value": 0.005557,
            "range": "0.005557-0.005557",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/medication-dispense-prescription-findings.rq",
            "value": 0.006511,
            "range": "0.006511-0.006511",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/medication-findings.rq",
            "value": 0.045111,
            "range": "0.045111-0.045111",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/narrative-findings.rq",
            "value": 0.004834,
            "range": "0.004834-0.004834",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/observation-component-findings.rq",
            "value": 0.007164,
            "range": "0.007164-0.007164",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/observation-findings.rq",
            "value": 0.045709,
            "range": "0.045709-0.045709",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/organization-findings.rq",
            "value": 0.005604,
            "range": "0.005604-0.005604",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/patient-findings.rq",
            "value": 0.004919,
            "range": "0.004919-0.004919",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/patient-reference-findings.rq",
            "value": 0.036493,
            "range": "0.036493-0.036493",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/recorded-date-findings.rq",
            "value": 0.006064,
            "range": "0.006064-0.006064",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/reference-display-findings.rq",
            "value": 0.024112,
            "range": "0.024112-0.024112",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/reference-without-server-findings.rq",
            "value": 0.028749,
            "range": "0.028749-0.028749",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/repeated-id-findings.rq",
            "value": 0.017601,
            "range": "0.017601-0.017601",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/report-and-note-findings.rq",
            "value": 0.011649,
            "range": "0.011649-0.011649",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/report-result-findings.rq",
            "value": 0.007136,
            "range": "0.007136-0.007136",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/resource-type-findings.rq",
            "value": 0.007324,
            "range": "0.007324-0.007324",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/variant-findings.rq",
            "value": 0.013192,
            "range": "0.013192-0.013192",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/variant-interpretation-findings.rq",
            "value": 0.007573,
            "range": "0.007573-0.007573",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/detect.rq",
            "value": 0.006135,
            "range": "0.006135-0.006135",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/document-table",
            "value": 0.146862,
            "range": "0.146862-0.146862",
            "unit": "s",
            "extra": "Running the adapter's document-table query on every record of the document, to link a record to any other record, in the document or not. (Specification stage: link records.)"
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/lift",
            "value": 0.288022,
            "range": "0.288022-0.288022",
            "unit": "s",
            "extra": "Reading the source document into the triples the adapter's queries can ask about, one record at a time. (Specification stage: read and chunk.)"
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/source-validation",
            "value": 0.694338,
            "range": "0.694338-0.694338",
            "unit": "s",
            "extra": "Checking each source record, and the document as a whole where the adapter gives a document schema, against the adapter's schema (XSD or JSON Schema). A failure is a finding, never a refusal. (Specification stage: check, validate.)"
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/accounting",
            "value": 0.010399,
            "range": "0.010399-0.010399",
            "unit": "s",
            "extra": "Working out, against the adapter's accounting file, what became of each part of each source record."
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/dataset",
            "value": 0.006898,
            "range": "0.006898-0.006898",
            "unit": "s",
            "extra": "Putting the facts about a record into the store its queries run over."
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/index",
            "value": 0.151176,
            "range": "0.151176-0.151176",
            "unit": "s",
            "extra": "Building the lookup structure that compiled queries read, from a record's triples. A query the general evaluator answers needs none."
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/mapping",
            "value": 0.645015,
            "range": "0.645015-0.645015",
            "unit": "s",
            "extra": "Running the adapter's mapping queries on each record, without the time of building their lookup structure. (Specification stage: transform.)"
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/records",
            "value": 0.163581,
            "range": "0.163581-0.163581",
            "unit": "s",
            "extra": "Turning what the mapping produced into each record's graph: naming its versions, keeping one where the mapping wrote several, and writing their arrivals. (Specification stage: name versions, write arrivals.)"
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/output-validation",
            "value": 1.940973,
            "range": "1.940973-1.940973",
            "unit": "s",
            "extra": "Checking each record's output against the vocabulary's SHACL shapes."
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/findings",
            "value": 0.542174,
            "range": "0.542174-0.542174",
            "unit": "s",
            "extra": "Running the adapter's findings queries to find what the source record got wrong or left out. (Specification stage: findings.)"
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/unresolved-addresses",
            "value": 0.026309,
            "range": "0.026309-0.026309",
            "unit": "s",
            "extra": "Finding the references to records the document does not hold, which keep their computed names."
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/detect",
            "value": 0.006195,
            "range": "0.006195-0.006195",
            "unit": "s",
            "extra": "Running the adapter's detect query, which reports whether the adapter reads this document. The Bridge converts every record whatever the answer; it never routes."
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/summary",
            "value": 0.01334,
            "range": "0.01334-0.01334",
            "unit": "s",
            "extra": "Counting the graph's distinct triples and the findings once every record is done."
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/serialising",
            "value": 0.106314,
            "range": "0.106314-0.106314",
            "unit": "s",
            "extra": "Writing the graph and the findings out as Turtle or N-Triples."
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/unaccounted",
            "value": 0.15114,
            "range": "0.15114-0.15114",
            "unit": "s",
            "extra": "Seconds of the profiled convert that none of the stages accounts for, such as writing the document and the import into the graph."
          },
          {
            "name": "fhir-r4-lab-pull-3000/output/graph",
            "value": 63025,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph, with a SHA-256 fingerprint of the output that is the same whenever the content is the same, whatever order the engine wrote it in and whatever it named its blank nodes. sha256 37ddd3ff45eedcca686486482a5dd0ea90120d791c04352fb96a41e20d5fe7ac"
          },
          {
            "name": "fhir-r4-lab-pull-3000/output/findings",
            "value": 45090,
            "unit": "{count}",
            "extra": "Distinct triples in the findings, with a SHA-256 fingerprint of them that is the same whenever the content is the same, whatever order the engine wrote them in and whatever it named its blank nodes. sha256 b882dcb20e82c494692121aa741018d60fa8b958d52919c0a6eca58735b69721"
          },
          {
            "name": "fhir-r4/load",
            "value": 0.56777,
            "range": "0.562824-0.571871",
            "unit": "s",
            "extra": "Seconds to read the adapter's and the vocabulary's files, describe the adapter and prepare it, with the engine already started."
          },
          {
            "name": "fhir-r4/load/read-files",
            "value": 0.009244,
            "range": "0.008146-0.011042",
            "unit": "s",
            "extra": "Seconds of the load spent reading files from disk."
          },
          {
            "name": "fhir-r4/load/prepare",
            "value": 0.558526,
            "range": "0.551782-0.563725",
            "unit": "s",
            "extra": "Seconds of the load spent describing the adapter and preparing it: parsing and compiling its queries, schemas and shapes."
          },
          {
            "name": "fhir-r4/boot/read-wasm",
            "value": 0.003812,
            "range": "0.003623-0.003837",
            "unit": "s",
            "extra": "Seconds to read the engine's compiled code from disk."
          },
          {
            "name": "fhir-r4/boot/compile-wasm",
            "value": 0.013507,
            "range": "0.013079-0.014009",
            "unit": "s",
            "extra": "Seconds to turn the engine's compiled code into something the machine can run."
          },
          {
            "name": "fhir-r4/boot/instantiate-wasm",
            "value": 0.000977,
            "range": "0.000922-0.001014",
            "unit": "s",
            "extra": "Seconds to start the compiled code, which runs its start-up work."
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "jayostis",
            "username": "jayostis",
            "email": "jayostis@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "23bbe7009eee5d035e5668f85f20b764e8b49e76",
          "message": "Merge pull request #153 from jayostis/feat/perf-files\n\nfeat(bench): stage rows, output digests, boot split, adapter A/B and a perf workflow",
          "timestamp": "2026-11-19T20:58:33Z",
          "url": "https://github.com/jayostis/cascade-bridge-rs/commit/23bbe7009eee5d035e5668f85f20b764e8b49e76"
        },
        "date": 1795147083919,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "ccda-entries-x10/convert/first",
            "value": 0.566586,
            "range": "0.539352-0.629339",
            "unit": "s",
            "extra": "Seconds the first convert of a fresh process takes, timed without the step-by-step breakdown. A caller that opens an adapter and converts once also waits for the engine to start (the boot rows) and the adapter to load (the load rows), which this does not count. The C-CDA medications-and-results example with its entries repeated 10 times, as one document. sha256 77454794ba3cf02cf329e249f7a4a1874ca58cd0761d4f5622163ea9e85efa0e, 138124 bytes"
          },
          {
            "name": "ccda-entries-x10/convert/second",
            "value": 0.296036,
            "range": "0.293679-0.316876",
            "unit": "s",
            "extra": "Seconds a second convert of the same document takes in the same process, timed without the step-by-step breakdown: the warm cost."
          },
          {
            "name": "ccda-entries-x10/convert/profiled",
            "value": 0.520059,
            "range": "0.520059-0.520059",
            "unit": "s",
            "extra": "Seconds of the one convert run with the step-by-step breakdown on, from which the stage and query rows are read. The breakdown makes it slower than a plain convert."
          },
          {
            "name": "ccda-entries-x10/wasm-memory/first",
            "value": 52625408,
            "range": "52625408-52625408",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the first convert is done."
          },
          {
            "name": "ccda-entries-x10/wasm-memory/second",
            "value": 52625408,
            "range": "52625408-52625408",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the second convert is done."
          },
          {
            "name": "ccda-entries-x10/rss",
            "value": 363003904,
            "range": "354840576-366256128",
            "unit": "By",
            "extra": "Peak memory the whole process held in RAM, in bytes: the engine's start, the adapter's load and both converts all count."
          },
          {
            "name": "ccda-entries-x10/records",
            "value": 1,
            "unit": "{count}",
            "extra": "Source records the Bridge found in the document and converted. The C-CDA medications-and-results example with its entries repeated 10 times, as one document."
          },
          {
            "name": "ccda-entries-x10/triples",
            "value": 1926,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph."
          },
          {
            "name": "ccda-entries-x10/findings",
            "value": 93,
            "unit": "{count}",
            "extra": "Findings the conversion wrote."
          },
          {
            "name": "ccda-entries-x10/query-time/document-table.rq",
            "value": 0.112992,
            "range": "0.112992-0.112992",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/allergy.rq",
            "value": 0.000439,
            "range": "0.000439-0.000439",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/condition.rq",
            "value": 0.000373,
            "range": "0.000373-0.000373",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/document-organizations.rq",
            "value": 0.001547,
            "range": "0.001547-0.001547",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/immunization.rq",
            "value": 0.000813,
            "range": "0.000813-0.000813",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/medication.rq",
            "value": 0.015661,
            "range": "0.015661-0.015661",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/note-or-document.rq",
            "value": 0.002348,
            "range": "0.002348-0.002348",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/patient.rq",
            "value": 0.000352,
            "range": "0.000352-0.000352",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/procedure.rq",
            "value": 0.000822,
            "range": "0.000822-0.000822",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/record.rq",
            "value": 0.007154,
            "range": "0.007154-0.007154",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/report.rq",
            "value": 0.00525,
            "range": "0.00525-0.00525",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/result-identifiers.rq",
            "value": 0.00289,
            "range": "0.00289-0.00289",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/result-quantities.rq",
            "value": 0.006895,
            "range": "0.006895-0.006895",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/result.rq",
            "value": 0.018429,
            "range": "0.018429-0.018429",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/allergy-findings.rq",
            "value": 0.000289,
            "range": "0.000289-0.000289",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/condition-findings.rq",
            "value": 0.000466,
            "range": "0.000466-0.000466",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/document-findings.rq",
            "value": 0.000407,
            "range": "0.000407-0.000407",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/entry-findings.rq",
            "value": 0.001918,
            "range": "0.001918-0.001918",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/entry-statement-findings.rq",
            "value": 0.002901,
            "range": "0.002901-0.002901",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/medication-findings.rq",
            "value": 0.001778,
            "range": "0.001778-0.001778",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/name-findings.rq",
            "value": 0.000791,
            "range": "0.000791-0.000791",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/narrative-findings.rq",
            "value": 0.002037,
            "range": "0.002037-0.002037",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/organization-findings.rq",
            "value": 0.000406,
            "range": "0.000406-0.000406",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/patient-findings.rq",
            "value": 0.000072,
            "range": "0.000072-0.000072",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/reference-findings.rq",
            "value": 0.002581,
            "range": "0.002581-0.002581",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/report-and-note-findings.rq",
            "value": 0.001654,
            "range": "0.001654-0.001654",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/result-findings.rq",
            "value": 0.014392,
            "range": "0.014392-0.014392",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/section-findings.rq",
            "value": 0.00081,
            "range": "0.00081-0.00081",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/detect.rq",
            "value": 0.01014,
            "range": "0.01014-0.01014",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/stage/document-table",
            "value": 0.11564,
            "range": "0.11564-0.11564",
            "unit": "s",
            "extra": "Running the adapter's document-table query on every record of the document, to link a record to any other record, in the document or not. (Specification stage: link records.)"
          },
          {
            "name": "ccda-entries-x10/stage/lift",
            "value": 0.066781,
            "range": "0.066781-0.066781",
            "unit": "s",
            "extra": "Reading the source document into the triples the adapter's queries can ask about, one record at a time. (Specification stage: read and chunk.)"
          },
          {
            "name": "ccda-entries-x10/stage/source-validation",
            "value": 0.030577,
            "range": "0.030577-0.030577",
            "unit": "s",
            "extra": "Checking each source record, and the document as a whole where the adapter gives a document schema, against the adapter's schema (XSD or JSON Schema). A failure is a finding, never a refusal. (Specification stage: check, validate.)"
          },
          {
            "name": "ccda-entries-x10/stage/accounting",
            "value": 0.001395,
            "range": "0.001395-0.001395",
            "unit": "s",
            "extra": "Working out, against the adapter's accounting file, what became of each part of each source record."
          },
          {
            "name": "ccda-entries-x10/stage/dataset",
            "value": 0.000087,
            "range": "0.000087-0.000087",
            "unit": "s",
            "extra": "Putting the facts about a record into the store its queries run over."
          },
          {
            "name": "ccda-entries-x10/stage/index",
            "value": 0.026552,
            "range": "0.026552-0.026552",
            "unit": "s",
            "extra": "Building the lookup structure that compiled queries read, from a record's triples. A query the general evaluator answers needs none."
          },
          {
            "name": "ccda-entries-x10/stage/mapping",
            "value": 0.063681,
            "range": "0.063681-0.063681",
            "unit": "s",
            "extra": "Running the adapter's mapping queries on each record, without the time of building their lookup structure. (Specification stage: transform.)"
          },
          {
            "name": "ccda-entries-x10/stage/records",
            "value": 0.03146,
            "range": "0.03146-0.03146",
            "unit": "s",
            "extra": "Turning what the mapping produced into each record's graph: naming its versions, keeping one where the mapping wrote several, and writing their arrivals. (Specification stage: name versions, write arrivals.)"
          },
          {
            "name": "ccda-entries-x10/stage/output-validation",
            "value": 0.095234,
            "range": "0.095234-0.095234",
            "unit": "s",
            "extra": "Checking each record's output against the vocabulary's SHACL shapes."
          },
          {
            "name": "ccda-entries-x10/stage/findings",
            "value": 0.03399,
            "range": "0.03399-0.03399",
            "unit": "s",
            "extra": "Running the adapter's findings queries to find what the source record got wrong or left out. (Specification stage: findings.)"
          },
          {
            "name": "ccda-entries-x10/stage/unresolved-addresses",
            "value": 0.017063,
            "range": "0.017063-0.017063",
            "unit": "s",
            "extra": "Finding the references to records the document does not hold, which keep their computed names."
          },
          {
            "name": "ccda-entries-x10/stage/detect",
            "value": 0.010199,
            "range": "0.010199-0.010199",
            "unit": "s",
            "extra": "Running the adapter's detect query, which reports whether the adapter reads this document. The Bridge converts every record whatever the answer; it never routes."
          },
          {
            "name": "ccda-entries-x10/stage/summary",
            "value": 0.001053,
            "range": "0.001053-0.001053",
            "unit": "s",
            "extra": "Counting the graph's distinct triples and the findings once every record is done."
          },
          {
            "name": "ccda-entries-x10/stage/serialising",
            "value": 0.01059,
            "range": "0.01059-0.01059",
            "unit": "s",
            "extra": "Writing the graph and the findings out as Turtle or N-Triples."
          },
          {
            "name": "ccda-entries-x10/stage/unaccounted",
            "value": 0.015756,
            "range": "0.015756-0.015756",
            "unit": "s",
            "extra": "Seconds of the profiled convert that none of the stages accounts for, such as writing the document and the import into the graph."
          },
          {
            "name": "ccda-entries-x10/output/graph",
            "value": 1926,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph, with a SHA-256 fingerprint of the output that is the same whenever the content is the same, whatever order the engine wrote it in and whatever it named its blank nodes. sha256 ad5fa27c4fe8b29ebed8c8892c3bf3df74f202365f5e14dca5a7981d130dfc16"
          },
          {
            "name": "ccda-entries-x10/output/findings",
            "value": 1210,
            "unit": "{count}",
            "extra": "Distinct triples in the findings, with a SHA-256 fingerprint of them that is the same whenever the content is the same, whatever order the engine wrote them in and whatever it named its blank nodes. sha256 95da111564c5677296616ba1bec9f3340f1788949bbde806f616a5ca56b4ba0a"
          },
          {
            "name": "ccda-entries-x50/convert/first",
            "value": 1.742025,
            "range": "1.726075-1.767357",
            "unit": "s",
            "extra": "Seconds the first convert of a fresh process takes, timed without the step-by-step breakdown. A caller that opens an adapter and converts once also waits for the engine to start (the boot rows) and the adapter to load (the load rows), which this does not count. The C-CDA medications-and-results example with its entries repeated 50 times, as one document. sha256 74d7ceb7fbc39145ce2f7d8c2aca5f71848dc80ced2ebbdf76ed795b43ba7f62, 667684 bytes"
          },
          {
            "name": "ccda-entries-x50/convert/second",
            "value": 1.356582,
            "range": "1.356389-1.362333",
            "unit": "s",
            "extra": "Seconds a second convert of the same document takes in the same process, timed without the step-by-step breakdown: the warm cost."
          },
          {
            "name": "ccda-entries-x50/convert/profiled",
            "value": 1.741047,
            "range": "1.741047-1.741047",
            "unit": "s",
            "extra": "Seconds of the one convert run with the step-by-step breakdown on, from which the stage and query rows are read. The breakdown makes it slower than a plain convert."
          },
          {
            "name": "ccda-entries-x50/wasm-memory/first",
            "value": 136511488,
            "range": "136511488-136511488",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the first convert is done."
          },
          {
            "name": "ccda-entries-x50/wasm-memory/second",
            "value": 136511488,
            "range": "136511488-136511488",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the second convert is done."
          },
          {
            "name": "ccda-entries-x50/rss",
            "value": 395591680,
            "range": "378216448-446738432",
            "unit": "By",
            "extra": "Peak memory the whole process held in RAM, in bytes: the engine's start, the adapter's load and both converts all count."
          },
          {
            "name": "ccda-entries-x50/records",
            "value": 1,
            "unit": "{count}",
            "extra": "Source records the Bridge found in the document and converted. The C-CDA medications-and-results example with its entries repeated 50 times, as one document."
          },
          {
            "name": "ccda-entries-x50/triples",
            "value": 9126,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph."
          },
          {
            "name": "ccda-entries-x50/findings",
            "value": 453,
            "unit": "{count}",
            "extra": "Findings the conversion wrote."
          },
          {
            "name": "ccda-entries-x50/query-time/document-table.rq",
            "value": 0.329152,
            "range": "0.329152-0.329152",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/allergy.rq",
            "value": 0.000971,
            "range": "0.000971-0.000971",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/condition.rq",
            "value": 0.000932,
            "range": "0.000932-0.000932",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/document-organizations.rq",
            "value": 0.001402,
            "range": "0.001402-0.001402",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/immunization.rq",
            "value": 0.002248,
            "range": "0.002248-0.002248",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/medication.rq",
            "value": 0.033287,
            "range": "0.033287-0.033287",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/note-or-document.rq",
            "value": 0.008648,
            "range": "0.008648-0.008648",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/patient.rq",
            "value": 0.000541,
            "range": "0.000541-0.000541",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/procedure.rq",
            "value": 0.002803,
            "range": "0.002803-0.002803",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/record.rq",
            "value": 0.020663,
            "range": "0.020663-0.020663",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/report.rq",
            "value": 0.01217,
            "range": "0.01217-0.01217",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/result-identifiers.rq",
            "value": 0.008305,
            "range": "0.008305-0.008305",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/result-quantities.rq",
            "value": 0.021979,
            "range": "0.021979-0.021979",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/result.rq",
            "value": 0.061529,
            "range": "0.061529-0.061529",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/allergy-findings.rq",
            "value": 0.001169,
            "range": "0.001169-0.001169",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/condition-findings.rq",
            "value": 0.002479,
            "range": "0.002479-0.002479",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/document-findings.rq",
            "value": 0.00115,
            "range": "0.00115-0.00115",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/entry-findings.rq",
            "value": 0.007892,
            "range": "0.007892-0.007892",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/entry-statement-findings.rq",
            "value": 0.013107,
            "range": "0.013107-0.013107",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/medication-findings.rq",
            "value": 0.016191,
            "range": "0.016191-0.016191",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/name-findings.rq",
            "value": 0.003957,
            "range": "0.003957-0.003957",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/narrative-findings.rq",
            "value": 0.003971,
            "range": "0.003971-0.003971",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/organization-findings.rq",
            "value": 0.000899,
            "range": "0.000899-0.000899",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/patient-findings.rq",
            "value": 0.000154,
            "range": "0.000154-0.000154",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/reference-findings.rq",
            "value": 0.013122,
            "range": "0.013122-0.013122",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/report-and-note-findings.rq",
            "value": 0.004503,
            "range": "0.004503-0.004503",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/result-findings.rq",
            "value": 0.063405,
            "range": "0.063405-0.063405",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/section-findings.rq",
            "value": 0.001899,
            "range": "0.001899-0.001899",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/detect.rq",
            "value": 0.006835,
            "range": "0.006835-0.006835",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/stage/document-table",
            "value": 0.337787,
            "range": "0.337787-0.337787",
            "unit": "s",
            "extra": "Running the adapter's document-table query on every record of the document, to link a record to any other record, in the document or not. (Specification stage: link records.)"
          },
          {
            "name": "ccda-entries-x50/stage/lift",
            "value": 0.22089,
            "range": "0.22089-0.22089",
            "unit": "s",
            "extra": "Reading the source document into the triples the adapter's queries can ask about, one record at a time. (Specification stage: read and chunk.)"
          },
          {
            "name": "ccda-entries-x50/stage/source-validation",
            "value": 0.094951,
            "range": "0.094951-0.094951",
            "unit": "s",
            "extra": "Checking each source record, and the document as a whole where the adapter gives a document schema, against the adapter's schema (XSD or JSON Schema). A failure is a finding, never a refusal. (Specification stage: check, validate.)"
          },
          {
            "name": "ccda-entries-x50/stage/accounting",
            "value": 0.000923,
            "range": "0.000923-0.000923",
            "unit": "s",
            "extra": "Working out, against the adapter's accounting file, what became of each part of each source record."
          },
          {
            "name": "ccda-entries-x50/stage/dataset",
            "value": 0.000054,
            "range": "0.000054-0.000054",
            "unit": "s",
            "extra": "Putting the facts about a record into the store its queries run over."
          },
          {
            "name": "ccda-entries-x50/stage/index",
            "value": 0.064347,
            "range": "0.064347-0.064347",
            "unit": "s",
            "extra": "Building the lookup structure that compiled queries read, from a record's triples. A query the general evaluator answers needs none."
          },
          {
            "name": "ccda-entries-x50/stage/mapping",
            "value": 0.177506,
            "range": "0.177506-0.177506",
            "unit": "s",
            "extra": "Running the adapter's mapping queries on each record, without the time of building their lookup structure. (Specification stage: transform.)"
          },
          {
            "name": "ccda-entries-x50/stage/records",
            "value": 0.267099,
            "range": "0.267099-0.267099",
            "unit": "s",
            "extra": "Turning what the mapping produced into each record's graph: naming its versions, keeping one where the mapping wrote several, and writing their arrivals. (Specification stage: name versions, write arrivals.)"
          },
          {
            "name": "ccda-entries-x50/stage/output-validation",
            "value": 0.286376,
            "range": "0.286376-0.286376",
            "unit": "s",
            "extra": "Checking each record's output against the vocabulary's SHACL shapes."
          },
          {
            "name": "ccda-entries-x50/stage/findings",
            "value": 0.142469,
            "range": "0.142469-0.142469",
            "unit": "s",
            "extra": "Running the adapter's findings queries to find what the source record got wrong or left out. (Specification stage: findings.)"
          },
          {
            "name": "ccda-entries-x50/stage/unresolved-addresses",
            "value": 0.063561,
            "range": "0.063561-0.063561",
            "unit": "s",
            "extra": "Finding the references to records the document does not hold, which keep their computed names."
          },
          {
            "name": "ccda-entries-x50/stage/detect",
            "value": 0.006877,
            "range": "0.006877-0.006877",
            "unit": "s",
            "extra": "Running the adapter's detect query, which reports whether the adapter reads this document. The Bridge converts every record whatever the answer; it never routes."
          },
          {
            "name": "ccda-entries-x50/stage/summary",
            "value": 0.00232,
            "range": "0.00232-0.00232",
            "unit": "s",
            "extra": "Counting the graph's distinct triples and the findings once every record is done."
          },
          {
            "name": "ccda-entries-x50/stage/serialising",
            "value": 0.025553,
            "range": "0.025553-0.025553",
            "unit": "s",
            "extra": "Writing the graph and the findings out as Turtle or N-Triples."
          },
          {
            "name": "ccda-entries-x50/stage/unaccounted",
            "value": 0.050333,
            "range": "0.050333-0.050333",
            "unit": "s",
            "extra": "Seconds of the profiled convert that none of the stages accounts for, such as writing the document and the import into the graph."
          },
          {
            "name": "ccda-entries-x50/output/graph",
            "value": 9126,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph, with a SHA-256 fingerprint of the output that is the same whenever the content is the same, whatever order the engine wrote it in and whatever it named its blank nodes. sha256 139d803f26e008902033b2b0e3393b1c53073557b209e9222053dbf3e93d0bfc"
          },
          {
            "name": "ccda-entries-x50/output/findings",
            "value": 5890,
            "unit": "{count}",
            "extra": "Distinct triples in the findings, with a SHA-256 fingerprint of them that is the same whenever the content is the same, whatever order the engine wrote them in and whatever it named its blank nodes. sha256 ade58a19db77babdcde6b85475e8f4b509473641fe2efeb3c993ffc1f4807c6c"
          },
          {
            "name": "ccda-synthea-10y/convert/first",
            "value": 3.789126,
            "range": "3.76751-3.833753",
            "unit": "s",
            "extra": "Seconds the first convert of a fresh process takes, timed without the step-by-step breakdown. A caller that opens an adapter and converts once also waits for the engine to start (the boot rows) and the adapter to load (the load rows), which this does not count. Ten years of one Synthea person as a single C-CDA document. sha256 643c55e2e3db4e52974cdf202b7a9a2f0f1bcf72e47a64414e014ae8c1ccd361, 1226024 bytes"
          },
          {
            "name": "ccda-synthea-10y/convert/second",
            "value": 3.27765,
            "range": "3.265895-3.289532",
            "unit": "s",
            "extra": "Seconds a second convert of the same document takes in the same process, timed without the step-by-step breakdown: the warm cost."
          },
          {
            "name": "ccda-synthea-10y/convert/profiled",
            "value": 3.766002,
            "range": "3.766002-3.766002",
            "unit": "s",
            "extra": "Seconds of the one convert run with the step-by-step breakdown on, from which the stage and query rows are read. The breakdown makes it slower than a plain convert."
          },
          {
            "name": "ccda-synthea-10y/wasm-memory/first",
            "value": 237174784,
            "range": "237174784-237174784",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the first convert is done."
          },
          {
            "name": "ccda-synthea-10y/wasm-memory/second",
            "value": 237174784,
            "range": "237174784-237174784",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the second convert is done."
          },
          {
            "name": "ccda-synthea-10y/rss",
            "value": 486932480,
            "range": "429809664-510459904",
            "unit": "By",
            "extra": "Peak memory the whole process held in RAM, in bytes: the engine's start, the adapter's load and both converts all count."
          },
          {
            "name": "ccda-synthea-10y/records",
            "value": 1,
            "unit": "{count}",
            "extra": "Source records the Bridge found in the document and converted. Ten years of one Synthea person as a single C-CDA document."
          },
          {
            "name": "ccda-synthea-10y/triples",
            "value": 21772,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph."
          },
          {
            "name": "ccda-synthea-10y/findings",
            "value": 1234,
            "unit": "{count}",
            "extra": "Findings the conversion wrote."
          },
          {
            "name": "ccda-synthea-10y/query-time/document-table.rq",
            "value": 0.604937,
            "range": "0.604937-0.604937",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/allergy.rq",
            "value": 0.010796,
            "range": "0.010796-0.010796",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/condition.rq",
            "value": 0.017935,
            "range": "0.017935-0.017935",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/document-organizations.rq",
            "value": 0.000532,
            "range": "0.000532-0.000532",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/immunization.rq",
            "value": 0.006561,
            "range": "0.006561-0.006561",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/medication.rq",
            "value": 0.027304,
            "range": "0.027304-0.027304",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/note-or-document.rq",
            "value": 0.007499,
            "range": "0.007499-0.007499",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/patient.rq",
            "value": 0.000457,
            "range": "0.000457-0.000457",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/procedure.rq",
            "value": 0.017832,
            "range": "0.017832-0.017832",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/record.rq",
            "value": 0.038736,
            "range": "0.038736-0.038736",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/report.rq",
            "value": 0.016027,
            "range": "0.016027-0.016027",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/result-identifiers.rq",
            "value": 0.018645,
            "range": "0.018645-0.018645",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/result-quantities.rq",
            "value": 0.034106,
            "range": "0.034106-0.034106",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/result.rq",
            "value": 0.152902,
            "range": "0.152902-0.152902",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/allergy-findings.rq",
            "value": 0.006204,
            "range": "0.006204-0.006204",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/condition-findings.rq",
            "value": 0.00696,
            "range": "0.00696-0.00696",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/document-findings.rq",
            "value": 0.002007,
            "range": "0.002007-0.002007",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/entry-findings.rq",
            "value": 0.016173,
            "range": "0.016173-0.016173",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/entry-statement-findings.rq",
            "value": 0.015743,
            "range": "0.015743-0.015743",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/medication-findings.rq",
            "value": 0.007843,
            "range": "0.007843-0.007843",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/name-findings.rq",
            "value": 0.016914,
            "range": "0.016914-0.016914",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/narrative-findings.rq",
            "value": 0.060785,
            "range": "0.060785-0.060785",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/organization-findings.rq",
            "value": 0.000607,
            "range": "0.000607-0.000607",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/patient-findings.rq",
            "value": 0.000273,
            "range": "0.000273-0.000273",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/reference-findings.rq",
            "value": 0.0283,
            "range": "0.0283-0.0283",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/report-and-note-findings.rq",
            "value": 0.007384,
            "range": "0.007384-0.007384",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/result-findings.rq",
            "value": 0.145727,
            "range": "0.145727-0.145727",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/section-findings.rq",
            "value": 0.003889,
            "range": "0.003889-0.003889",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/detect.rq",
            "value": 0.006692,
            "range": "0.006692-0.006692",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/stage/document-table",
            "value": 0.628545,
            "range": "0.628545-0.628545",
            "unit": "s",
            "extra": "Running the adapter's document-table query on every record of the document, to link a record to any other record, in the document or not. (Specification stage: link records.)"
          },
          {
            "name": "ccda-synthea-10y/stage/lift",
            "value": 0.414285,
            "range": "0.414285-0.414285",
            "unit": "s",
            "extra": "Reading the source document into the triples the adapter's queries can ask about, one record at a time. (Specification stage: read and chunk.)"
          },
          {
            "name": "ccda-synthea-10y/stage/source-validation",
            "value": 0.142632,
            "range": "0.142632-0.142632",
            "unit": "s",
            "extra": "Checking each source record, and the document as a whole where the adapter gives a document schema, against the adapter's schema (XSD or JSON Schema). A failure is a finding, never a refusal. (Specification stage: check, validate.)"
          },
          {
            "name": "ccda-synthea-10y/stage/accounting",
            "value": 0.002324,
            "range": "0.002324-0.002324",
            "unit": "s",
            "extra": "Working out, against the adapter's accounting file, what became of each part of each source record."
          },
          {
            "name": "ccda-synthea-10y/stage/dataset",
            "value": 0.000083,
            "range": "0.000083-0.000083",
            "unit": "s",
            "extra": "Putting the facts about a record into the store its queries run over."
          },
          {
            "name": "ccda-synthea-10y/stage/index",
            "value": 0.169624,
            "range": "0.169624-0.169624",
            "unit": "s",
            "extra": "Building the lookup structure that compiled queries read, from a record's triples. A query the general evaluator answers needs none."
          },
          {
            "name": "ccda-synthea-10y/stage/mapping",
            "value": 0.351762,
            "range": "0.351762-0.351762",
            "unit": "s",
            "extra": "Running the adapter's mapping queries on each record, without the time of building their lookup structure. (Specification stage: transform.)"
          },
          {
            "name": "ccda-synthea-10y/stage/records",
            "value": 0.847036,
            "range": "0.847036-0.847036",
            "unit": "s",
            "extra": "Turning what the mapping produced into each record's graph: naming its versions, keeping one where the mapping wrote several, and writing their arrivals. (Specification stage: name versions, write arrivals.)"
          },
          {
            "name": "ccda-synthea-10y/stage/output-validation",
            "value": 0.581757,
            "range": "0.581757-0.581757",
            "unit": "s",
            "extra": "Checking each record's output against the vocabulary's SHACL shapes."
          },
          {
            "name": "ccda-synthea-10y/stage/findings",
            "value": 0.340052,
            "range": "0.340052-0.340052",
            "unit": "s",
            "extra": "Running the adapter's findings queries to find what the source record got wrong or left out. (Specification stage: findings.)"
          },
          {
            "name": "ccda-synthea-10y/stage/unresolved-addresses",
            "value": 0.136424,
            "range": "0.136424-0.136424",
            "unit": "s",
            "extra": "Finding the references to records the document does not hold, which keep their computed names."
          },
          {
            "name": "ccda-synthea-10y/stage/detect",
            "value": 0.006738,
            "range": "0.006738-0.006738",
            "unit": "s",
            "extra": "Running the adapter's detect query, which reports whether the adapter reads this document. The Bridge converts every record whatever the answer; it never routes."
          },
          {
            "name": "ccda-synthea-10y/stage/summary",
            "value": 0.005584,
            "range": "0.005584-0.005584",
            "unit": "s",
            "extra": "Counting the graph's distinct triples and the findings once every record is done."
          },
          {
            "name": "ccda-synthea-10y/stage/serialising",
            "value": 0.066035,
            "range": "0.066035-0.066035",
            "unit": "s",
            "extra": "Writing the graph and the findings out as Turtle or N-Triples."
          },
          {
            "name": "ccda-synthea-10y/stage/unaccounted",
            "value": 0.073122,
            "range": "0.073122-0.073122",
            "unit": "s",
            "extra": "Seconds of the profiled convert that none of the stages accounts for, such as writing the document and the import into the graph."
          },
          {
            "name": "ccda-synthea-10y/output/graph",
            "value": 21772,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph, with a SHA-256 fingerprint of the output that is the same whenever the content is the same, whatever order the engine wrote it in and whatever it named its blank nodes. sha256 4c682c9d7746d30c8e5fd69e73f2e02378d0d2906267d583be3e5c817fd47817"
          },
          {
            "name": "ccda-synthea-10y/output/findings",
            "value": 16083,
            "unit": "{count}",
            "extra": "Distinct triples in the findings, with a SHA-256 fingerprint of them that is the same whenever the content is the same, whatever order the engine wrote them in and whatever it named its blank nodes. sha256 65c70b4b78caf2e112089b8b759422c88c7e53903bf481200f38011bb40dcb8c"
          },
          {
            "name": "ccda/load",
            "value": 0.909499,
            "range": "0.874838-1.166627",
            "unit": "s",
            "extra": "Seconds to read the adapter's and the vocabulary's files, describe the adapter and prepare it, with the engine already started."
          },
          {
            "name": "ccda/load/read-files",
            "value": 0.006873,
            "range": "0.006282-0.012916",
            "unit": "s",
            "extra": "Seconds of the load spent reading files from disk."
          },
          {
            "name": "ccda/load/prepare",
            "value": 0.903217,
            "range": "0.868353-1.159103",
            "unit": "s",
            "extra": "Seconds of the load spent describing the adapter and preparing it: parsing and compiling its queries, schemas and shapes."
          },
          {
            "name": "ccda/boot/read-wasm",
            "value": 0.00442,
            "range": "0.004282-0.004782",
            "unit": "s",
            "extra": "Seconds to read the engine's compiled code from disk."
          },
          {
            "name": "ccda/boot/compile-wasm",
            "value": 0.02217,
            "range": "0.021796-0.0273",
            "unit": "s",
            "extra": "Seconds to turn the engine's compiled code into something the machine can run."
          },
          {
            "name": "ccda/boot/instantiate-wasm",
            "value": 0.001429,
            "range": "0.001347-0.003082",
            "unit": "s",
            "extra": "Seconds to start the compiled code, which runs its start-up work."
          },
          {
            "name": "fhir-r4-lab-pull-3000/convert/first",
            "value": 7.720955,
            "range": "7.714942-7.871157",
            "unit": "s",
            "extra": "Seconds the first convert of a fresh process takes, timed without the step-by-step breakdown. A caller that opens an adapter and converts once also waits for the engine to start (the boot rows) and the adapter to load (the load rows), which this does not count. A pull of 3,000 lab results in one FHIR R4 searchset Bundle. sha256 69b4daac45fd32449b8c290be9a7551566b537a6171218e8125169e94ca5882b, 2760035 bytes"
          },
          {
            "name": "fhir-r4-lab-pull-3000/convert/second",
            "value": 7.322634,
            "range": "7.282309-7.443235",
            "unit": "s",
            "extra": "Seconds a second convert of the same document takes in the same process, timed without the step-by-step breakdown: the warm cost."
          },
          {
            "name": "fhir-r4-lab-pull-3000/convert/profiled",
            "value": 7.796597,
            "range": "7.796597-7.796597",
            "unit": "s",
            "extra": "Seconds of the one convert run with the step-by-step breakdown on, from which the stage and query rows are read. The breakdown makes it slower than a plain convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/wasm-memory/first",
            "value": 136511488,
            "range": "136511488-136511488",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the first convert is done."
          },
          {
            "name": "fhir-r4-lab-pull-3000/wasm-memory/second",
            "value": 136511488,
            "range": "136511488-136511488",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the second convert is done."
          },
          {
            "name": "fhir-r4-lab-pull-3000/rss",
            "value": 369438720,
            "range": "366366720-389758976",
            "unit": "By",
            "extra": "Peak memory the whole process held in RAM, in bytes: the engine's start, the adapter's load and both converts all count."
          },
          {
            "name": "fhir-r4-lab-pull-3000/records",
            "value": 3004,
            "unit": "{count}",
            "extra": "Source records the Bridge found in the document and converted. A pull of 3,000 lab results in one FHIR R4 searchset Bundle."
          },
          {
            "name": "fhir-r4-lab-pull-3000/triples",
            "value": 63025,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph."
          },
          {
            "name": "fhir-r4-lab-pull-3000/findings",
            "value": 3006,
            "unit": "{count}",
            "extra": "Findings the conversion wrote."
          },
          {
            "name": "fhir-r4-lab-pull-3000/fallbacks",
            "value": 0,
            "unit": "{count}",
            "extra": "Record executions the general evaluator answered because a query was not compiled or a compiled plan gave up. The detect query, which runs once, is not counted."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/coding-system-table.rq",
            "value": 0.075494,
            "range": "0.075494-0.075494",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/document-table.rq",
            "value": 0.011163,
            "range": "0.011163-0.011163",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/lab-result-table.rq",
            "value": 0.022146,
            "range": "0.022146-0.022146",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/medication-table.rq",
            "value": 0.01191,
            "range": "0.01191-0.01191",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/note-or-document-table.rq",
            "value": 0.005942,
            "range": "0.005942-0.005942",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/organization-table.rq",
            "value": 0.024019,
            "range": "0.024019-0.024019",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/variant-interpretation-table.rq",
            "value": 0.008511,
            "range": "0.008511-0.008511",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/variant-table.rq",
            "value": 0.009325,
            "range": "0.009325-0.009325",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/absent-reasons.rq",
            "value": 0.074746,
            "range": "0.074746-0.074746",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/allergy-intolerance.rq",
            "value": 0.01126,
            "range": "0.01126-0.01126",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/attachments.rq",
            "value": 0.013453,
            "range": "0.013453-0.013453",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/condition.rq",
            "value": 0.010086,
            "range": "0.010086-0.010086",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/immunization.rq",
            "value": 0.009199,
            "range": "0.009199-0.009199",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/lab-result-codes.rq",
            "value": 0.119565,
            "range": "0.119565-0.119565",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/lab-result-identifiers.rq",
            "value": 0.026178,
            "range": "0.026178-0.026178",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/lab-result-performer.rq",
            "value": 0.004194,
            "range": "0.004194-0.004194",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/lab-result-quantities.rq",
            "value": 0.139987,
            "range": "0.139987-0.139987",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/lab-result.rq",
            "value": 0.139111,
            "range": "0.139111-0.139111",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/medication-dispense-prescription.rq",
            "value": 0.013718,
            "range": "0.013718-0.013718",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/medication-dispense.rq",
            "value": 0.008233,
            "range": "0.008233-0.008233",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/medication-supply-and-dosages.rq",
            "value": 0.010968,
            "range": "0.010968-0.010968",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/medication.rq",
            "value": 0.010346,
            "range": "0.010346-0.010346",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/narrative-text.rq",
            "value": 0.035667,
            "range": "0.035667-0.035667",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/note-or-document.rq",
            "value": 0.008788,
            "range": "0.008788-0.008788",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/organization-from-server.rq",
            "value": 0.006106,
            "range": "0.006106-0.006106",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/organization.rq",
            "value": 0.014357,
            "range": "0.014357-0.014357",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/patient.rq",
            "value": 0.009161,
            "range": "0.009161-0.009161",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/procedure.rq",
            "value": 0.0081,
            "range": "0.0081-0.0081",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/record.rq",
            "value": 0.334653,
            "range": "0.334653-0.334653",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/report-and-note-performer.rq",
            "value": 0.004728,
            "range": "0.004728-0.004728",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/report-results.rq",
            "value": 0.012351,
            "range": "0.012351-0.012351",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/report.rq",
            "value": 0.008809,
            "range": "0.008809-0.008809",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/variant-interpretation.rq",
            "value": 0.012246,
            "range": "0.012246-0.012246",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/variant.rq",
            "value": 0.010358,
            "range": "0.010358-0.010358",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/absent-reason-findings.rq",
            "value": 0.04634,
            "range": "0.04634-0.04634",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/category-text-findings.rq",
            "value": 0.015537,
            "range": "0.015537-0.015537",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/code-findings.rq",
            "value": 0.01017,
            "range": "0.01017-0.01017",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/date-findings.rq",
            "value": 0.050089,
            "range": "0.050089-0.050089",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/document-version-findings.rq",
            "value": 0.00164,
            "range": "0.00164-0.00164",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/lab-result-findings.rq",
            "value": 0.095564,
            "range": "0.095564-0.095564",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/medication-dispense-findings.rq",
            "value": 0.010225,
            "range": "0.010225-0.010225",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/medication-dispense-prescription-findings.rq",
            "value": 0.012458,
            "range": "0.012458-0.012458",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/medication-findings.rq",
            "value": 0.078956,
            "range": "0.078956-0.078956",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/narrative-findings.rq",
            "value": 0.008512,
            "range": "0.008512-0.008512",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/observation-component-findings.rq",
            "value": 0.011742,
            "range": "0.011742-0.011742",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/observation-findings.rq",
            "value": 0.076769,
            "range": "0.076769-0.076769",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/organization-findings.rq",
            "value": 0.009423,
            "range": "0.009423-0.009423",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/patient-findings.rq",
            "value": 0.008025,
            "range": "0.008025-0.008025",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/patient-reference-findings.rq",
            "value": 0.067961,
            "range": "0.067961-0.067961",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/recorded-date-findings.rq",
            "value": 0.009996,
            "range": "0.009996-0.009996",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/reference-display-findings.rq",
            "value": 0.037414,
            "range": "0.037414-0.037414",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/reference-without-server-findings.rq",
            "value": 0.044947,
            "range": "0.044947-0.044947",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/repeated-id-findings.rq",
            "value": 0.028979,
            "range": "0.028979-0.028979",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/report-and-note-findings.rq",
            "value": 0.013586,
            "range": "0.013586-0.013586",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/report-result-findings.rq",
            "value": 0.012298,
            "range": "0.012298-0.012298",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/resource-type-findings.rq",
            "value": 0.011231,
            "range": "0.011231-0.011231",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/variant-findings.rq",
            "value": 0.021765,
            "range": "0.021765-0.021765",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/variant-interpretation-findings.rq",
            "value": 0.015034,
            "range": "0.015034-0.015034",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/detect.rq",
            "value": 0.009455,
            "range": "0.009455-0.009455",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/document-table",
            "value": 0.217157,
            "range": "0.217157-0.217157",
            "unit": "s",
            "extra": "Running the adapter's document-table query on every record of the document, to link a record to any other record, in the document or not. (Specification stage: link records.)"
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/lift",
            "value": 0.444809,
            "range": "0.444809-0.444809",
            "unit": "s",
            "extra": "Reading the source document into the triples the adapter's queries can ask about, one record at a time. (Specification stage: read and chunk.)"
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/source-validation",
            "value": 1.040331,
            "range": "1.040331-1.040331",
            "unit": "s",
            "extra": "Checking each source record, and the document as a whole where the adapter gives a document schema, against the adapter's schema (XSD or JSON Schema). A failure is a finding, never a refusal. (Specification stage: check, validate.)"
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/accounting",
            "value": 0.019713,
            "range": "0.019713-0.019713",
            "unit": "s",
            "extra": "Working out, against the adapter's accounting file, what became of each part of each source record."
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/dataset",
            "value": 0.010795,
            "range": "0.010795-0.010795",
            "unit": "s",
            "extra": "Putting the facts about a record into the store its queries run over."
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/index",
            "value": 0.244721,
            "range": "0.244721-0.244721",
            "unit": "s",
            "extra": "Building the lookup structure that compiled queries read, from a record's triples. A query the general evaluator answers needs none."
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/mapping",
            "value": 1.09213,
            "range": "1.09213-1.09213",
            "unit": "s",
            "extra": "Running the adapter's mapping queries on each record, without the time of building their lookup structure. (Specification stage: transform.)"
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/records",
            "value": 0.276458,
            "range": "0.276458-0.276458",
            "unit": "s",
            "extra": "Turning what the mapping produced into each record's graph: naming its versions, keeping one where the mapping wrote several, and writing their arrivals. (Specification stage: name versions, write arrivals.)"
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/output-validation",
            "value": 3.117016,
            "range": "3.117016-3.117016",
            "unit": "s",
            "extra": "Checking each record's output against the vocabulary's SHACL shapes."
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/findings",
            "value": 0.911416,
            "range": "0.911416-0.911416",
            "unit": "s",
            "extra": "Running the adapter's findings queries to find what the source record got wrong or left out. (Specification stage: findings.)"
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/unresolved-addresses",
            "value": 0.045723,
            "range": "0.045723-0.045723",
            "unit": "s",
            "extra": "Finding the references to records the document does not hold, which keep their computed names."
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/detect",
            "value": 0.009562,
            "range": "0.009562-0.009562",
            "unit": "s",
            "extra": "Running the adapter's detect query, which reports whether the adapter reads this document. The Bridge converts every record whatever the answer; it never routes."
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/summary",
            "value": 0.015521,
            "range": "0.015521-0.015521",
            "unit": "s",
            "extra": "Counting the graph's distinct triples and the findings once every record is done."
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/serialising",
            "value": 0.155692,
            "range": "0.155692-0.155692",
            "unit": "s",
            "extra": "Writing the graph and the findings out as Turtle or N-Triples."
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/unaccounted",
            "value": 0.195554,
            "range": "0.195554-0.195554",
            "unit": "s",
            "extra": "Seconds of the profiled convert that none of the stages accounts for, such as writing the document and the import into the graph."
          },
          {
            "name": "fhir-r4-lab-pull-3000/output/graph",
            "value": 63025,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph, with a SHA-256 fingerprint of the output that is the same whenever the content is the same, whatever order the engine wrote it in and whatever it named its blank nodes. sha256 37ddd3ff45eedcca686486482a5dd0ea90120d791c04352fb96a41e20d5fe7ac"
          },
          {
            "name": "fhir-r4-lab-pull-3000/output/findings",
            "value": 45090,
            "unit": "{count}",
            "extra": "Distinct triples in the findings, with a SHA-256 fingerprint of them that is the same whenever the content is the same, whatever order the engine wrote them in and whatever it named its blank nodes. sha256 b882dcb20e82c494692121aa741018d60fa8b958d52919c0a6eca58735b69721"
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/convert/first",
            "value": 2.468185,
            "range": "2.4623-2.483494",
            "unit": "s",
            "extra": "Seconds the first convert of a fresh process takes, timed without the step-by-step breakdown. A caller that opens an adapter and converts once also waits for the engine to start (the boot rows) and the adapter to load (the load rows), which this does not count. The trimmed ten-year Synthea record of the demo hospitals, as a FHIR R4 Bundle. sha256 2703dc1e7e920fa1e45d16c58f8338ef13b3cc1916ee40ffa461112d54dce5de, 1098812 bytes"
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/convert/second",
            "value": 2.150153,
            "range": "2.148354-2.167503",
            "unit": "s",
            "extra": "Seconds a second convert of the same document takes in the same process, timed without the step-by-step breakdown: the warm cost."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/convert/profiled",
            "value": 2.4938,
            "range": "2.4938-2.4938",
            "unit": "s",
            "extra": "Seconds of the one convert run with the step-by-step breakdown on, from which the stage and query rows are read. The breakdown makes it slower than a plain convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/wasm-memory/first",
            "value": 86179840,
            "range": "86179840-86179840",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the first convert is done."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/wasm-memory/second",
            "value": 86179840,
            "range": "86179840-86179840",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the second convert is done."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/rss",
            "value": 364613632,
            "range": "351236096-370065408",
            "unit": "By",
            "extra": "Peak memory the whole process held in RAM, in bytes: the engine's start, the adapter's load and both converts all count."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/records",
            "value": 647,
            "unit": "{count}",
            "extra": "Source records the Bridge found in the document and converted. The trimmed ten-year Synthea record of the demo hospitals, as a FHIR R4 Bundle."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/triples",
            "value": 12654,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/findings",
            "value": 2256,
            "unit": "{count}",
            "extra": "Findings the conversion wrote."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/coding-system-table.rq",
            "value": 0.023983,
            "range": "0.023983-0.023983",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/document-table.rq",
            "value": 0.003527,
            "range": "0.003527-0.003527",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/lab-result-table.rq",
            "value": 0.005364,
            "range": "0.005364-0.005364",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/medication-table.rq",
            "value": 0.00355,
            "range": "0.00355-0.00355",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/note-or-document-table.rq",
            "value": 0.001806,
            "range": "0.001806-0.001806",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/organization-table.rq",
            "value": 0.005455,
            "range": "0.005455-0.005455",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/variant-interpretation-table.rq",
            "value": 0.002428,
            "range": "0.002428-0.002428",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/variant-table.rq",
            "value": 0.002548,
            "range": "0.002548-0.002548",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/absent-reasons.rq",
            "value": 0.019498,
            "range": "0.019498-0.019498",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/allergy-intolerance.rq",
            "value": 0.003196,
            "range": "0.003196-0.003196",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/attachments.rq",
            "value": 0.003106,
            "range": "0.003106-0.003106",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/condition.rq",
            "value": 0.007567,
            "range": "0.007567-0.007567",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/immunization.rq",
            "value": 0.002503,
            "range": "0.002503-0.002503",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/lab-result-codes.rq",
            "value": 0.018657,
            "range": "0.018657-0.018657",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/lab-result-identifiers.rq",
            "value": 0.004692,
            "range": "0.004692-0.004692",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/lab-result-performer.rq",
            "value": 0.001019,
            "range": "0.001019-0.001019",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/lab-result-quantities.rq",
            "value": 0.019503,
            "range": "0.019503-0.019503",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/lab-result.rq",
            "value": 0.023236,
            "range": "0.023236-0.023236",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/medication-dispense-prescription.rq",
            "value": 0.00318,
            "range": "0.00318-0.00318",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/medication-dispense.rq",
            "value": 0.001936,
            "range": "0.001936-0.001936",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/medication-supply-and-dosages.rq",
            "value": 0.008519,
            "range": "0.008519-0.008519",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/medication.rq",
            "value": 0.008335,
            "range": "0.008335-0.008335",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/narrative-text.rq",
            "value": 0.009606,
            "range": "0.009606-0.009606",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/note-or-document.rq",
            "value": 0.002214,
            "range": "0.002214-0.002214",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/organization-from-server.rq",
            "value": 0.001439,
            "range": "0.001439-0.001439",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/organization.rq",
            "value": 0.003366,
            "range": "0.003366-0.003366",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/patient.rq",
            "value": 0.002452,
            "range": "0.002452-0.002452",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/procedure.rq",
            "value": 0.00493,
            "range": "0.00493-0.00493",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/record.rq",
            "value": 0.077985,
            "range": "0.077985-0.077985",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/report-and-note-performer.rq",
            "value": 0.001111,
            "range": "0.001111-0.001111",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/report-results.rq",
            "value": 0.002969,
            "range": "0.002969-0.002969",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/report.rq",
            "value": 0.002044,
            "range": "0.002044-0.002044",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/variant-interpretation.rq",
            "value": 0.002726,
            "range": "0.002726-0.002726",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/variant.rq",
            "value": 0.00224,
            "range": "0.00224-0.00224",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/absent-reason-findings.rq",
            "value": 0.012807,
            "range": "0.012807-0.012807",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/category-text-findings.rq",
            "value": 0.004186,
            "range": "0.004186-0.004186",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/code-findings.rq",
            "value": 0.003362,
            "range": "0.003362-0.003362",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/date-findings.rq",
            "value": 0.015679,
            "range": "0.015679-0.015679",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/document-version-findings.rq",
            "value": 0.000416,
            "range": "0.000416-0.000416",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/lab-result-findings.rq",
            "value": 0.020566,
            "range": "0.020566-0.020566",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/medication-dispense-findings.rq",
            "value": 0.002197,
            "range": "0.002197-0.002197",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/medication-dispense-prescription-findings.rq",
            "value": 0.003878,
            "range": "0.003878-0.003878",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/medication-findings.rq",
            "value": 0.026619,
            "range": "0.026619-0.026619",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/narrative-findings.rq",
            "value": 0.002046,
            "range": "0.002046-0.002046",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/observation-component-findings.rq",
            "value": 0.003379,
            "range": "0.003379-0.003379",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/observation-findings.rq",
            "value": 0.013211,
            "range": "0.013211-0.013211",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/organization-findings.rq",
            "value": 0.002015,
            "range": "0.002015-0.002015",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/patient-findings.rq",
            "value": 0.001849,
            "range": "0.001849-0.001849",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/patient-reference-findings.rq",
            "value": 0.015491,
            "range": "0.015491-0.015491",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/recorded-date-findings.rq",
            "value": 0.002722,
            "range": "0.002722-0.002722",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/reference-display-findings.rq",
            "value": 0.008918,
            "range": "0.008918-0.008918",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/reference-without-server-findings.rq",
            "value": 0.010042,
            "range": "0.010042-0.010042",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/repeated-id-findings.rq",
            "value": 0.005714,
            "range": "0.005714-0.005714",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/report-and-note-findings.rq",
            "value": 0.003104,
            "range": "0.003104-0.003104",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/report-result-findings.rq",
            "value": 0.002795,
            "range": "0.002795-0.002795",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/resource-type-findings.rq",
            "value": 0.002767,
            "range": "0.002767-0.002767",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/variant-findings.rq",
            "value": 0.004344,
            "range": "0.004344-0.004344",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/variant-interpretation-findings.rq",
            "value": 0.002649,
            "range": "0.002649-0.002649",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/detect.rq",
            "value": 0.009227,
            "range": "0.009227-0.009227",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/stage/document-table",
            "value": 0.063073,
            "range": "0.063073-0.063073",
            "unit": "s",
            "extra": "Running the adapter's document-table query on every record of the document, to link a record to any other record, in the document or not. (Specification stage: link records.)"
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/stage/lift",
            "value": 0.146715,
            "range": "0.146715-0.146715",
            "unit": "s",
            "extra": "Reading the source document into the triples the adapter's queries can ask about, one record at a time. (Specification stage: read and chunk.)"
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/stage/source-validation",
            "value": 0.314347,
            "range": "0.314347-0.314347",
            "unit": "s",
            "extra": "Checking each source record, and the document as a whole where the adapter gives a document schema, against the adapter's schema (XSD or JSON Schema). A failure is a finding, never a refusal. (Specification stage: check, validate.)"
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/stage/accounting",
            "value": 0.025553,
            "range": "0.025553-0.025553",
            "unit": "s",
            "extra": "Working out, against the adapter's accounting file, what became of each part of each source record."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/stage/dataset",
            "value": 0.003086,
            "range": "0.003086-0.003086",
            "unit": "s",
            "extra": "Putting the facts about a record into the store its queries run over."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/stage/index",
            "value": 0.075349,
            "range": "0.075349-0.075349",
            "unit": "s",
            "extra": "Building the lookup structure that compiled queries read, from a record's triples. A query the general evaluator answers needs none."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/stage/mapping",
            "value": 0.24657,
            "range": "0.24657-0.24657",
            "unit": "s",
            "extra": "Running the adapter's mapping queries on each record, without the time of building their lookup structure. (Specification stage: transform.)"
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/stage/records",
            "value": 0.503184,
            "range": "0.503184-0.503184",
            "unit": "s",
            "extra": "Turning what the mapping produced into each record's graph: naming its versions, keeping one where the mapping wrote several, and writing their arrivals. (Specification stage: name versions, write arrivals.)"
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/stage/output-validation",
            "value": 0.722867,
            "range": "0.722867-0.722867",
            "unit": "s",
            "extra": "Checking each record's output against the vocabulary's SHACL shapes."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/stage/findings",
            "value": 0.226747,
            "range": "0.226747-0.226747",
            "unit": "s",
            "extra": "Running the adapter's findings queries to find what the source record got wrong or left out. (Specification stage: findings.)"
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/stage/unresolved-addresses",
            "value": 0.012673,
            "range": "0.012673-0.012673",
            "unit": "s",
            "extra": "Finding the references to records the document does not hold, which keep their computed names."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/stage/detect",
            "value": 0.0093,
            "range": "0.0093-0.0093",
            "unit": "s",
            "extra": "Running the adapter's detect query, which reports whether the adapter reads this document. The Bridge converts every record whatever the answer; it never routes."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/stage/summary",
            "value": 0.003854,
            "range": "0.003854-0.003854",
            "unit": "s",
            "extra": "Counting the graph's distinct triples and the findings once every record is done."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/stage/serialising",
            "value": 0.058774,
            "range": "0.058774-0.058774",
            "unit": "s",
            "extra": "Writing the graph and the findings out as Turtle or N-Triples."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/stage/unaccounted",
            "value": 0.081706,
            "range": "0.081706-0.081706",
            "unit": "s",
            "extra": "Seconds of the profiled convert that none of the stages accounts for, such as writing the document and the import into the graph."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/output/graph",
            "value": 12654,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph, with a SHA-256 fingerprint of the output that is the same whenever the content is the same, whatever order the engine wrote it in and whatever it named its blank nodes. sha256 7cc449ffbde5128e52fafcf8acfb64362df9b08f54ac053668923208f8175856"
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/output/findings",
            "value": 33875,
            "unit": "{count}",
            "extra": "Distinct triples in the findings, with a SHA-256 fingerprint of them that is the same whenever the content is the same, whatever order the engine wrote them in and whatever it named its blank nodes. sha256 d537a811bedd885d56ef2486e522f7087e751130f23262dcfd5a6a287d47e7de"
          },
          {
            "name": "fhir-r4-synthea-lifetime/convert/first",
            "value": 59.810181,
            "range": "58.609247-60.148722",
            "unit": "s",
            "extra": "Seconds the first convert of a fresh process takes, timed without the step-by-step breakdown. A caller that opens an adapter and converts once also waits for the engine to start (the boot rows) and the adapter to load (the load rows), which this does not count. The lifetime Synthea record of one person, a 51 MB FHIR R4 Bundle. sha256 f20b86408066ece2d25a7b02d380b7652f22d16df9dab613a0c73c9969b17da8, 51227791 bytes"
          },
          {
            "name": "fhir-r4-synthea-lifetime/convert/second",
            "value": 56.087344,
            "range": "55.570295-57.108641",
            "unit": "s",
            "extra": "Seconds a second convert of the same document takes in the same process, timed without the step-by-step breakdown: the warm cost."
          },
          {
            "name": "fhir-r4-synthea-lifetime/convert/profiled",
            "value": 59.783077,
            "range": "59.783077-59.783077",
            "unit": "s",
            "extra": "Seconds of the one convert run with the step-by-step breakdown on, from which the stage and query rows are read. The breakdown makes it slower than a plain convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/wasm-memory/first",
            "value": 2518876160,
            "range": "2518876160-2518876160",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the first convert is done."
          },
          {
            "name": "fhir-r4-synthea-lifetime/wasm-memory/second",
            "value": 2518876160,
            "range": "2518876160-2518876160",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the second convert is done."
          },
          {
            "name": "fhir-r4-synthea-lifetime/rss",
            "value": 3358216192,
            "range": "3351719936-3360739328",
            "unit": "By",
            "extra": "Peak memory the whole process held in RAM, in bytes: the engine's start, the adapter's load and both converts all count."
          },
          {
            "name": "fhir-r4-synthea-lifetime/records",
            "value": 16833,
            "unit": "{count}",
            "extra": "Source records the Bridge found in the document and converted. The lifetime Synthea record of one person, a 51 MB FHIR R4 Bundle."
          },
          {
            "name": "fhir-r4-synthea-lifetime/triples",
            "value": 184332,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph."
          },
          {
            "name": "fhir-r4-synthea-lifetime/findings",
            "value": 267199,
            "unit": "{count}",
            "extra": "Findings the conversion wrote."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/coding-system-table.rq",
            "value": 0.588229,
            "range": "0.588229-0.588229",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/document-table.rq",
            "value": 0.058268,
            "range": "0.058268-0.058268",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/lab-result-table.rq",
            "value": 0.068349,
            "range": "0.068349-0.068349",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/medication-table.rq",
            "value": 0.087152,
            "range": "0.087152-0.087152",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/note-or-document-table.rq",
            "value": 0.037615,
            "range": "0.037615-0.037615",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/organization-table.rq",
            "value": 0.095838,
            "range": "0.095838-0.095838",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/variant-interpretation-table.rq",
            "value": 0.037069,
            "range": "0.037069-0.037069",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/variant-table.rq",
            "value": 0.037238,
            "range": "0.037238-0.037238",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/absent-reasons.rq",
            "value": 0.511777,
            "range": "0.511777-0.511777",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/allergy-intolerance.rq",
            "value": 0.079709,
            "range": "0.079709-0.079709",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/attachments.rq",
            "value": 0.127663,
            "range": "0.127663-0.127663",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/condition.rq",
            "value": 0.084806,
            "range": "0.084806-0.084806",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/immunization.rq",
            "value": 0.065176,
            "range": "0.065176-0.065176",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/lab-result-codes.rq",
            "value": 0.219137,
            "range": "0.219137-0.219137",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/lab-result-identifiers.rq",
            "value": 0.081707,
            "range": "0.081707-0.081707",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/lab-result-performer.rq",
            "value": 0.024162,
            "range": "0.024162-0.024162",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/lab-result-quantities.rq",
            "value": 0.209291,
            "range": "0.209291-0.209291",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/lab-result.rq",
            "value": 0.255582,
            "range": "0.255582-0.255582",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/medication-dispense-prescription.rq",
            "value": 0.077296,
            "range": "0.077296-0.077296",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/medication-dispense.rq",
            "value": 0.051711,
            "range": "0.051711-0.051711",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/medication-supply-and-dosages.rq",
            "value": 0.08562,
            "range": "0.08562-0.08562",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/medication.rq",
            "value": 0.140578,
            "range": "0.140578-0.140578",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/narrative-text.rq",
            "value": 0.476868,
            "range": "0.476868-0.476868",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/note-or-document.rq",
            "value": 0.153312,
            "range": "0.153312-0.153312",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/organization-from-server.rq",
            "value": 0.036567,
            "range": "0.036567-0.036567",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/organization.rq",
            "value": 0.089783,
            "range": "0.089783-0.089783",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/patient.rq",
            "value": 0.055021,
            "range": "0.055021-0.055021",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/procedure.rq",
            "value": 0.156386,
            "range": "0.156386-0.156386",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/record.rq",
            "value": 1.292364,
            "range": "1.292364-1.292364",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/report-and-note-performer.rq",
            "value": 0.026284,
            "range": "0.026284-0.026284",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/report-results.rq",
            "value": 0.201551,
            "range": "0.201551-0.201551",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/report.rq",
            "value": 0.203204,
            "range": "0.203204-0.203204",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/variant-interpretation.rq",
            "value": 0.067244,
            "range": "0.067244-0.067244",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/variant.rq",
            "value": 0.05795,
            "range": "0.05795-0.05795",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/absent-reason-findings.rq",
            "value": 0.223016,
            "range": "0.223016-0.223016",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/category-text-findings.rq",
            "value": 0.099898,
            "range": "0.099898-0.099898",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/code-findings.rq",
            "value": 0.104228,
            "range": "0.104228-0.104228",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/date-findings.rq",
            "value": 0.203836,
            "range": "0.203836-0.203836",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/document-version-findings.rq",
            "value": 0.009717,
            "range": "0.009717-0.009717",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/lab-result-findings.rq",
            "value": 0.308399,
            "range": "0.308399-0.308399",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/medication-dispense-findings.rq",
            "value": 0.053815,
            "range": "0.053815-0.053815",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/medication-dispense-prescription-findings.rq",
            "value": 0.069843,
            "range": "0.069843-0.069843",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/medication-findings.rq",
            "value": 0.743524,
            "range": "0.743524-0.743524",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/narrative-findings.rq",
            "value": 0.054494,
            "range": "0.054494-0.054494",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/observation-component-findings.rq",
            "value": 0.094854,
            "range": "0.094854-0.094854",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/observation-findings.rq",
            "value": 0.222382,
            "range": "0.222382-0.222382",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/organization-findings.rq",
            "value": 0.053119,
            "range": "0.053119-0.053119",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/patient-findings.rq",
            "value": 0.050391,
            "range": "0.050391-0.050391",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/patient-reference-findings.rq",
            "value": 0.222142,
            "range": "0.222142-0.222142",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/recorded-date-findings.rq",
            "value": 0.059782,
            "range": "0.059782-0.059782",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/reference-display-findings.rq",
            "value": 0.235681,
            "range": "0.235681-0.235681",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/reference-without-server-findings.rq",
            "value": 0.099178,
            "range": "0.099178-0.099178",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/repeated-id-findings.rq",
            "value": 0.104673,
            "range": "0.104673-0.104673",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/report-and-note-findings.rq",
            "value": 0.13426,
            "range": "0.13426-0.13426",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/report-result-findings.rq",
            "value": 0.106887,
            "range": "0.106887-0.106887",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/resource-type-findings.rq",
            "value": 0.942869,
            "range": "0.942869-0.942869",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/variant-findings.rq",
            "value": 0.094327,
            "range": "0.094327-0.094327",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/variant-interpretation-findings.rq",
            "value": 0.070359,
            "range": "0.070359-0.070359",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/detect.rq",
            "value": 0.009316,
            "range": "0.009316-0.009316",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/stage/document-table",
            "value": 1.456204,
            "range": "1.456204-1.456204",
            "unit": "s",
            "extra": "Running the adapter's document-table query on every record of the document, to link a record to any other record, in the document or not. (Specification stage: link records.)"
          },
          {
            "name": "fhir-r4-synthea-lifetime/stage/lift",
            "value": 5.100124,
            "range": "5.100124-5.100124",
            "unit": "s",
            "extra": "Reading the source document into the triples the adapter's queries can ask about, one record at a time. (Specification stage: read and chunk.)"
          },
          {
            "name": "fhir-r4-synthea-lifetime/stage/source-validation",
            "value": 8.621649,
            "range": "8.621649-8.621649",
            "unit": "s",
            "extra": "Checking each source record, and the document as a whole where the adapter gives a document schema, against the adapter's schema (XSD or JSON Schema). A failure is a finding, never a refusal. (Specification stage: check, validate.)"
          },
          {
            "name": "fhir-r4-synthea-lifetime/stage/accounting",
            "value": 2.565526,
            "range": "2.565526-2.565526",
            "unit": "s",
            "extra": "Working out, against the adapter's accounting file, what became of each part of each source record."
          },
          {
            "name": "fhir-r4-synthea-lifetime/stage/dataset",
            "value": 0.067738,
            "range": "0.067738-0.067738",
            "unit": "s",
            "extra": "Putting the facts about a record into the store its queries run over."
          },
          {
            "name": "fhir-r4-synthea-lifetime/stage/index",
            "value": 2.525195,
            "range": "2.525195-2.525195",
            "unit": "s",
            "extra": "Building the lookup structure that compiled queries read, from a record's triples. A query the general evaluator answers needs none."
          },
          {
            "name": "fhir-r4-synthea-lifetime/stage/mapping",
            "value": 5.009989,
            "range": "5.009989-5.009989",
            "unit": "s",
            "extra": "Running the adapter's mapping queries on each record, without the time of building their lookup structure. (Specification stage: transform.)"
          },
          {
            "name": "fhir-r4-synthea-lifetime/stage/records",
            "value": 0.982312,
            "range": "0.982312-0.982312",
            "unit": "s",
            "extra": "Turning what the mapping produced into each record's graph: naming its versions, keeping one where the mapping wrote several, and writing their arrivals. (Specification stage: name versions, write arrivals.)"
          },
          {
            "name": "fhir-r4-synthea-lifetime/stage/output-validation",
            "value": 16.007665,
            "range": "16.007665-16.007665",
            "unit": "s",
            "extra": "Checking each record's output against the vocabulary's SHACL shapes."
          },
          {
            "name": "fhir-r4-synthea-lifetime/stage/findings",
            "value": 5.565129,
            "range": "5.565129-5.565129",
            "unit": "s",
            "extra": "Running the adapter's findings queries to find what the source record got wrong or left out. (Specification stage: findings.)"
          },
          {
            "name": "fhir-r4-synthea-lifetime/stage/unresolved-addresses",
            "value": 0.342663,
            "range": "0.342663-0.342663",
            "unit": "s",
            "extra": "Finding the references to records the document does not hold, which keep their computed names."
          },
          {
            "name": "fhir-r4-synthea-lifetime/stage/detect",
            "value": 0.00944,
            "range": "0.00944-0.00944",
            "unit": "s",
            "extra": "Running the adapter's detect query, which reports whether the adapter reads this document. The Bridge converts every record whatever the answer; it never routes."
          },
          {
            "name": "fhir-r4-synthea-lifetime/stage/summary",
            "value": 0.129478,
            "range": "0.129478-0.129478",
            "unit": "s",
            "extra": "Counting the graph's distinct triples and the findings once every record is done."
          },
          {
            "name": "fhir-r4-synthea-lifetime/stage/serialising",
            "value": 8.011097,
            "range": "8.011097-8.011097",
            "unit": "s",
            "extra": "Writing the graph and the findings out as Turtle or N-Triples."
          },
          {
            "name": "fhir-r4-synthea-lifetime/stage/unaccounted",
            "value": 3.388867,
            "range": "3.388867-3.388867",
            "unit": "s",
            "extra": "Seconds of the profiled convert that none of the stages accounts for, such as writing the document and the import into the graph."
          },
          {
            "name": "fhir-r4-synthea-lifetime/output/graph",
            "value": 184332,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph, with a SHA-256 fingerprint of the output that is the same whenever the content is the same, whatever order the engine wrote it in and whatever it named its blank nodes. sha256 7a293b50f329e9697119ef46a49e0e55365198126021106ead2739fd58656a51"
          },
          {
            "name": "fhir-r4-synthea-lifetime/output/findings",
            "value": 4050084,
            "unit": "{count}",
            "extra": "Distinct triples in the findings, with a SHA-256 fingerprint of them that is the same whenever the content is the same, whatever order the engine wrote them in and whatever it named its blank nodes. sha256 ff18a1907da595ae57df30a0e8d2e3562de67ab8261c8799e7e8b02259945069"
          },
          {
            "name": "fhir-r4/load",
            "value": 0.893028,
            "range": "0.877515-0.907542",
            "unit": "s",
            "extra": "Seconds to read the adapter's and the vocabulary's files, describe the adapter and prepare it, with the engine already started."
          },
          {
            "name": "fhir-r4/load/read-files",
            "value": 0.011933,
            "range": "0.0108-0.018426",
            "unit": "s",
            "extra": "Seconds of the load spent reading files from disk."
          },
          {
            "name": "fhir-r4/load/prepare",
            "value": 0.880799,
            "range": "0.86568-0.895755",
            "unit": "s",
            "extra": "Seconds of the load spent describing the adapter and preparing it: parsing and compiling its queries, schemas and shapes."
          },
          {
            "name": "fhir-r4/boot/read-wasm",
            "value": 0.004414,
            "range": "0.004185-0.004869",
            "unit": "s",
            "extra": "Seconds to read the engine's compiled code from disk."
          },
          {
            "name": "fhir-r4/boot/compile-wasm",
            "value": 0.021965,
            "range": "0.021598-0.022085",
            "unit": "s",
            "extra": "Seconds to turn the engine's compiled code into something the machine can run."
          },
          {
            "name": "fhir-r4/boot/instantiate-wasm",
            "value": 0.00145,
            "range": "0.001318-0.00164",
            "unit": "s",
            "extra": "Seconds to start the compiled code, which runs its start-up work."
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "jayostis@gmail.com",
            "name": "jayostis",
            "username": "jayostis"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f5528c5ef4da326e8212110e5a61996015d53910",
          "message": "Merge pull request #155 from jayostis/feat/wasm-function-names\n\nfeat(package): keep the wasm's function names in a second module, so a CPU profile names the Rust functions",
          "timestamp": "2026-11-20T09:48:17-08:00",
          "tree_id": "8ca387d87fab9126c86cff19068af2f49808edc4",
          "url": "https://github.com/jayostis/cascade-bridge-rs/commit/f5528c5ef4da326e8212110e5a61996015d53910"
        },
        "date": 1795197091111,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "ccda-entries-x10/convert/first",
            "value": 0.536745,
            "range": "0.532404-0.547754",
            "unit": "s",
            "extra": "Seconds the first convert of a fresh process takes, timed without the step-by-step breakdown. A caller that opens an adapter and converts once also waits for the engine to start (the boot rows) and the adapter to load (the load rows), which this does not count. The C-CDA medications-and-results example with its entries repeated 10 times, as one document. sha256 77454794ba3cf02cf329e249f7a4a1874ca58cd0761d4f5622163ea9e85efa0e, 138124 bytes"
          },
          {
            "name": "ccda-entries-x10/convert/second",
            "value": 0.290043,
            "range": "0.2877-0.302073",
            "unit": "s",
            "extra": "Seconds a second convert of the same document takes in the same process, timed without the step-by-step breakdown: the warm cost."
          },
          {
            "name": "ccda-entries-x10/convert/profiled",
            "value": 0.540058,
            "range": "0.540058-0.540058",
            "unit": "s",
            "extra": "Seconds of the one convert run with the step-by-step breakdown on, from which the stage and query rows are read. The breakdown makes it slower than a plain convert."
          },
          {
            "name": "ccda-entries-x10/wasm-memory/first",
            "value": 52625408,
            "range": "52625408-52625408",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the first convert is done."
          },
          {
            "name": "ccda-entries-x10/wasm-memory/second",
            "value": 52625408,
            "range": "52625408-52625408",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the second convert is done."
          },
          {
            "name": "ccda-entries-x10/rss",
            "value": 342892544,
            "range": "338042880-360644608",
            "unit": "By",
            "extra": "Peak memory the whole process held in RAM, in bytes: the engine's start, the adapter's load and both converts all count."
          },
          {
            "name": "ccda-entries-x10/records",
            "value": 1,
            "unit": "{count}",
            "extra": "Source records the Bridge found in the document and converted. The C-CDA medications-and-results example with its entries repeated 10 times, as one document."
          },
          {
            "name": "ccda-entries-x10/triples",
            "value": 1926,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph."
          },
          {
            "name": "ccda-entries-x10/findings",
            "value": 93,
            "unit": "{count}",
            "extra": "Findings the conversion wrote."
          },
          {
            "name": "ccda-entries-x10/query-time/document-table.rq",
            "value": 0.098713,
            "range": "0.098713-0.098713",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/allergy.rq",
            "value": 0.000431,
            "range": "0.000431-0.000431",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/condition.rq",
            "value": 0.000389,
            "range": "0.000389-0.000389",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/document-organizations.rq",
            "value": 0.001478,
            "range": "0.001478-0.001478",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/immunization.rq",
            "value": 0.000888,
            "range": "0.000888-0.000888",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/medication.rq",
            "value": 0.016844,
            "range": "0.016844-0.016844",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/note-or-document.rq",
            "value": 0.002615,
            "range": "0.002615-0.002615",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/patient.rq",
            "value": 0.000301,
            "range": "0.000301-0.000301",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/procedure.rq",
            "value": 0.000896,
            "range": "0.000896-0.000896",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/record.rq",
            "value": 0.007786,
            "range": "0.007786-0.007786",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/report.rq",
            "value": 0.004955,
            "range": "0.004955-0.004955",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/result-identifiers.rq",
            "value": 0.002205,
            "range": "0.002205-0.002205",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/result-quantities.rq",
            "value": 0.004749,
            "range": "0.004749-0.004749",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/result.rq",
            "value": 0.016579,
            "range": "0.016579-0.016579",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/allergy-findings.rq",
            "value": 0.000324,
            "range": "0.000324-0.000324",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/condition-findings.rq",
            "value": 0.000669,
            "range": "0.000669-0.000669",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/document-findings.rq",
            "value": 0.000682,
            "range": "0.000682-0.000682",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/entry-findings.rq",
            "value": 0.003672,
            "range": "0.003672-0.003672",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/entry-statement-findings.rq",
            "value": 0.003983,
            "range": "0.003983-0.003983",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/medication-findings.rq",
            "value": 0.002114,
            "range": "0.002114-0.002114",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/name-findings.rq",
            "value": 0.00099,
            "range": "0.00099-0.00099",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/narrative-findings.rq",
            "value": 0.002585,
            "range": "0.002585-0.002585",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/organization-findings.rq",
            "value": 0.000458,
            "range": "0.000458-0.000458",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/patient-findings.rq",
            "value": 0.00008,
            "range": "0.00008-0.00008",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/reference-findings.rq",
            "value": 0.003103,
            "range": "0.003103-0.003103",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/report-and-note-findings.rq",
            "value": 0.001844,
            "range": "0.001844-0.001844",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/result-findings.rq",
            "value": 0.016605,
            "range": "0.016605-0.016605",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/section-findings.rq",
            "value": 0.000944,
            "range": "0.000944-0.000944",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/detect.rq",
            "value": 0.010524,
            "range": "0.010524-0.010524",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/stage/document-table",
            "value": 0.101158,
            "range": "0.101158-0.101158",
            "unit": "s",
            "extra": "Running the adapter's document-table query on every record of the document, to link a record to any other record, in the document or not. (Specification stage: link records.)"
          },
          {
            "name": "ccda-entries-x10/stage/lift",
            "value": 0.081293,
            "range": "0.081293-0.081293",
            "unit": "s",
            "extra": "Reading the source document into the triples the adapter's queries can ask about, one record at a time. (Specification stage: read and chunk.)"
          },
          {
            "name": "ccda-entries-x10/stage/source-validation",
            "value": 0.033444,
            "range": "0.033444-0.033444",
            "unit": "s",
            "extra": "Checking each source record, and the document as a whole where the adapter gives a document schema, against the adapter's schema (XSD or JSON Schema). A failure is a finding, never a refusal. (Specification stage: check, validate.)"
          },
          {
            "name": "ccda-entries-x10/stage/accounting",
            "value": 0.001363,
            "range": "0.001363-0.001363",
            "unit": "s",
            "extra": "Working out, against the adapter's accounting file, what became of each part of each source record."
          },
          {
            "name": "ccda-entries-x10/stage/dataset",
            "value": 0.000059,
            "range": "0.000059-0.000059",
            "unit": "s",
            "extra": "Putting the facts about a record into the store its queries run over."
          },
          {
            "name": "ccda-entries-x10/stage/index",
            "value": 0.028094,
            "range": "0.028094-0.028094",
            "unit": "s",
            "extra": "Building the lookup structure that compiled queries read, from a record's triples. A query the general evaluator answers needs none."
          },
          {
            "name": "ccda-entries-x10/stage/mapping",
            "value": 0.060872,
            "range": "0.060872-0.060872",
            "unit": "s",
            "extra": "Running the adapter's mapping queries on each record, without the time of building their lookup structure. (Specification stage: transform.)"
          },
          {
            "name": "ccda-entries-x10/stage/records",
            "value": 0.033259,
            "range": "0.033259-0.033259",
            "unit": "s",
            "extra": "Turning what the mapping produced into each record's graph: naming its versions, keeping one where the mapping wrote several, and writing their arrivals. (Specification stage: name versions, write arrivals.)"
          },
          {
            "name": "ccda-entries-x10/stage/output-validation",
            "value": 0.098418,
            "range": "0.098418-0.098418",
            "unit": "s",
            "extra": "Checking each record's output against the vocabulary's SHACL shapes."
          },
          {
            "name": "ccda-entries-x10/stage/findings",
            "value": 0.043241,
            "range": "0.043241-0.043241",
            "unit": "s",
            "extra": "Running the adapter's findings queries to find what the source record got wrong or left out. (Specification stage: findings.)"
          },
          {
            "name": "ccda-entries-x10/stage/unresolved-addresses",
            "value": 0.018213,
            "range": "0.018213-0.018213",
            "unit": "s",
            "extra": "Finding the references to records the document does not hold, which keep their computed names."
          },
          {
            "name": "ccda-entries-x10/stage/detect",
            "value": 0.010585,
            "range": "0.010585-0.010585",
            "unit": "s",
            "extra": "Running the adapter's detect query, which reports whether the adapter reads this document. The Bridge converts every record whatever the answer; it never routes."
          },
          {
            "name": "ccda-entries-x10/stage/summary",
            "value": 0.001134,
            "range": "0.001134-0.001134",
            "unit": "s",
            "extra": "Counting the graph's distinct triples and the findings once every record is done."
          },
          {
            "name": "ccda-entries-x10/stage/serialising",
            "value": 0.012134,
            "range": "0.012134-0.012134",
            "unit": "s",
            "extra": "Writing the graph and the findings out as Turtle or N-Triples."
          },
          {
            "name": "ccda-entries-x10/stage/unaccounted",
            "value": 0.016791,
            "range": "0.016791-0.016791",
            "unit": "s",
            "extra": "Seconds of the profiled convert that none of the stages accounts for, such as writing the document and the import into the graph."
          },
          {
            "name": "ccda-entries-x10/output/graph",
            "value": 1926,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph, with a SHA-256 fingerprint of the output that is the same whenever the content is the same, whatever order the engine wrote it in and whatever it named its blank nodes. sha256 ad5fa27c4fe8b29ebed8c8892c3bf3df74f202365f5e14dca5a7981d130dfc16"
          },
          {
            "name": "ccda-entries-x10/output/findings",
            "value": 1210,
            "unit": "{count}",
            "extra": "Distinct triples in the findings, with a SHA-256 fingerprint of them that is the same whenever the content is the same, whatever order the engine wrote them in and whatever it named its blank nodes. sha256 95da111564c5677296616ba1bec9f3340f1788949bbde806f616a5ca56b4ba0a"
          },
          {
            "name": "ccda/load",
            "value": 0.932123,
            "range": "0.915307-0.958434",
            "unit": "s",
            "extra": "Seconds to read the adapter's and the vocabulary's files, describe the adapter and prepare it, with the engine already started."
          },
          {
            "name": "ccda/load/read-files",
            "value": 0.008399,
            "range": "0.007285-0.008434",
            "unit": "s",
            "extra": "Seconds of the load spent reading files from disk."
          },
          {
            "name": "ccda/load/prepare",
            "value": 0.923725,
            "range": "0.906873-0.951149",
            "unit": "s",
            "extra": "Seconds of the load spent describing the adapter and preparing it: parsing and compiling its queries, schemas and shapes."
          },
          {
            "name": "ccda/boot/read-wasm",
            "value": 0.007458,
            "range": "0.007432-0.007816",
            "unit": "s",
            "extra": "Seconds to read the engine's compiled code from disk."
          },
          {
            "name": "ccda/boot/compile-wasm",
            "value": 0.021547,
            "range": "0.021189-0.024137",
            "unit": "s",
            "extra": "Seconds to turn the engine's compiled code into something the machine can run."
          },
          {
            "name": "ccda/boot/instantiate-wasm",
            "value": 0.001294,
            "range": "0.001253-0.002903",
            "unit": "s",
            "extra": "Seconds to start the compiled code, which runs its start-up work."
          },
          {
            "name": "fhir-r4-lab-pull-3000/convert/first",
            "value": 7.839007,
            "range": "7.822716-7.924855",
            "unit": "s",
            "extra": "Seconds the first convert of a fresh process takes, timed without the step-by-step breakdown. A caller that opens an adapter and converts once also waits for the engine to start (the boot rows) and the adapter to load (the load rows), which this does not count. A pull of 3,000 lab results in one FHIR R4 searchset Bundle. sha256 69b4daac45fd32449b8c290be9a7551566b537a6171218e8125169e94ca5882b, 2760035 bytes"
          },
          {
            "name": "fhir-r4-lab-pull-3000/convert/second",
            "value": 7.392124,
            "range": "7.374807-7.401132",
            "unit": "s",
            "extra": "Seconds a second convert of the same document takes in the same process, timed without the step-by-step breakdown: the warm cost."
          },
          {
            "name": "fhir-r4-lab-pull-3000/convert/profiled",
            "value": 7.952234,
            "range": "7.952234-7.952234",
            "unit": "s",
            "extra": "Seconds of the one convert run with the step-by-step breakdown on, from which the stage and query rows are read. The breakdown makes it slower than a plain convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/wasm-memory/first",
            "value": 136511488,
            "range": "136511488-136511488",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the first convert is done."
          },
          {
            "name": "fhir-r4-lab-pull-3000/wasm-memory/second",
            "value": 136511488,
            "range": "136511488-136511488",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the second convert is done."
          },
          {
            "name": "fhir-r4-lab-pull-3000/rss",
            "value": 388886528,
            "range": "384462848-437276672",
            "unit": "By",
            "extra": "Peak memory the whole process held in RAM, in bytes: the engine's start, the adapter's load and both converts all count."
          },
          {
            "name": "fhir-r4-lab-pull-3000/records",
            "value": 3004,
            "unit": "{count}",
            "extra": "Source records the Bridge found in the document and converted. A pull of 3,000 lab results in one FHIR R4 searchset Bundle."
          },
          {
            "name": "fhir-r4-lab-pull-3000/triples",
            "value": 63025,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph."
          },
          {
            "name": "fhir-r4-lab-pull-3000/findings",
            "value": 3006,
            "unit": "{count}",
            "extra": "Findings the conversion wrote."
          },
          {
            "name": "fhir-r4-lab-pull-3000/fallbacks",
            "value": 0,
            "unit": "{count}",
            "extra": "Record executions the general evaluator answered because a query was not compiled or a compiled plan gave up. The detect query, which runs once, is not counted."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/coding-system-table.rq",
            "value": 0.081434,
            "range": "0.081434-0.081434",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/document-table.rq",
            "value": 0.012628,
            "range": "0.012628-0.012628",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/lab-result-table.rq",
            "value": 0.024901,
            "range": "0.024901-0.024901",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/medication-table.rq",
            "value": 0.01359,
            "range": "0.01359-0.01359",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/note-or-document-table.rq",
            "value": 0.00701,
            "range": "0.00701-0.00701",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/organization-table.rq",
            "value": 0.023778,
            "range": "0.023778-0.023778",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/variant-interpretation-table.rq",
            "value": 0.010542,
            "range": "0.010542-0.010542",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/variant-table.rq",
            "value": 0.011405,
            "range": "0.011405-0.011405",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/absent-reasons.rq",
            "value": 0.069207,
            "range": "0.069207-0.069207",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/allergy-intolerance.rq",
            "value": 0.011073,
            "range": "0.011073-0.011073",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/attachments.rq",
            "value": 0.012342,
            "range": "0.012342-0.012342",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/condition.rq",
            "value": 0.009206,
            "range": "0.009206-0.009206",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/immunization.rq",
            "value": 0.008915,
            "range": "0.008915-0.008915",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/lab-result-codes.rq",
            "value": 0.112592,
            "range": "0.112592-0.112592",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/lab-result-identifiers.rq",
            "value": 0.026422,
            "range": "0.026422-0.026422",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/lab-result-performer.rq",
            "value": 0.004268,
            "range": "0.004268-0.004268",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/lab-result-quantities.rq",
            "value": 0.126049,
            "range": "0.126049-0.126049",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/lab-result.rq",
            "value": 0.132719,
            "range": "0.132719-0.132719",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/medication-dispense-prescription.rq",
            "value": 0.013535,
            "range": "0.013535-0.013535",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/medication-dispense.rq",
            "value": 0.008643,
            "range": "0.008643-0.008643",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/medication-supply-and-dosages.rq",
            "value": 0.010913,
            "range": "0.010913-0.010913",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/medication.rq",
            "value": 0.010352,
            "range": "0.010352-0.010352",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/narrative-text.rq",
            "value": 0.03497,
            "range": "0.03497-0.03497",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/note-or-document.rq",
            "value": 0.00873,
            "range": "0.00873-0.00873",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/organization-from-server.rq",
            "value": 0.005728,
            "range": "0.005728-0.005728",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/organization.rq",
            "value": 0.013972,
            "range": "0.013972-0.013972",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/patient.rq",
            "value": 0.009526,
            "range": "0.009526-0.009526",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/procedure.rq",
            "value": 0.008089,
            "range": "0.008089-0.008089",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/record.rq",
            "value": 0.328717,
            "range": "0.328717-0.328717",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/report-and-note-performer.rq",
            "value": 0.004827,
            "range": "0.004827-0.004827",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/report-results.rq",
            "value": 0.013555,
            "range": "0.013555-0.013555",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/report.rq",
            "value": 0.009161,
            "range": "0.009161-0.009161",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/variant-interpretation.rq",
            "value": 0.012171,
            "range": "0.012171-0.012171",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/variant.rq",
            "value": 0.010675,
            "range": "0.010675-0.010675",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/absent-reason-findings.rq",
            "value": 0.045706,
            "range": "0.045706-0.045706",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/category-text-findings.rq",
            "value": 0.014204,
            "range": "0.014204-0.014204",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/code-findings.rq",
            "value": 0.009817,
            "range": "0.009817-0.009817",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/date-findings.rq",
            "value": 0.045642,
            "range": "0.045642-0.045642",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/document-version-findings.rq",
            "value": 0.001526,
            "range": "0.001526-0.001526",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/lab-result-findings.rq",
            "value": 0.088996,
            "range": "0.088996-0.088996",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/medication-dispense-findings.rq",
            "value": 0.01042,
            "range": "0.01042-0.01042",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/medication-dispense-prescription-findings.rq",
            "value": 0.011784,
            "range": "0.011784-0.011784",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/medication-findings.rq",
            "value": 0.0754,
            "range": "0.0754-0.0754",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/narrative-findings.rq",
            "value": 0.007671,
            "range": "0.007671-0.007671",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/observation-component-findings.rq",
            "value": 0.011903,
            "range": "0.011903-0.011903",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/observation-findings.rq",
            "value": 0.072678,
            "range": "0.072678-0.072678",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/organization-findings.rq",
            "value": 0.00977,
            "range": "0.00977-0.00977",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/patient-findings.rq",
            "value": 0.009788,
            "range": "0.009788-0.009788",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/patient-reference-findings.rq",
            "value": 0.059688,
            "range": "0.059688-0.059688",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/recorded-date-findings.rq",
            "value": 0.01028,
            "range": "0.01028-0.01028",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/reference-display-findings.rq",
            "value": 0.040177,
            "range": "0.040177-0.040177",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/reference-without-server-findings.rq",
            "value": 0.047068,
            "range": "0.047068-0.047068",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/repeated-id-findings.rq",
            "value": 0.028675,
            "range": "0.028675-0.028675",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/report-and-note-findings.rq",
            "value": 0.013533,
            "range": "0.013533-0.013533",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/report-result-findings.rq",
            "value": 0.012486,
            "range": "0.012486-0.012486",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/resource-type-findings.rq",
            "value": 0.011582,
            "range": "0.011582-0.011582",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/variant-findings.rq",
            "value": 0.022314,
            "range": "0.022314-0.022314",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/variant-interpretation-findings.rq",
            "value": 0.012459,
            "range": "0.012459-0.012459",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/detect.rq",
            "value": 0.009572,
            "range": "0.009572-0.009572",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/document-table",
            "value": 0.233403,
            "range": "0.233403-0.233403",
            "unit": "s",
            "extra": "Running the adapter's document-table query on every record of the document, to link a record to any other record, in the document or not. (Specification stage: link records.)"
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/lift",
            "value": 0.467554,
            "range": "0.467554-0.467554",
            "unit": "s",
            "extra": "Reading the source document into the triples the adapter's queries can ask about, one record at a time. (Specification stage: read and chunk.)"
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/source-validation",
            "value": 1.144597,
            "range": "1.144597-1.144597",
            "unit": "s",
            "extra": "Checking each source record, and the document as a whole where the adapter gives a document schema, against the adapter's schema (XSD or JSON Schema). A failure is a finding, never a refusal. (Specification stage: check, validate.)"
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/accounting",
            "value": 0.016177,
            "range": "0.016177-0.016177",
            "unit": "s",
            "extra": "Working out, against the adapter's accounting file, what became of each part of each source record."
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/dataset",
            "value": 0.011373,
            "range": "0.011373-0.011373",
            "unit": "s",
            "extra": "Putting the facts about a record into the store its queries run over."
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/index",
            "value": 0.254236,
            "range": "0.254236-0.254236",
            "unit": "s",
            "extra": "Building the lookup structure that compiled queries read, from a record's triples. A query the general evaluator answers needs none."
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/mapping",
            "value": 1.054528,
            "range": "1.054528-1.054528",
            "unit": "s",
            "extra": "Running the adapter's mapping queries on each record, without the time of building their lookup structure. (Specification stage: transform.)"
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/records",
            "value": 0.257237,
            "range": "0.257237-0.257237",
            "unit": "s",
            "extra": "Turning what the mapping produced into each record's graph: naming its versions, keeping one where the mapping wrote several, and writing their arrivals. (Specification stage: name versions, write arrivals.)"
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/output-validation",
            "value": 3.124826,
            "range": "3.124826-3.124826",
            "unit": "s",
            "extra": "Checking each record's output against the vocabulary's SHACL shapes."
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/findings",
            "value": 0.895773,
            "range": "0.895773-0.895773",
            "unit": "s",
            "extra": "Running the adapter's findings queries to find what the source record got wrong or left out. (Specification stage: findings.)"
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/unresolved-addresses",
            "value": 0.039654,
            "range": "0.039654-0.039654",
            "unit": "s",
            "extra": "Finding the references to records the document does not hold, which keep their computed names."
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/detect",
            "value": 0.009653,
            "range": "0.009653-0.009653",
            "unit": "s",
            "extra": "Running the adapter's detect query, which reports whether the adapter reads this document. The Bridge converts every record whatever the answer; it never routes."
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/summary",
            "value": 0.016815,
            "range": "0.016815-0.016815",
            "unit": "s",
            "extra": "Counting the graph's distinct triples and the findings once every record is done."
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/serialising",
            "value": 0.179401,
            "range": "0.179401-0.179401",
            "unit": "s",
            "extra": "Writing the graph and the findings out as Turtle or N-Triples."
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/unaccounted",
            "value": 0.247006,
            "range": "0.247006-0.247006",
            "unit": "s",
            "extra": "Seconds of the profiled convert that none of the stages accounts for, such as writing the document and the import into the graph."
          },
          {
            "name": "fhir-r4-lab-pull-3000/output/graph",
            "value": 63025,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph, with a SHA-256 fingerprint of the output that is the same whenever the content is the same, whatever order the engine wrote it in and whatever it named its blank nodes. sha256 37ddd3ff45eedcca686486482a5dd0ea90120d791c04352fb96a41e20d5fe7ac"
          },
          {
            "name": "fhir-r4-lab-pull-3000/output/findings",
            "value": 45090,
            "unit": "{count}",
            "extra": "Distinct triples in the findings, with a SHA-256 fingerprint of them that is the same whenever the content is the same, whatever order the engine wrote them in and whatever it named its blank nodes. sha256 b882dcb20e82c494692121aa741018d60fa8b958d52919c0a6eca58735b69721"
          },
          {
            "name": "fhir-r4/load",
            "value": 0.915418,
            "range": "0.911282-0.940271",
            "unit": "s",
            "extra": "Seconds to read the adapter's and the vocabulary's files, describe the adapter and prepare it, with the engine already started."
          },
          {
            "name": "fhir-r4/load/read-files",
            "value": 0.012124,
            "range": "0.010822-0.018105",
            "unit": "s",
            "extra": "Seconds of the load spent reading files from disk."
          },
          {
            "name": "fhir-r4/load/prepare",
            "value": 0.904596,
            "range": "0.899158-0.922166",
            "unit": "s",
            "extra": "Seconds of the load spent describing the adapter and preparing it: parsing and compiling its queries, schemas and shapes."
          },
          {
            "name": "fhir-r4/boot/read-wasm",
            "value": 0.007465,
            "range": "0.007366-0.007716",
            "unit": "s",
            "extra": "Seconds to read the engine's compiled code from disk."
          },
          {
            "name": "fhir-r4/boot/compile-wasm",
            "value": 0.021565,
            "range": "0.021287-0.021582",
            "unit": "s",
            "extra": "Seconds to turn the engine's compiled code into something the machine can run."
          },
          {
            "name": "fhir-r4/boot/instantiate-wasm",
            "value": 0.001273,
            "range": "0.001233-0.001293",
            "unit": "s",
            "extra": "Seconds to start the compiled code, which runs its start-up work."
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "jayostis",
            "username": "jayostis",
            "email": "jayostis@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "f5528c5ef4da326e8212110e5a61996015d53910",
          "message": "Merge pull request #155 from jayostis/feat/wasm-function-names\n\nfeat(package): keep the wasm's function names in a second module, so a CPU profile names the Rust functions",
          "timestamp": "2026-11-20T17:48:17Z",
          "url": "https://github.com/the-cascade-protocol/cascade-bridge-rs/commit/f5528c5ef4da326e8212110e5a61996015d53910"
        },
        "date": 1795234302148,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "ccda-entries-x10/convert/first",
            "value": 0.509255,
            "range": "0.498997-0.529454",
            "unit": "s",
            "extra": "Seconds the first convert of a fresh process takes, timed without the step-by-step breakdown. A caller that opens an adapter and converts once also waits for the engine to start (the boot rows) and the adapter to load (the load rows), which this does not count. The C-CDA medications-and-results example with its entries repeated 10 times, as one document. sha256 77454794ba3cf02cf329e249f7a4a1874ca58cd0761d4f5622163ea9e85efa0e, 138124 bytes"
          },
          {
            "name": "ccda-entries-x10/convert/second",
            "value": 0.265642,
            "range": "0.26259-0.274311",
            "unit": "s",
            "extra": "Seconds a second convert of the same document takes in the same process, timed without the step-by-step breakdown: the warm cost."
          },
          {
            "name": "ccda-entries-x10/convert/profiled",
            "value": 0.511804,
            "range": "0.511804-0.511804",
            "unit": "s",
            "extra": "Seconds of the one convert run with the step-by-step breakdown on, from which the stage and query rows are read. The breakdown makes it slower than a plain convert."
          },
          {
            "name": "ccda-entries-x10/wasm-memory/first",
            "value": 52625408,
            "range": "52625408-52625408",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the first convert is done."
          },
          {
            "name": "ccda-entries-x10/wasm-memory/second",
            "value": 52625408,
            "range": "52625408-52625408",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the second convert is done."
          },
          {
            "name": "ccda-entries-x10/rss",
            "value": 348942336,
            "range": "340758528-353673216",
            "unit": "By",
            "extra": "Peak memory the whole process held in RAM, in bytes: the engine's start, the adapter's load and both converts all count."
          },
          {
            "name": "ccda-entries-x10/records",
            "value": 1,
            "unit": "{count}",
            "extra": "Source records the Bridge found in the document and converted. The C-CDA medications-and-results example with its entries repeated 10 times, as one document."
          },
          {
            "name": "ccda-entries-x10/triples",
            "value": 1926,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph."
          },
          {
            "name": "ccda-entries-x10/findings",
            "value": 93,
            "unit": "{count}",
            "extra": "Findings the conversion wrote."
          },
          {
            "name": "ccda-entries-x10/query-time/document-table.rq",
            "value": 0.104057,
            "range": "0.104057-0.104057",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/allergy.rq",
            "value": 0.000381,
            "range": "0.000381-0.000381",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/condition.rq",
            "value": 0.000359,
            "range": "0.000359-0.000359",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/document-organizations.rq",
            "value": 0.001437,
            "range": "0.001437-0.001437",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/immunization.rq",
            "value": 0.000785,
            "range": "0.000785-0.000785",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/medication.rq",
            "value": 0.015515,
            "range": "0.015515-0.015515",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/note-or-document.rq",
            "value": 0.002348,
            "range": "0.002348-0.002348",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/patient.rq",
            "value": 0.000331,
            "range": "0.000331-0.000331",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/procedure.rq",
            "value": 0.000809,
            "range": "0.000809-0.000809",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/record.rq",
            "value": 0.007387,
            "range": "0.007387-0.007387",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/report.rq",
            "value": 0.005093,
            "range": "0.005093-0.005093",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/result-identifiers.rq",
            "value": 0.002766,
            "range": "0.002766-0.002766",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/result-quantities.rq",
            "value": 0.005478,
            "range": "0.005478-0.005478",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/result.rq",
            "value": 0.01364,
            "range": "0.01364-0.01364",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/allergy-findings.rq",
            "value": 0.000448,
            "range": "0.000448-0.000448",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/condition-findings.rq",
            "value": 0.000837,
            "range": "0.000837-0.000837",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/document-findings.rq",
            "value": 0.000694,
            "range": "0.000694-0.000694",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/entry-findings.rq",
            "value": 0.00326,
            "range": "0.00326-0.00326",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/entry-statement-findings.rq",
            "value": 0.004628,
            "range": "0.004628-0.004628",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/medication-findings.rq",
            "value": 0.001778,
            "range": "0.001778-0.001778",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/name-findings.rq",
            "value": 0.000792,
            "range": "0.000792-0.000792",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/narrative-findings.rq",
            "value": 0.002294,
            "range": "0.002294-0.002294",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/organization-findings.rq",
            "value": 0.00044,
            "range": "0.00044-0.00044",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/patient-findings.rq",
            "value": 0.000079,
            "range": "0.000079-0.000079",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/reference-findings.rq",
            "value": 0.002378,
            "range": "0.002378-0.002378",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/report-and-note-findings.rq",
            "value": 0.000871,
            "range": "0.000871-0.000871",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/result-findings.rq",
            "value": 0.013767,
            "range": "0.013767-0.013767",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/section-findings.rq",
            "value": 0.00078,
            "range": "0.00078-0.00078",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/query-time/detect.rq",
            "value": 0.010206,
            "range": "0.010206-0.010206",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x10/stage/document-table",
            "value": 0.106667,
            "range": "0.106667-0.106667",
            "unit": "s",
            "extra": "Running the adapter's document-table query on every record of the document, to link a record to any other record, in the document or not. (Specification stage: link records.)"
          },
          {
            "name": "ccda-entries-x10/stage/lift",
            "value": 0.066488,
            "range": "0.066488-0.066488",
            "unit": "s",
            "extra": "Reading the source document into the triples the adapter's queries can ask about, one record at a time. (Specification stage: read and chunk.)"
          },
          {
            "name": "ccda-entries-x10/stage/source-validation",
            "value": 0.030274,
            "range": "0.030274-0.030274",
            "unit": "s",
            "extra": "Checking each source record, and the document as a whole where the adapter gives a document schema, against the adapter's schema (XSD or JSON Schema). A failure is a finding, never a refusal. (Specification stage: check, validate.)"
          },
          {
            "name": "ccda-entries-x10/stage/accounting",
            "value": 0.001332,
            "range": "0.001332-0.001332",
            "unit": "s",
            "extra": "Working out, against the adapter's accounting file, what became of each part of each source record."
          },
          {
            "name": "ccda-entries-x10/stage/dataset",
            "value": 0.000088,
            "range": "0.000088-0.000088",
            "unit": "s",
            "extra": "Putting the facts about a record into the store its queries run over."
          },
          {
            "name": "ccda-entries-x10/stage/index",
            "value": 0.026224,
            "range": "0.026224-0.026224",
            "unit": "s",
            "extra": "Building the lookup structure that compiled queries read, from a record's triples. A query the general evaluator answers needs none."
          },
          {
            "name": "ccda-entries-x10/stage/mapping",
            "value": 0.057002,
            "range": "0.057002-0.057002",
            "unit": "s",
            "extra": "Running the adapter's mapping queries on each record, without the time of building their lookup structure. (Specification stage: transform.)"
          },
          {
            "name": "ccda-entries-x10/stage/records",
            "value": 0.030976,
            "range": "0.030976-0.030976",
            "unit": "s",
            "extra": "Turning what the mapping produced into each record's graph: naming its versions, keeping one where the mapping wrote several, and writing their arrivals. (Specification stage: name versions, write arrivals.)"
          },
          {
            "name": "ccda-entries-x10/stage/output-validation",
            "value": 0.092708,
            "range": "0.092708-0.092708",
            "unit": "s",
            "extra": "Checking each record's output against the vocabulary's SHACL shapes."
          },
          {
            "name": "ccda-entries-x10/stage/findings",
            "value": 0.037544,
            "range": "0.037544-0.037544",
            "unit": "s",
            "extra": "Running the adapter's findings queries to find what the source record got wrong or left out. (Specification stage: findings.)"
          },
          {
            "name": "ccda-entries-x10/stage/unresolved-addresses",
            "value": 0.016745,
            "range": "0.016745-0.016745",
            "unit": "s",
            "extra": "Finding the references to records the document does not hold, which keep their computed names."
          },
          {
            "name": "ccda-entries-x10/stage/detect",
            "value": 0.010262,
            "range": "0.010262-0.010262",
            "unit": "s",
            "extra": "Running the adapter's detect query, which reports whether the adapter reads this document. The Bridge converts every record whatever the answer; it never routes."
          },
          {
            "name": "ccda-entries-x10/stage/summary",
            "value": 0.001002,
            "range": "0.001002-0.001002",
            "unit": "s",
            "extra": "Counting the graph's distinct triples and the findings once every record is done."
          },
          {
            "name": "ccda-entries-x10/stage/serialising",
            "value": 0.016653,
            "range": "0.016653-0.016653",
            "unit": "s",
            "extra": "Writing the graph and the findings out as Turtle or N-Triples."
          },
          {
            "name": "ccda-entries-x10/stage/unaccounted",
            "value": 0.017837,
            "range": "0.017837-0.017837",
            "unit": "s",
            "extra": "Seconds of the profiled convert that none of the stages accounts for, such as writing the document and the import into the graph."
          },
          {
            "name": "ccda-entries-x10/output/graph",
            "value": 1926,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph, with a SHA-256 fingerprint of the output that is the same whenever the content is the same, whatever order the engine wrote it in and whatever it named its blank nodes. sha256 ad5fa27c4fe8b29ebed8c8892c3bf3df74f202365f5e14dca5a7981d130dfc16"
          },
          {
            "name": "ccda-entries-x10/output/findings",
            "value": 1210,
            "unit": "{count}",
            "extra": "Distinct triples in the findings, with a SHA-256 fingerprint of them that is the same whenever the content is the same, whatever order the engine wrote them in and whatever it named its blank nodes. sha256 95da111564c5677296616ba1bec9f3340f1788949bbde806f616a5ca56b4ba0a"
          },
          {
            "name": "ccda-entries-x50/convert/first",
            "value": 1.686448,
            "range": "1.646147-1.815448",
            "unit": "s",
            "extra": "Seconds the first convert of a fresh process takes, timed without the step-by-step breakdown. A caller that opens an adapter and converts once also waits for the engine to start (the boot rows) and the adapter to load (the load rows), which this does not count. The C-CDA medications-and-results example with its entries repeated 50 times, as one document. sha256 74d7ceb7fbc39145ce2f7d8c2aca5f71848dc80ced2ebbdf76ed795b43ba7f62, 667684 bytes"
          },
          {
            "name": "ccda-entries-x50/convert/second",
            "value": 1.331986,
            "range": "1.323556-1.351344",
            "unit": "s",
            "extra": "Seconds a second convert of the same document takes in the same process, timed without the step-by-step breakdown: the warm cost."
          },
          {
            "name": "ccda-entries-x50/convert/profiled",
            "value": 1.668966,
            "range": "1.668966-1.668966",
            "unit": "s",
            "extra": "Seconds of the one convert run with the step-by-step breakdown on, from which the stage and query rows are read. The breakdown makes it slower than a plain convert."
          },
          {
            "name": "ccda-entries-x50/wasm-memory/first",
            "value": 136511488,
            "range": "136511488-136511488",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the first convert is done."
          },
          {
            "name": "ccda-entries-x50/wasm-memory/second",
            "value": 136511488,
            "range": "136511488-136511488",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the second convert is done."
          },
          {
            "name": "ccda-entries-x50/rss",
            "value": 434331648,
            "range": "369205248-441241600",
            "unit": "By",
            "extra": "Peak memory the whole process held in RAM, in bytes: the engine's start, the adapter's load and both converts all count."
          },
          {
            "name": "ccda-entries-x50/records",
            "value": 1,
            "unit": "{count}",
            "extra": "Source records the Bridge found in the document and converted. The C-CDA medications-and-results example with its entries repeated 50 times, as one document."
          },
          {
            "name": "ccda-entries-x50/triples",
            "value": 9126,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph."
          },
          {
            "name": "ccda-entries-x50/findings",
            "value": 453,
            "unit": "{count}",
            "extra": "Findings the conversion wrote."
          },
          {
            "name": "ccda-entries-x50/query-time/document-table.rq",
            "value": 0.32965,
            "range": "0.32965-0.32965",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/allergy.rq",
            "value": 0.00093,
            "range": "0.00093-0.00093",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/condition.rq",
            "value": 0.000873,
            "range": "0.000873-0.000873",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/document-organizations.rq",
            "value": 0.001269,
            "range": "0.001269-0.001269",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/immunization.rq",
            "value": 0.002089,
            "range": "0.002089-0.002089",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/medication.rq",
            "value": 0.033259,
            "range": "0.033259-0.033259",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/note-or-document.rq",
            "value": 0.008365,
            "range": "0.008365-0.008365",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/patient.rq",
            "value": 0.000529,
            "range": "0.000529-0.000529",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/procedure.rq",
            "value": 0.002533,
            "range": "0.002533-0.002533",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/record.rq",
            "value": 0.020028,
            "range": "0.020028-0.020028",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/report.rq",
            "value": 0.011854,
            "range": "0.011854-0.011854",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/result-identifiers.rq",
            "value": 0.008437,
            "range": "0.008437-0.008437",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/result-quantities.rq",
            "value": 0.018588,
            "range": "0.018588-0.018588",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/result.rq",
            "value": 0.061791,
            "range": "0.061791-0.061791",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/allergy-findings.rq",
            "value": 0.001199,
            "range": "0.001199-0.001199",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/condition-findings.rq",
            "value": 0.002425,
            "range": "0.002425-0.002425",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/document-findings.rq",
            "value": 0.001166,
            "range": "0.001166-0.001166",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/entry-findings.rq",
            "value": 0.010231,
            "range": "0.010231-0.010231",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/entry-statement-findings.rq",
            "value": 0.012882,
            "range": "0.012882-0.012882",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/medication-findings.rq",
            "value": 0.014727,
            "range": "0.014727-0.014727",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/name-findings.rq",
            "value": 0.00382,
            "range": "0.00382-0.00382",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/narrative-findings.rq",
            "value": 0.003772,
            "range": "0.003772-0.003772",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/organization-findings.rq",
            "value": 0.00097,
            "range": "0.00097-0.00097",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/patient-findings.rq",
            "value": 0.000155,
            "range": "0.000155-0.000155",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/reference-findings.rq",
            "value": 0.012491,
            "range": "0.012491-0.012491",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/report-and-note-findings.rq",
            "value": 0.004336,
            "range": "0.004336-0.004336",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/result-findings.rq",
            "value": 0.061819,
            "range": "0.061819-0.061819",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/section-findings.rq",
            "value": 0.001745,
            "range": "0.001745-0.001745",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/query-time/detect.rq",
            "value": 0.006639,
            "range": "0.006639-0.006639",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-entries-x50/stage/document-table",
            "value": 0.338064,
            "range": "0.338064-0.338064",
            "unit": "s",
            "extra": "Running the adapter's document-table query on every record of the document, to link a record to any other record, in the document or not. (Specification stage: link records.)"
          },
          {
            "name": "ccda-entries-x50/stage/lift",
            "value": 0.20896,
            "range": "0.20896-0.20896",
            "unit": "s",
            "extra": "Reading the source document into the triples the adapter's queries can ask about, one record at a time. (Specification stage: read and chunk.)"
          },
          {
            "name": "ccda-entries-x50/stage/source-validation",
            "value": 0.095965,
            "range": "0.095965-0.095965",
            "unit": "s",
            "extra": "Checking each source record, and the document as a whole where the adapter gives a document schema, against the adapter's schema (XSD or JSON Schema). A failure is a finding, never a refusal. (Specification stage: check, validate.)"
          },
          {
            "name": "ccda-entries-x50/stage/accounting",
            "value": 0.000985,
            "range": "0.000985-0.000985",
            "unit": "s",
            "extra": "Working out, against the adapter's accounting file, what became of each part of each source record."
          },
          {
            "name": "ccda-entries-x50/stage/dataset",
            "value": 0.000054,
            "range": "0.000054-0.000054",
            "unit": "s",
            "extra": "Putting the facts about a record into the store its queries run over."
          },
          {
            "name": "ccda-entries-x50/stage/index",
            "value": 0.06097,
            "range": "0.06097-0.06097",
            "unit": "s",
            "extra": "Building the lookup structure that compiled queries read, from a record's triples. A query the general evaluator answers needs none."
          },
          {
            "name": "ccda-entries-x50/stage/mapping",
            "value": 0.172562,
            "range": "0.172562-0.172562",
            "unit": "s",
            "extra": "Running the adapter's mapping queries on each record, without the time of building their lookup structure. (Specification stage: transform.)"
          },
          {
            "name": "ccda-entries-x50/stage/records",
            "value": 0.247357,
            "range": "0.247357-0.247357",
            "unit": "s",
            "extra": "Turning what the mapping produced into each record's graph: naming its versions, keeping one where the mapping wrote several, and writing their arrivals. (Specification stage: name versions, write arrivals.)"
          },
          {
            "name": "ccda-entries-x50/stage/output-validation",
            "value": 0.270515,
            "range": "0.270515-0.270515",
            "unit": "s",
            "extra": "Checking each record's output against the vocabulary's SHACL shapes."
          },
          {
            "name": "ccda-entries-x50/stage/findings",
            "value": 0.139038,
            "range": "0.139038-0.139038",
            "unit": "s",
            "extra": "Running the adapter's findings queries to find what the source record got wrong or left out. (Specification stage: findings.)"
          },
          {
            "name": "ccda-entries-x50/stage/unresolved-addresses",
            "value": 0.060187,
            "range": "0.060187-0.060187",
            "unit": "s",
            "extra": "Finding the references to records the document does not hold, which keep their computed names."
          },
          {
            "name": "ccda-entries-x50/stage/detect",
            "value": 0.006682,
            "range": "0.006682-0.006682",
            "unit": "s",
            "extra": "Running the adapter's detect query, which reports whether the adapter reads this document. The Bridge converts every record whatever the answer; it never routes."
          },
          {
            "name": "ccda-entries-x50/stage/summary",
            "value": 0.002478,
            "range": "0.002478-0.002478",
            "unit": "s",
            "extra": "Counting the graph's distinct triples and the findings once every record is done."
          },
          {
            "name": "ccda-entries-x50/stage/serialising",
            "value": 0.02583,
            "range": "0.02583-0.02583",
            "unit": "s",
            "extra": "Writing the graph and the findings out as Turtle or N-Triples."
          },
          {
            "name": "ccda-entries-x50/stage/unaccounted",
            "value": 0.03932,
            "range": "0.03932-0.03932",
            "unit": "s",
            "extra": "Seconds of the profiled convert that none of the stages accounts for, such as writing the document and the import into the graph."
          },
          {
            "name": "ccda-entries-x50/output/graph",
            "value": 9126,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph, with a SHA-256 fingerprint of the output that is the same whenever the content is the same, whatever order the engine wrote it in and whatever it named its blank nodes. sha256 139d803f26e008902033b2b0e3393b1c53073557b209e9222053dbf3e93d0bfc"
          },
          {
            "name": "ccda-entries-x50/output/findings",
            "value": 5890,
            "unit": "{count}",
            "extra": "Distinct triples in the findings, with a SHA-256 fingerprint of them that is the same whenever the content is the same, whatever order the engine wrote them in and whatever it named its blank nodes. sha256 ade58a19db77babdcde6b85475e8f4b509473641fe2efeb3c993ffc1f4807c6c"
          },
          {
            "name": "ccda-synthea-10y/convert/first",
            "value": 3.590173,
            "range": "3.586932-3.602765",
            "unit": "s",
            "extra": "Seconds the first convert of a fresh process takes, timed without the step-by-step breakdown. A caller that opens an adapter and converts once also waits for the engine to start (the boot rows) and the adapter to load (the load rows), which this does not count. Ten years of one Synthea person as a single C-CDA document. sha256 643c55e2e3db4e52974cdf202b7a9a2f0f1bcf72e47a64414e014ae8c1ccd361, 1226024 bytes"
          },
          {
            "name": "ccda-synthea-10y/convert/second",
            "value": 3.172037,
            "range": "3.164082-3.178682",
            "unit": "s",
            "extra": "Seconds a second convert of the same document takes in the same process, timed without the step-by-step breakdown: the warm cost."
          },
          {
            "name": "ccda-synthea-10y/convert/profiled",
            "value": 3.614648,
            "range": "3.614648-3.614648",
            "unit": "s",
            "extra": "Seconds of the one convert run with the step-by-step breakdown on, from which the stage and query rows are read. The breakdown makes it slower than a plain convert."
          },
          {
            "name": "ccda-synthea-10y/wasm-memory/first",
            "value": 237174784,
            "range": "237174784-237174784",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the first convert is done."
          },
          {
            "name": "ccda-synthea-10y/wasm-memory/second",
            "value": 237174784,
            "range": "237174784-237174784",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the second convert is done."
          },
          {
            "name": "ccda-synthea-10y/rss",
            "value": 549793792,
            "range": "547819520-550776832",
            "unit": "By",
            "extra": "Peak memory the whole process held in RAM, in bytes: the engine's start, the adapter's load and both converts all count."
          },
          {
            "name": "ccda-synthea-10y/records",
            "value": 1,
            "unit": "{count}",
            "extra": "Source records the Bridge found in the document and converted. Ten years of one Synthea person as a single C-CDA document."
          },
          {
            "name": "ccda-synthea-10y/triples",
            "value": 21772,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph."
          },
          {
            "name": "ccda-synthea-10y/findings",
            "value": 1234,
            "unit": "{count}",
            "extra": "Findings the conversion wrote."
          },
          {
            "name": "ccda-synthea-10y/query-time/document-table.rq",
            "value": 0.59618,
            "range": "0.59618-0.59618",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/allergy.rq",
            "value": 0.008295,
            "range": "0.008295-0.008295",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/condition.rq",
            "value": 0.018496,
            "range": "0.018496-0.018496",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/document-organizations.rq",
            "value": 0.000503,
            "range": "0.000503-0.000503",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/immunization.rq",
            "value": 0.007034,
            "range": "0.007034-0.007034",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/medication.rq",
            "value": 0.029303,
            "range": "0.029303-0.029303",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/note-or-document.rq",
            "value": 0.007298,
            "range": "0.007298-0.007298",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/patient.rq",
            "value": 0.000456,
            "range": "0.000456-0.000456",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/procedure.rq",
            "value": 0.019225,
            "range": "0.019225-0.019225",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/record.rq",
            "value": 0.039763,
            "range": "0.039763-0.039763",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/report.rq",
            "value": 0.015729,
            "range": "0.015729-0.015729",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/result-identifiers.rq",
            "value": 0.017416,
            "range": "0.017416-0.017416",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/result-quantities.rq",
            "value": 0.032531,
            "range": "0.032531-0.032531",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/result.rq",
            "value": 0.150963,
            "range": "0.150963-0.150963",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/allergy-findings.rq",
            "value": 0.006007,
            "range": "0.006007-0.006007",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/condition-findings.rq",
            "value": 0.006749,
            "range": "0.006749-0.006749",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/document-findings.rq",
            "value": 0.002012,
            "range": "0.002012-0.002012",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/entry-findings.rq",
            "value": 0.015281,
            "range": "0.015281-0.015281",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/entry-statement-findings.rq",
            "value": 0.015024,
            "range": "0.015024-0.015024",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/medication-findings.rq",
            "value": 0.00728,
            "range": "0.00728-0.00728",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/name-findings.rq",
            "value": 0.012222,
            "range": "0.012222-0.012222",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/narrative-findings.rq",
            "value": 0.058828,
            "range": "0.058828-0.058828",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/organization-findings.rq",
            "value": 0.000566,
            "range": "0.000566-0.000566",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/patient-findings.rq",
            "value": 0.000261,
            "range": "0.000261-0.000261",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/reference-findings.rq",
            "value": 0.025695,
            "range": "0.025695-0.025695",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/report-and-note-findings.rq",
            "value": 0.006701,
            "range": "0.006701-0.006701",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/result-findings.rq",
            "value": 0.141628,
            "range": "0.141628-0.141628",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/section-findings.rq",
            "value": 0.003519,
            "range": "0.003519-0.003519",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/query-time/detect.rq",
            "value": 0.006673,
            "range": "0.006673-0.006673",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "ccda-synthea-10y/stage/document-table",
            "value": 0.618287,
            "range": "0.618287-0.618287",
            "unit": "s",
            "extra": "Running the adapter's document-table query on every record of the document, to link a record to any other record, in the document or not. (Specification stage: link records.)"
          },
          {
            "name": "ccda-synthea-10y/stage/lift",
            "value": 0.408578,
            "range": "0.408578-0.408578",
            "unit": "s",
            "extra": "Reading the source document into the triples the adapter's queries can ask about, one record at a time. (Specification stage: read and chunk.)"
          },
          {
            "name": "ccda-synthea-10y/stage/source-validation",
            "value": 0.140014,
            "range": "0.140014-0.140014",
            "unit": "s",
            "extra": "Checking each source record, and the document as a whole where the adapter gives a document schema, against the adapter's schema (XSD or JSON Schema). A failure is a finding, never a refusal. (Specification stage: check, validate.)"
          },
          {
            "name": "ccda-synthea-10y/stage/accounting",
            "value": 0.001928,
            "range": "0.001928-0.001928",
            "unit": "s",
            "extra": "Working out, against the adapter's accounting file, what became of each part of each source record."
          },
          {
            "name": "ccda-synthea-10y/stage/dataset",
            "value": 0.000063,
            "range": "0.000063-0.000063",
            "unit": "s",
            "extra": "Putting the facts about a record into the store its queries run over."
          },
          {
            "name": "ccda-synthea-10y/stage/index",
            "value": 0.126329,
            "range": "0.126329-0.126329",
            "unit": "s",
            "extra": "Building the lookup structure that compiled queries read, from a record's triples. A query the general evaluator answers needs none."
          },
          {
            "name": "ccda-synthea-10y/stage/mapping",
            "value": 0.349412,
            "range": "0.349412-0.349412",
            "unit": "s",
            "extra": "Running the adapter's mapping queries on each record, without the time of building their lookup structure. (Specification stage: transform.)"
          },
          {
            "name": "ccda-synthea-10y/stage/records",
            "value": 0.84371,
            "range": "0.84371-0.84371",
            "unit": "s",
            "extra": "Turning what the mapping produced into each record's graph: naming its versions, keeping one where the mapping wrote several, and writing their arrivals. (Specification stage: name versions, write arrivals.)"
          },
          {
            "name": "ccda-synthea-10y/stage/output-validation",
            "value": 0.519493,
            "range": "0.519493-0.519493",
            "unit": "s",
            "extra": "Checking each record's output against the vocabulary's SHACL shapes."
          },
          {
            "name": "ccda-synthea-10y/stage/findings",
            "value": 0.321307,
            "range": "0.321307-0.321307",
            "unit": "s",
            "extra": "Running the adapter's findings queries to find what the source record got wrong or left out. (Specification stage: findings.)"
          },
          {
            "name": "ccda-synthea-10y/stage/unresolved-addresses",
            "value": 0.144936,
            "range": "0.144936-0.144936",
            "unit": "s",
            "extra": "Finding the references to records the document does not hold, which keep their computed names."
          },
          {
            "name": "ccda-synthea-10y/stage/detect",
            "value": 0.006716,
            "range": "0.006716-0.006716",
            "unit": "s",
            "extra": "Running the adapter's detect query, which reports whether the adapter reads this document. The Bridge converts every record whatever the answer; it never routes."
          },
          {
            "name": "ccda-synthea-10y/stage/summary",
            "value": 0.005047,
            "range": "0.005047-0.005047",
            "unit": "s",
            "extra": "Counting the graph's distinct triples and the findings once every record is done."
          },
          {
            "name": "ccda-synthea-10y/stage/serialising",
            "value": 0.062106,
            "range": "0.062106-0.062106",
            "unit": "s",
            "extra": "Writing the graph and the findings out as Turtle or N-Triples."
          },
          {
            "name": "ccda-synthea-10y/stage/unaccounted",
            "value": 0.066724,
            "range": "0.066724-0.066724",
            "unit": "s",
            "extra": "Seconds of the profiled convert that none of the stages accounts for, such as writing the document and the import into the graph."
          },
          {
            "name": "ccda-synthea-10y/output/graph",
            "value": 21772,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph, with a SHA-256 fingerprint of the output that is the same whenever the content is the same, whatever order the engine wrote it in and whatever it named its blank nodes. sha256 4c682c9d7746d30c8e5fd69e73f2e02378d0d2906267d583be3e5c817fd47817"
          },
          {
            "name": "ccda-synthea-10y/output/findings",
            "value": 16083,
            "unit": "{count}",
            "extra": "Distinct triples in the findings, with a SHA-256 fingerprint of them that is the same whenever the content is the same, whatever order the engine wrote them in and whatever it named its blank nodes. sha256 65c70b4b78caf2e112089b8b759422c88c7e53903bf481200f38011bb40dcb8c"
          },
          {
            "name": "ccda/load",
            "value": 0.918838,
            "range": "0.864062-1.080943",
            "unit": "s",
            "extra": "Seconds to read the adapter's and the vocabulary's files, describe the adapter and prepare it, with the engine already started."
          },
          {
            "name": "ccda/load/read-files",
            "value": 0.007016,
            "range": "0.006759-0.0103",
            "unit": "s",
            "extra": "Seconds of the load spent reading files from disk."
          },
          {
            "name": "ccda/load/prepare",
            "value": 0.911952,
            "range": "0.857193-1.073927",
            "unit": "s",
            "extra": "Seconds of the load spent describing the adapter and preparing it: parsing and compiling its queries, schemas and shapes."
          },
          {
            "name": "ccda/boot/read-wasm",
            "value": 0.004183,
            "range": "0.004125-0.004352",
            "unit": "s",
            "extra": "Seconds to read the engine's compiled code from disk."
          },
          {
            "name": "ccda/boot/compile-wasm",
            "value": 0.021615,
            "range": "0.02149-0.024851",
            "unit": "s",
            "extra": "Seconds to turn the engine's compiled code into something the machine can run."
          },
          {
            "name": "ccda/boot/instantiate-wasm",
            "value": 0.001478,
            "range": "0.001319-0.002617",
            "unit": "s",
            "extra": "Seconds to start the compiled code, which runs its start-up work."
          },
          {
            "name": "fhir-r4-lab-pull-3000/convert/first",
            "value": 7.607649,
            "range": "7.590689-7.676645",
            "unit": "s",
            "extra": "Seconds the first convert of a fresh process takes, timed without the step-by-step breakdown. A caller that opens an adapter and converts once also waits for the engine to start (the boot rows) and the adapter to load (the load rows), which this does not count. A pull of 3,000 lab results in one FHIR R4 searchset Bundle. sha256 69b4daac45fd32449b8c290be9a7551566b537a6171218e8125169e94ca5882b, 2760035 bytes"
          },
          {
            "name": "fhir-r4-lab-pull-3000/convert/second",
            "value": 7.232376,
            "range": "7.199702-7.240029",
            "unit": "s",
            "extra": "Seconds a second convert of the same document takes in the same process, timed without the step-by-step breakdown: the warm cost."
          },
          {
            "name": "fhir-r4-lab-pull-3000/convert/profiled",
            "value": 7.774514,
            "range": "7.774514-7.774514",
            "unit": "s",
            "extra": "Seconds of the one convert run with the step-by-step breakdown on, from which the stage and query rows are read. The breakdown makes it slower than a plain convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/wasm-memory/first",
            "value": 136511488,
            "range": "136511488-136511488",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the first convert is done."
          },
          {
            "name": "fhir-r4-lab-pull-3000/wasm-memory/second",
            "value": 136511488,
            "range": "136511488-136511488",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the second convert is done."
          },
          {
            "name": "fhir-r4-lab-pull-3000/rss",
            "value": 377516032,
            "range": "358477824-420552704",
            "unit": "By",
            "extra": "Peak memory the whole process held in RAM, in bytes: the engine's start, the adapter's load and both converts all count."
          },
          {
            "name": "fhir-r4-lab-pull-3000/records",
            "value": 3004,
            "unit": "{count}",
            "extra": "Source records the Bridge found in the document and converted. A pull of 3,000 lab results in one FHIR R4 searchset Bundle."
          },
          {
            "name": "fhir-r4-lab-pull-3000/triples",
            "value": 63025,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph."
          },
          {
            "name": "fhir-r4-lab-pull-3000/findings",
            "value": 3006,
            "unit": "{count}",
            "extra": "Findings the conversion wrote."
          },
          {
            "name": "fhir-r4-lab-pull-3000/fallbacks",
            "value": 0,
            "unit": "{count}",
            "extra": "Record executions the general evaluator answered because a query was not compiled or a compiled plan gave up. The detect query, which runs once, is not counted."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/coding-system-table.rq",
            "value": 0.074594,
            "range": "0.074594-0.074594",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/document-table.rq",
            "value": 0.011227,
            "range": "0.011227-0.011227",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/lab-result-table.rq",
            "value": 0.02253,
            "range": "0.02253-0.02253",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/medication-table.rq",
            "value": 0.011977,
            "range": "0.011977-0.011977",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/note-or-document-table.rq",
            "value": 0.005988,
            "range": "0.005988-0.005988",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/organization-table.rq",
            "value": 0.020541,
            "range": "0.020541-0.020541",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/variant-interpretation-table.rq",
            "value": 0.008565,
            "range": "0.008565-0.008565",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/variant-table.rq",
            "value": 0.009201,
            "range": "0.009201-0.009201",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/absent-reasons.rq",
            "value": 0.076215,
            "range": "0.076215-0.076215",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/allergy-intolerance.rq",
            "value": 0.011579,
            "range": "0.011579-0.011579",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/attachments.rq",
            "value": 0.012989,
            "range": "0.012989-0.012989",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/condition.rq",
            "value": 0.009758,
            "range": "0.009758-0.009758",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/immunization.rq",
            "value": 0.00902,
            "range": "0.00902-0.00902",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/lab-result-codes.rq",
            "value": 0.119757,
            "range": "0.119757-0.119757",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/lab-result-identifiers.rq",
            "value": 0.0267,
            "range": "0.0267-0.0267",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/lab-result-performer.rq",
            "value": 0.004133,
            "range": "0.004133-0.004133",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/lab-result-quantities.rq",
            "value": 0.14079,
            "range": "0.14079-0.14079",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/lab-result.rq",
            "value": 0.14114,
            "range": "0.14114-0.14114",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/medication-dispense-prescription.rq",
            "value": 0.014117,
            "range": "0.014117-0.014117",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/medication-dispense.rq",
            "value": 0.008919,
            "range": "0.008919-0.008919",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/medication-supply-and-dosages.rq",
            "value": 0.010728,
            "range": "0.010728-0.010728",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/medication.rq",
            "value": 0.010342,
            "range": "0.010342-0.010342",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/narrative-text.rq",
            "value": 0.035495,
            "range": "0.035495-0.035495",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/note-or-document.rq",
            "value": 0.009183,
            "range": "0.009183-0.009183",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/organization-from-server.rq",
            "value": 0.00588,
            "range": "0.00588-0.00588",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/organization.rq",
            "value": 0.014331,
            "range": "0.014331-0.014331",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/patient.rq",
            "value": 0.009301,
            "range": "0.009301-0.009301",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/procedure.rq",
            "value": 0.008333,
            "range": "0.008333-0.008333",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/record.rq",
            "value": 0.333205,
            "range": "0.333205-0.333205",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/report-and-note-performer.rq",
            "value": 0.004396,
            "range": "0.004396-0.004396",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/report-results.rq",
            "value": 0.012178,
            "range": "0.012178-0.012178",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/report.rq",
            "value": 0.008238,
            "range": "0.008238-0.008238",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/variant-interpretation.rq",
            "value": 0.011734,
            "range": "0.011734-0.011734",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/variant.rq",
            "value": 0.010525,
            "range": "0.010525-0.010525",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/absent-reason-findings.rq",
            "value": 0.047902,
            "range": "0.047902-0.047902",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/category-text-findings.rq",
            "value": 0.015659,
            "range": "0.015659-0.015659",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/code-findings.rq",
            "value": 0.009772,
            "range": "0.009772-0.009772",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/date-findings.rq",
            "value": 0.050553,
            "range": "0.050553-0.050553",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/document-version-findings.rq",
            "value": 0.001584,
            "range": "0.001584-0.001584",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/lab-result-findings.rq",
            "value": 0.093341,
            "range": "0.093341-0.093341",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/medication-dispense-findings.rq",
            "value": 0.010304,
            "range": "0.010304-0.010304",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/medication-dispense-prescription-findings.rq",
            "value": 0.011437,
            "range": "0.011437-0.011437",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/medication-findings.rq",
            "value": 0.078607,
            "range": "0.078607-0.078607",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/narrative-findings.rq",
            "value": 0.008671,
            "range": "0.008671-0.008671",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/observation-component-findings.rq",
            "value": 0.012109,
            "range": "0.012109-0.012109",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/observation-findings.rq",
            "value": 0.077625,
            "range": "0.077625-0.077625",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/organization-findings.rq",
            "value": 0.009655,
            "range": "0.009655-0.009655",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/patient-findings.rq",
            "value": 0.009977,
            "range": "0.009977-0.009977",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/patient-reference-findings.rq",
            "value": 0.067005,
            "range": "0.067005-0.067005",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/recorded-date-findings.rq",
            "value": 0.010558,
            "range": "0.010558-0.010558",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/reference-display-findings.rq",
            "value": 0.037661,
            "range": "0.037661-0.037661",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/reference-without-server-findings.rq",
            "value": 0.04661,
            "range": "0.04661-0.04661",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/repeated-id-findings.rq",
            "value": 0.029058,
            "range": "0.029058-0.029058",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/report-and-note-findings.rq",
            "value": 0.013846,
            "range": "0.013846-0.013846",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/report-result-findings.rq",
            "value": 0.01194,
            "range": "0.01194-0.01194",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/resource-type-findings.rq",
            "value": 0.011729,
            "range": "0.011729-0.011729",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/variant-findings.rq",
            "value": 0.022084,
            "range": "0.022084-0.022084",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/variant-interpretation-findings.rq",
            "value": 0.012232,
            "range": "0.012232-0.012232",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/query-time/detect.rq",
            "value": 0.009333,
            "range": "0.009333-0.009333",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/document-table",
            "value": 0.207511,
            "range": "0.207511-0.207511",
            "unit": "s",
            "extra": "Running the adapter's document-table query on every record of the document, to link a record to any other record, in the document or not. (Specification stage: link records.)"
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/lift",
            "value": 0.447821,
            "range": "0.447821-0.447821",
            "unit": "s",
            "extra": "Reading the source document into the triples the adapter's queries can ask about, one record at a time. (Specification stage: read and chunk.)"
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/source-validation",
            "value": 1.047513,
            "range": "1.047513-1.047513",
            "unit": "s",
            "extra": "Checking each source record, and the document as a whole where the adapter gives a document schema, against the adapter's schema (XSD or JSON Schema). A failure is a finding, never a refusal. (Specification stage: check, validate.)"
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/accounting",
            "value": 0.018334,
            "range": "0.018334-0.018334",
            "unit": "s",
            "extra": "Working out, against the adapter's accounting file, what became of each part of each source record."
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/dataset",
            "value": 0.010366,
            "range": "0.010366-0.010366",
            "unit": "s",
            "extra": "Putting the facts about a record into the store its queries run over."
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/index",
            "value": 0.241376,
            "range": "0.241376-0.241376",
            "unit": "s",
            "extra": "Building the lookup structure that compiled queries read, from a record's triples. A query the general evaluator answers needs none."
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/mapping",
            "value": 1.095345,
            "range": "1.095345-1.095345",
            "unit": "s",
            "extra": "Running the adapter's mapping queries on each record, without the time of building their lookup structure. (Specification stage: transform.)"
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/records",
            "value": 0.276664,
            "range": "0.276664-0.276664",
            "unit": "s",
            "extra": "Turning what the mapping produced into each record's graph: naming its versions, keeping one where the mapping wrote several, and writing their arrivals. (Specification stage: name versions, write arrivals.)"
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/output-validation",
            "value": 3.119937,
            "range": "3.119937-3.119937",
            "unit": "s",
            "extra": "Checking each record's output against the vocabulary's SHACL shapes."
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/findings",
            "value": 0.911843,
            "range": "0.911843-0.911843",
            "unit": "s",
            "extra": "Running the adapter's findings queries to find what the source record got wrong or left out. (Specification stage: findings.)"
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/unresolved-addresses",
            "value": 0.045513,
            "range": "0.045513-0.045513",
            "unit": "s",
            "extra": "Finding the references to records the document does not hold, which keep their computed names."
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/detect",
            "value": 0.009413,
            "range": "0.009413-0.009413",
            "unit": "s",
            "extra": "Running the adapter's detect query, which reports whether the adapter reads this document. The Bridge converts every record whatever the answer; it never routes."
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/summary",
            "value": 0.014452,
            "range": "0.014452-0.014452",
            "unit": "s",
            "extra": "Counting the graph's distinct triples and the findings once every record is done."
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/serialising",
            "value": 0.139192,
            "range": "0.139192-0.139192",
            "unit": "s",
            "extra": "Writing the graph and the findings out as Turtle or N-Triples."
          },
          {
            "name": "fhir-r4-lab-pull-3000/stage/unaccounted",
            "value": 0.189233,
            "range": "0.189233-0.189233",
            "unit": "s",
            "extra": "Seconds of the profiled convert that none of the stages accounts for, such as writing the document and the import into the graph."
          },
          {
            "name": "fhir-r4-lab-pull-3000/output/graph",
            "value": 63025,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph, with a SHA-256 fingerprint of the output that is the same whenever the content is the same, whatever order the engine wrote it in and whatever it named its blank nodes. sha256 37ddd3ff45eedcca686486482a5dd0ea90120d791c04352fb96a41e20d5fe7ac"
          },
          {
            "name": "fhir-r4-lab-pull-3000/output/findings",
            "value": 45090,
            "unit": "{count}",
            "extra": "Distinct triples in the findings, with a SHA-256 fingerprint of them that is the same whenever the content is the same, whatever order the engine wrote them in and whatever it named its blank nodes. sha256 b882dcb20e82c494692121aa741018d60fa8b958d52919c0a6eca58735b69721"
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/convert/first",
            "value": 2.395635,
            "range": "2.382578-2.491069",
            "unit": "s",
            "extra": "Seconds the first convert of a fresh process takes, timed without the step-by-step breakdown. A caller that opens an adapter and converts once also waits for the engine to start (the boot rows) and the adapter to load (the load rows), which this does not count. The trimmed ten-year Synthea record of the demo hospitals, as a FHIR R4 Bundle. sha256 2703dc1e7e920fa1e45d16c58f8338ef13b3cc1916ee40ffa461112d54dce5de, 1098812 bytes"
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/convert/second",
            "value": 2.11765,
            "range": "2.109574-2.124703",
            "unit": "s",
            "extra": "Seconds a second convert of the same document takes in the same process, timed without the step-by-step breakdown: the warm cost."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/convert/profiled",
            "value": 2.421823,
            "range": "2.421823-2.421823",
            "unit": "s",
            "extra": "Seconds of the one convert run with the step-by-step breakdown on, from which the stage and query rows are read. The breakdown makes it slower than a plain convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/wasm-memory/first",
            "value": 86179840,
            "range": "69402624-86179840",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the first convert is done."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/wasm-memory/second",
            "value": 86179840,
            "range": "86179840-86179840",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the second convert is done."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/rss",
            "value": 384516096,
            "range": "372510720-388096000",
            "unit": "By",
            "extra": "Peak memory the whole process held in RAM, in bytes: the engine's start, the adapter's load and both converts all count."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/records",
            "value": 647,
            "unit": "{count}",
            "extra": "Source records the Bridge found in the document and converted. The trimmed ten-year Synthea record of the demo hospitals, as a FHIR R4 Bundle."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/triples",
            "value": 12654,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/findings",
            "value": 2256,
            "unit": "{count}",
            "extra": "Findings the conversion wrote."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/coding-system-table.rq",
            "value": 0.025016,
            "range": "0.025016-0.025016",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/document-table.rq",
            "value": 0.003942,
            "range": "0.003942-0.003942",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/lab-result-table.rq",
            "value": 0.005792,
            "range": "0.005792-0.005792",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/medication-table.rq",
            "value": 0.003795,
            "range": "0.003795-0.003795",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/note-or-document-table.rq",
            "value": 0.001942,
            "range": "0.001942-0.001942",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/organization-table.rq",
            "value": 0.00582,
            "range": "0.00582-0.00582",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/variant-interpretation-table.rq",
            "value": 0.002539,
            "range": "0.002539-0.002539",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/variant-table.rq",
            "value": 0.002701,
            "range": "0.002701-0.002701",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/absent-reasons.rq",
            "value": 0.018968,
            "range": "0.018968-0.018968",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/allergy-intolerance.rq",
            "value": 0.003153,
            "range": "0.003153-0.003153",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/attachments.rq",
            "value": 0.002959,
            "range": "0.002959-0.002959",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/condition.rq",
            "value": 0.007089,
            "range": "0.007089-0.007089",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/immunization.rq",
            "value": 0.002439,
            "range": "0.002439-0.002439",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/lab-result-codes.rq",
            "value": 0.017834,
            "range": "0.017834-0.017834",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/lab-result-identifiers.rq",
            "value": 0.004579,
            "range": "0.004579-0.004579",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/lab-result-performer.rq",
            "value": 0.00098,
            "range": "0.00098-0.00098",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/lab-result-quantities.rq",
            "value": 0.018655,
            "range": "0.018655-0.018655",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/lab-result.rq",
            "value": 0.02237,
            "range": "0.02237-0.02237",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/medication-dispense-prescription.rq",
            "value": 0.003152,
            "range": "0.003152-0.003152",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/medication-dispense.rq",
            "value": 0.002095,
            "range": "0.002095-0.002095",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/medication-supply-and-dosages.rq",
            "value": 0.007954,
            "range": "0.007954-0.007954",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/medication.rq",
            "value": 0.008056,
            "range": "0.008056-0.008056",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/narrative-text.rq",
            "value": 0.009674,
            "range": "0.009674-0.009674",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/note-or-document.rq",
            "value": 0.002144,
            "range": "0.002144-0.002144",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/organization-from-server.rq",
            "value": 0.001345,
            "range": "0.001345-0.001345",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/organization.rq",
            "value": 0.003306,
            "range": "0.003306-0.003306",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/patient.rq",
            "value": 0.002606,
            "range": "0.002606-0.002606",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/procedure.rq",
            "value": 0.004863,
            "range": "0.004863-0.004863",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/record.rq",
            "value": 0.076761,
            "range": "0.076761-0.076761",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/report-and-note-performer.rq",
            "value": 0.001063,
            "range": "0.001063-0.001063",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/report-results.rq",
            "value": 0.002872,
            "range": "0.002872-0.002872",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/report.rq",
            "value": 0.00188,
            "range": "0.00188-0.00188",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/variant-interpretation.rq",
            "value": 0.002482,
            "range": "0.002482-0.002482",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/variant.rq",
            "value": 0.002333,
            "range": "0.002333-0.002333",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/absent-reason-findings.rq",
            "value": 0.012345,
            "range": "0.012345-0.012345",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/category-text-findings.rq",
            "value": 0.004039,
            "range": "0.004039-0.004039",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/code-findings.rq",
            "value": 0.003133,
            "range": "0.003133-0.003133",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/date-findings.rq",
            "value": 0.015109,
            "range": "0.015109-0.015109",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/document-version-findings.rq",
            "value": 0.000371,
            "range": "0.000371-0.000371",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/lab-result-findings.rq",
            "value": 0.019444,
            "range": "0.019444-0.019444",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/medication-dispense-findings.rq",
            "value": 0.002209,
            "range": "0.002209-0.002209",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/medication-dispense-prescription-findings.rq",
            "value": 0.002524,
            "range": "0.002524-0.002524",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/medication-findings.rq",
            "value": 0.026149,
            "range": "0.026149-0.026149",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/narrative-findings.rq",
            "value": 0.00214,
            "range": "0.00214-0.00214",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/observation-component-findings.rq",
            "value": 0.002827,
            "range": "0.002827-0.002827",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/observation-findings.rq",
            "value": 0.013135,
            "range": "0.013135-0.013135",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/organization-findings.rq",
            "value": 0.001975,
            "range": "0.001975-0.001975",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/patient-findings.rq",
            "value": 0.0019,
            "range": "0.0019-0.0019",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/patient-reference-findings.rq",
            "value": 0.015038,
            "range": "0.015038-0.015038",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/recorded-date-findings.rq",
            "value": 0.002682,
            "range": "0.002682-0.002682",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/reference-display-findings.rq",
            "value": 0.008775,
            "range": "0.008775-0.008775",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/reference-without-server-findings.rq",
            "value": 0.009612,
            "range": "0.009612-0.009612",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/repeated-id-findings.rq",
            "value": 0.005556,
            "range": "0.005556-0.005556",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/report-and-note-findings.rq",
            "value": 0.003052,
            "range": "0.003052-0.003052",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/report-result-findings.rq",
            "value": 0.002641,
            "range": "0.002641-0.002641",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/resource-type-findings.rq",
            "value": 0.002847,
            "range": "0.002847-0.002847",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/variant-findings.rq",
            "value": 0.004238,
            "range": "0.004238-0.004238",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/variant-interpretation-findings.rq",
            "value": 0.002697,
            "range": "0.002697-0.002697",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/query-time/detect.rq",
            "value": 0.009157,
            "range": "0.009157-0.009157",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/stage/document-table",
            "value": 0.067048,
            "range": "0.067048-0.067048",
            "unit": "s",
            "extra": "Running the adapter's document-table query on every record of the document, to link a record to any other record, in the document or not. (Specification stage: link records.)"
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/stage/lift",
            "value": 0.144456,
            "range": "0.144456-0.144456",
            "unit": "s",
            "extra": "Reading the source document into the triples the adapter's queries can ask about, one record at a time. (Specification stage: read and chunk.)"
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/stage/source-validation",
            "value": 0.303014,
            "range": "0.303014-0.303014",
            "unit": "s",
            "extra": "Checking each source record, and the document as a whole where the adapter gives a document schema, against the adapter's schema (XSD or JSON Schema). A failure is a finding, never a refusal. (Specification stage: check, validate.)"
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/stage/accounting",
            "value": 0.024916,
            "range": "0.024916-0.024916",
            "unit": "s",
            "extra": "Working out, against the adapter's accounting file, what became of each part of each source record."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/stage/dataset",
            "value": 0.003039,
            "range": "0.003039-0.003039",
            "unit": "s",
            "extra": "Putting the facts about a record into the store its queries run over."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/stage/index",
            "value": 0.075747,
            "range": "0.075747-0.075747",
            "unit": "s",
            "extra": "Building the lookup structure that compiled queries read, from a record's triples. A query the general evaluator answers needs none."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/stage/mapping",
            "value": 0.240222,
            "range": "0.240222-0.240222",
            "unit": "s",
            "extra": "Running the adapter's mapping queries on each record, without the time of building their lookup structure. (Specification stage: transform.)"
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/stage/records",
            "value": 0.497293,
            "range": "0.497293-0.497293",
            "unit": "s",
            "extra": "Turning what the mapping produced into each record's graph: naming its versions, keeping one where the mapping wrote several, and writing their arrivals. (Specification stage: name versions, write arrivals.)"
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/stage/output-validation",
            "value": 0.705076,
            "range": "0.705076-0.705076",
            "unit": "s",
            "extra": "Checking each record's output against the vocabulary's SHACL shapes."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/stage/findings",
            "value": 0.219056,
            "range": "0.219056-0.219056",
            "unit": "s",
            "extra": "Running the adapter's findings queries to find what the source record got wrong or left out. (Specification stage: findings.)"
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/stage/unresolved-addresses",
            "value": 0.012516,
            "range": "0.012516-0.012516",
            "unit": "s",
            "extra": "Finding the references to records the document does not hold, which keep their computed names."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/stage/detect",
            "value": 0.009222,
            "range": "0.009222-0.009222",
            "unit": "s",
            "extra": "Running the adapter's detect query, which reports whether the adapter reads this document. The Bridge converts every record whatever the answer; it never routes."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/stage/summary",
            "value": 0.003495,
            "range": "0.003495-0.003495",
            "unit": "s",
            "extra": "Counting the graph's distinct triples and the findings once every record is done."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/stage/serialising",
            "value": 0.05224,
            "range": "0.05224-0.05224",
            "unit": "s",
            "extra": "Writing the graph and the findings out as Turtle or N-Triples."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/stage/unaccounted",
            "value": 0.064483,
            "range": "0.064483-0.064483",
            "unit": "s",
            "extra": "Seconds of the profiled convert that none of the stages accounts for, such as writing the document and the import into the graph."
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/output/graph",
            "value": 12654,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph, with a SHA-256 fingerprint of the output that is the same whenever the content is the same, whatever order the engine wrote it in and whatever it named its blank nodes. sha256 7cc449ffbde5128e52fafcf8acfb64362df9b08f54ac053668923208f8175856"
          },
          {
            "name": "fhir-r4-synthea-10y-trimmed/output/findings",
            "value": 33875,
            "unit": "{count}",
            "extra": "Distinct triples in the findings, with a SHA-256 fingerprint of them that is the same whenever the content is the same, whatever order the engine wrote them in and whatever it named its blank nodes. sha256 d537a811bedd885d56ef2486e522f7087e751130f23262dcfd5a6a287d47e7de"
          },
          {
            "name": "fhir-r4-synthea-lifetime/convert/first",
            "value": 57.495944,
            "range": "57.441942-57.796395",
            "unit": "s",
            "extra": "Seconds the first convert of a fresh process takes, timed without the step-by-step breakdown. A caller that opens an adapter and converts once also waits for the engine to start (the boot rows) and the adapter to load (the load rows), which this does not count. The lifetime Synthea record of one person, Xochitl Corkery, a 51 MB FHIR R4 Bundle of 16,833 entries. With the records of Dona Okuneva and Cole Gaylord it is also one pod of 42,784 entries; the three are unrelated people and Xochitl's is 39% of the pod, so the step from this input to the pod is not independent data. sha256 f20b86408066ece2d25a7b02d380b7652f22d16df9dab613a0c73c9969b17da8, 51227791 bytes"
          },
          {
            "name": "fhir-r4-synthea-lifetime/convert/second",
            "value": 55.021384,
            "range": "54.864659-55.296101",
            "unit": "s",
            "extra": "Seconds a second convert of the same document takes in the same process, timed without the step-by-step breakdown: the warm cost."
          },
          {
            "name": "fhir-r4-synthea-lifetime/convert/profiled",
            "value": 57.496559,
            "range": "57.496559-57.496559",
            "unit": "s",
            "extra": "Seconds of the one convert run with the step-by-step breakdown on, from which the stage and query rows are read. The breakdown makes it slower than a plain convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/wasm-memory/first",
            "value": 2518876160,
            "range": "2518876160-2518876160",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the first convert is done."
          },
          {
            "name": "fhir-r4-synthea-lifetime/wasm-memory/second",
            "value": 2518876160,
            "range": "2518876160-2518876160",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the second convert is done."
          },
          {
            "name": "fhir-r4-synthea-lifetime/rss",
            "value": 3362816000,
            "range": "3321913344-3362885632",
            "unit": "By",
            "extra": "Peak memory the whole process held in RAM, in bytes: the engine's start, the adapter's load and both converts all count."
          },
          {
            "name": "fhir-r4-synthea-lifetime/records",
            "value": 16833,
            "unit": "{count}",
            "extra": "Source records the Bridge found in the document and converted. The lifetime Synthea record of one person, Xochitl Corkery, a 51 MB FHIR R4 Bundle of 16,833 entries. With the records of Dona Okuneva and Cole Gaylord it is also one pod of 42,784 entries; the three are unrelated people and Xochitl's is 39% of the pod, so the step from this input to the pod is not independent data."
          },
          {
            "name": "fhir-r4-synthea-lifetime/triples",
            "value": 184332,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph."
          },
          {
            "name": "fhir-r4-synthea-lifetime/findings",
            "value": 267199,
            "unit": "{count}",
            "extra": "Findings the conversion wrote."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/coding-system-table.rq",
            "value": 0.592455,
            "range": "0.592455-0.592455",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/document-table.rq",
            "value": 0.05973,
            "range": "0.05973-0.05973",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/lab-result-table.rq",
            "value": 0.070039,
            "range": "0.070039-0.070039",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/medication-table.rq",
            "value": 0.087224,
            "range": "0.087224-0.087224",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/note-or-document-table.rq",
            "value": 0.037944,
            "range": "0.037944-0.037944",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/organization-table.rq",
            "value": 0.096167,
            "range": "0.096167-0.096167",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/variant-interpretation-table.rq",
            "value": 0.038166,
            "range": "0.038166-0.038166",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/variant-table.rq",
            "value": 0.039001,
            "range": "0.039001-0.039001",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/absent-reasons.rq",
            "value": 0.496064,
            "range": "0.496064-0.496064",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/allergy-intolerance.rq",
            "value": 0.077021,
            "range": "0.077021-0.077021",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/attachments.rq",
            "value": 0.119796,
            "range": "0.119796-0.119796",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/condition.rq",
            "value": 0.07938,
            "range": "0.07938-0.07938",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/immunization.rq",
            "value": 0.06127,
            "range": "0.06127-0.06127",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/lab-result-codes.rq",
            "value": 0.211331,
            "range": "0.211331-0.211331",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/lab-result-identifiers.rq",
            "value": 0.078806,
            "range": "0.078806-0.078806",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/lab-result-performer.rq",
            "value": 0.02221,
            "range": "0.02221-0.02221",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/lab-result-quantities.rq",
            "value": 0.19655,
            "range": "0.19655-0.19655",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/lab-result.rq",
            "value": 0.248768,
            "range": "0.248768-0.248768",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/medication-dispense-prescription.rq",
            "value": 0.077037,
            "range": "0.077037-0.077037",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/medication-dispense.rq",
            "value": 0.053578,
            "range": "0.053578-0.053578",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/medication-supply-and-dosages.rq",
            "value": 0.07914,
            "range": "0.07914-0.07914",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/medication.rq",
            "value": 0.13178,
            "range": "0.13178-0.13178",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/narrative-text.rq",
            "value": 0.456605,
            "range": "0.456605-0.456605",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/note-or-document.rq",
            "value": 0.144452,
            "range": "0.144452-0.144452",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/organization-from-server.rq",
            "value": 0.033517,
            "range": "0.033517-0.033517",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/organization.rq",
            "value": 0.08482,
            "range": "0.08482-0.08482",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/patient.rq",
            "value": 0.051888,
            "range": "0.051888-0.051888",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/procedure.rq",
            "value": 0.148147,
            "range": "0.148147-0.148147",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/record.rq",
            "value": 1.277605,
            "range": "1.277605-1.277605",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/report-and-note-performer.rq",
            "value": 0.023919,
            "range": "0.023919-0.023919",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/report-results.rq",
            "value": 0.197711,
            "range": "0.197711-0.197711",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/report.rq",
            "value": 0.188158,
            "range": "0.188158-0.188158",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/variant-interpretation.rq",
            "value": 0.062096,
            "range": "0.062096-0.062096",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/variant.rq",
            "value": 0.056806,
            "range": "0.056806-0.056806",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/absent-reason-findings.rq",
            "value": 0.21638,
            "range": "0.21638-0.21638",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/category-text-findings.rq",
            "value": 0.098222,
            "range": "0.098222-0.098222",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/code-findings.rq",
            "value": 0.101537,
            "range": "0.101537-0.101537",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/date-findings.rq",
            "value": 0.199158,
            "range": "0.199158-0.199158",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/document-version-findings.rq",
            "value": 0.009353,
            "range": "0.009353-0.009353",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/lab-result-findings.rq",
            "value": 0.29659,
            "range": "0.29659-0.29659",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/medication-dispense-findings.rq",
            "value": 0.054656,
            "range": "0.054656-0.054656",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/medication-dispense-prescription-findings.rq",
            "value": 0.062126,
            "range": "0.062126-0.062126",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/medication-findings.rq",
            "value": 0.719247,
            "range": "0.719247-0.719247",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/narrative-findings.rq",
            "value": 0.055036,
            "range": "0.055036-0.055036",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/observation-component-findings.rq",
            "value": 0.094771,
            "range": "0.094771-0.094771",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/observation-findings.rq",
            "value": 0.214268,
            "range": "0.214268-0.214268",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/organization-findings.rq",
            "value": 0.054245,
            "range": "0.054245-0.054245",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/patient-findings.rq",
            "value": 0.049606,
            "range": "0.049606-0.049606",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/patient-reference-findings.rq",
            "value": 0.209487,
            "range": "0.209487-0.209487",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/recorded-date-findings.rq",
            "value": 0.060693,
            "range": "0.060693-0.060693",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/reference-display-findings.rq",
            "value": 0.231217,
            "range": "0.231217-0.231217",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/reference-without-server-findings.rq",
            "value": 0.100193,
            "range": "0.100193-0.100193",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/repeated-id-findings.rq",
            "value": 0.103343,
            "range": "0.103343-0.103343",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/report-and-note-findings.rq",
            "value": 0.120159,
            "range": "0.120159-0.120159",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/report-result-findings.rq",
            "value": 0.101748,
            "range": "0.101748-0.101748",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/resource-type-findings.rq",
            "value": 0.80958,
            "range": "0.80958-0.80958",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/variant-findings.rq",
            "value": 0.091617,
            "range": "0.091617-0.091617",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/variant-interpretation-findings.rq",
            "value": 0.06589,
            "range": "0.06589-0.06589",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/query-time/detect.rq",
            "value": 0.009265,
            "range": "0.009265-0.009265",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime/stage/document-table",
            "value": 1.468378,
            "range": "1.468378-1.468378",
            "unit": "s",
            "extra": "Running the adapter's document-table query on every record of the document, to link a record to any other record, in the document or not. (Specification stage: link records.)"
          },
          {
            "name": "fhir-r4-synthea-lifetime/stage/lift",
            "value": 4.951154,
            "range": "4.951154-4.951154",
            "unit": "s",
            "extra": "Reading the source document into the triples the adapter's queries can ask about, one record at a time. (Specification stage: read and chunk.)"
          },
          {
            "name": "fhir-r4-synthea-lifetime/stage/source-validation",
            "value": 8.470481,
            "range": "8.470481-8.470481",
            "unit": "s",
            "extra": "Checking each source record, and the document as a whole where the adapter gives a document schema, against the adapter's schema (XSD or JSON Schema). A failure is a finding, never a refusal. (Specification stage: check, validate.)"
          },
          {
            "name": "fhir-r4-synthea-lifetime/stage/accounting",
            "value": 2.486682,
            "range": "2.486682-2.486682",
            "unit": "s",
            "extra": "Working out, against the adapter's accounting file, what became of each part of each source record."
          },
          {
            "name": "fhir-r4-synthea-lifetime/stage/dataset",
            "value": 0.06236,
            "range": "0.06236-0.06236",
            "unit": "s",
            "extra": "Putting the facts about a record into the store its queries run over."
          },
          {
            "name": "fhir-r4-synthea-lifetime/stage/index",
            "value": 2.378112,
            "range": "2.378112-2.378112",
            "unit": "s",
            "extra": "Building the lookup structure that compiled queries read, from a record's triples. A query the general evaluator answers needs none."
          },
          {
            "name": "fhir-r4-synthea-lifetime/stage/mapping",
            "value": 4.832258,
            "range": "4.832258-4.832258",
            "unit": "s",
            "extra": "Running the adapter's mapping queries on each record, without the time of building their lookup structure. (Specification stage: transform.)"
          },
          {
            "name": "fhir-r4-synthea-lifetime/stage/records",
            "value": 0.934571,
            "range": "0.934571-0.934571",
            "unit": "s",
            "extra": "Turning what the mapping produced into each record's graph: naming its versions, keeping one where the mapping wrote several, and writing their arrivals. (Specification stage: name versions, write arrivals.)"
          },
          {
            "name": "fhir-r4-synthea-lifetime/stage/output-validation",
            "value": 15.284812,
            "range": "15.284812-15.284812",
            "unit": "s",
            "extra": "Checking each record's output against the vocabulary's SHACL shapes."
          },
          {
            "name": "fhir-r4-synthea-lifetime/stage/findings",
            "value": 5.287854,
            "range": "5.287854-5.287854",
            "unit": "s",
            "extra": "Running the adapter's findings queries to find what the source record got wrong or left out. (Specification stage: findings.)"
          },
          {
            "name": "fhir-r4-synthea-lifetime/stage/unresolved-addresses",
            "value": 0.319938,
            "range": "0.319938-0.319938",
            "unit": "s",
            "extra": "Finding the references to records the document does not hold, which keep their computed names."
          },
          {
            "name": "fhir-r4-synthea-lifetime/stage/detect",
            "value": 0.009424,
            "range": "0.009424-0.009424",
            "unit": "s",
            "extra": "Running the adapter's detect query, which reports whether the adapter reads this document. The Bridge converts every record whatever the answer; it never routes."
          },
          {
            "name": "fhir-r4-synthea-lifetime/stage/summary",
            "value": 0.121773,
            "range": "0.121773-0.121773",
            "unit": "s",
            "extra": "Counting the graph's distinct triples and the findings once every record is done."
          },
          {
            "name": "fhir-r4-synthea-lifetime/stage/serialising",
            "value": 7.61886,
            "range": "7.61886-7.61886",
            "unit": "s",
            "extra": "Writing the graph and the findings out as Turtle or N-Triples."
          },
          {
            "name": "fhir-r4-synthea-lifetime/stage/unaccounted",
            "value": 3.269902,
            "range": "3.269902-3.269902",
            "unit": "s",
            "extra": "Seconds of the profiled convert that none of the stages accounts for, such as writing the document and the import into the graph."
          },
          {
            "name": "fhir-r4-synthea-lifetime/output/graph",
            "value": 184332,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph, with a SHA-256 fingerprint of the output that is the same whenever the content is the same, whatever order the engine wrote it in and whatever it named its blank nodes. sha256 7a293b50f329e9697119ef46a49e0e55365198126021106ead2739fd58656a51"
          },
          {
            "name": "fhir-r4-synthea-lifetime/output/findings",
            "value": 4050084,
            "unit": "{count}",
            "extra": "Distinct triples in the findings, with a SHA-256 fingerprint of them that is the same whenever the content is the same, whatever order the engine wrote them in and whatever it named its blank nodes. sha256 ff18a1907da595ae57df30a0e8d2e3562de67ab8261c8799e7e8b02259945069"
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/convert/first",
            "value": 14.039581,
            "range": "14.0167-14.062866",
            "unit": "s",
            "extra": "Seconds the first convert of a fresh process takes, timed without the step-by-step breakdown. A caller that opens an adapter and converts once also waits for the engine to start (the boot rows) and the adapter to load (the load rows), which this does not count. The lifetime Synthea record of Justa Moore, a 15 MB FHIR R4 Bundle of 3,987 entries. sha256 da8b599d1bafbc500fd74a1aded71f6a426dde36edc25b0fd86856595e9ad292, 14900379 bytes"
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/convert/second",
            "value": 13.208307,
            "range": "13.107269-13.219412",
            "unit": "s",
            "extra": "Seconds a second convert of the same document takes in the same process, timed without the step-by-step breakdown: the warm cost."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/convert/profiled",
            "value": 14.212078,
            "range": "14.212078-14.212078",
            "unit": "s",
            "extra": "Seconds of the one convert run with the step-by-step breakdown on, from which the stage and query rows are read. The breakdown makes it slower than a plain convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/wasm-memory/first",
            "value": 639827968,
            "range": "639827968-639827968",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the first convert is done."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/wasm-memory/second",
            "value": 639827968,
            "range": "639827968-639827968",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the second convert is done."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/rss",
            "value": 999964672,
            "range": "999145472-1060618240",
            "unit": "By",
            "extra": "Peak memory the whole process held in RAM, in bytes: the engine's start, the adapter's load and both converts all count."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/records",
            "value": 3987,
            "unit": "{count}",
            "extra": "Source records the Bridge found in the document and converted. The lifetime Synthea record of Justa Moore, a 15 MB FHIR R4 Bundle of 3,987 entries."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/triples",
            "value": 41528,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/findings",
            "value": 68397,
            "unit": "{count}",
            "extra": "Findings the conversion wrote."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/coding-system-table.rq",
            "value": 0.19706,
            "range": "0.19706-0.19706",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/document-table.rq",
            "value": 0.015163,
            "range": "0.015163-0.015163",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/lab-result-table.rq",
            "value": 0.016136,
            "range": "0.016136-0.016136",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/medication-table.rq",
            "value": 0.018256,
            "range": "0.018256-0.018256",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/note-or-document-table.rq",
            "value": 0.009847,
            "range": "0.009847-0.009847",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/organization-table.rq",
            "value": 0.023324,
            "range": "0.023324-0.023324",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/variant-interpretation-table.rq",
            "value": 0.008995,
            "range": "0.008995-0.008995",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/variant-table.rq",
            "value": 0.008781,
            "range": "0.008781-0.008781",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/absent-reasons.rq",
            "value": 0.122176,
            "range": "0.122176-0.122176",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/allergy-intolerance.rq",
            "value": 0.017805,
            "range": "0.017805-0.017805",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/attachments.rq",
            "value": 0.031745,
            "range": "0.031745-0.031745",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/condition.rq",
            "value": 0.025219,
            "range": "0.025219-0.025219",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/immunization.rq",
            "value": 0.017674,
            "range": "0.017674-0.017674",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/lab-result-codes.rq",
            "value": 0.022186,
            "range": "0.022186-0.022186",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/lab-result-identifiers.rq",
            "value": 0.014102,
            "range": "0.014102-0.014102",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/lab-result-performer.rq",
            "value": 0.005318,
            "range": "0.005318-0.005318",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/lab-result-quantities.rq",
            "value": 0.024722,
            "range": "0.024722-0.024722",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/lab-result.rq",
            "value": 0.025814,
            "range": "0.025814-0.025814",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/medication-dispense-prescription.rq",
            "value": 0.017592,
            "range": "0.017592-0.017592",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/medication-dispense.rq",
            "value": 0.012712,
            "range": "0.012712-0.012712",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/medication-supply-and-dosages.rq",
            "value": 0.015909,
            "range": "0.015909-0.015909",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/medication.rq",
            "value": 0.017911,
            "range": "0.017911-0.017911",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/narrative-text.rq",
            "value": 0.113534,
            "range": "0.113534-0.113534",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/note-or-document.rq",
            "value": 0.043653,
            "range": "0.043653-0.043653",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/organization-from-server.rq",
            "value": 0.008059,
            "range": "0.008059-0.008059",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/organization.rq",
            "value": 0.020242,
            "range": "0.020242-0.020242",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/patient.rq",
            "value": 0.012508,
            "range": "0.012508-0.012508",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/procedure.rq",
            "value": 0.07159,
            "range": "0.07159-0.07159",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/record.rq",
            "value": 0.278533,
            "range": "0.278533-0.278533",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/report-and-note-performer.rq",
            "value": 0.005888,
            "range": "0.005888-0.005888",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/report-results.rq",
            "value": 0.027113,
            "range": "0.027113-0.027113",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/report.rq",
            "value": 0.054041,
            "range": "0.054041-0.054041",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/variant-interpretation.rq",
            "value": 0.014775,
            "range": "0.014775-0.014775",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/variant.rq",
            "value": 0.01293,
            "range": "0.01293-0.01293",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/absent-reason-findings.rq",
            "value": 0.051739,
            "range": "0.051739-0.051739",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/category-text-findings.rq",
            "value": 0.024191,
            "range": "0.024191-0.024191",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/code-findings.rq",
            "value": 0.03214,
            "range": "0.03214-0.03214",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/date-findings.rq",
            "value": 0.043121,
            "range": "0.043121-0.043121",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/document-version-findings.rq",
            "value": 0.002134,
            "range": "0.002134-0.002134",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/lab-result-findings.rq",
            "value": 0.030553,
            "range": "0.030553-0.030553",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/medication-dispense-findings.rq",
            "value": 0.012082,
            "range": "0.012082-0.012082",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/medication-dispense-prescription-findings.rq",
            "value": 0.014806,
            "range": "0.014806-0.014806",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/medication-findings.rq",
            "value": 0.166803,
            "range": "0.166803-0.166803",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/narrative-findings.rq",
            "value": 0.012982,
            "range": "0.012982-0.012982",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/observation-component-findings.rq",
            "value": 0.030693,
            "range": "0.030693-0.030693",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/observation-findings.rq",
            "value": 0.037468,
            "range": "0.037468-0.037468",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/organization-findings.rq",
            "value": 0.012297,
            "range": "0.012297-0.012297",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/patient-findings.rq",
            "value": 0.011905,
            "range": "0.011905-0.011905",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/patient-reference-findings.rq",
            "value": 0.048379,
            "range": "0.048379-0.048379",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/recorded-date-findings.rq",
            "value": 0.014619,
            "range": "0.014619-0.014619",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/reference-display-findings.rq",
            "value": 0.058003,
            "range": "0.058003-0.058003",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/reference-without-server-findings.rq",
            "value": 0.023366,
            "range": "0.023366-0.023366",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/repeated-id-findings.rq",
            "value": 0.022014,
            "range": "0.022014-0.022014",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/report-and-note-findings.rq",
            "value": 0.032849,
            "range": "0.032849-0.032849",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/report-result-findings.rq",
            "value": 0.019706,
            "range": "0.019706-0.019706",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/resource-type-findings.rq",
            "value": 0.026874,
            "range": "0.026874-0.026874",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/variant-findings.rq",
            "value": 0.019752,
            "range": "0.019752-0.019752",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/variant-interpretation-findings.rq",
            "value": 0.015187,
            "range": "0.015187-0.015187",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/query-time/detect.rq",
            "value": 0.009263,
            "range": "0.009263-0.009263",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/stage/document-table",
            "value": 0.421525,
            "range": "0.421525-0.421525",
            "unit": "s",
            "extra": "Running the adapter's document-table query on every record of the document, to link a record to any other record, in the document or not. (Specification stage: link records.)"
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/stage/lift",
            "value": 1.470552,
            "range": "1.470552-1.470552",
            "unit": "s",
            "extra": "Reading the source document into the triples the adapter's queries can ask about, one record at a time. (Specification stage: read and chunk.)"
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/stage/source-validation",
            "value": 2.130763,
            "range": "2.130763-2.130763",
            "unit": "s",
            "extra": "Checking each source record, and the document as a whole where the adapter gives a document schema, against the adapter's schema (XSD or JSON Schema). A failure is a finding, never a refusal. (Specification stage: check, validate.)"
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/stage/accounting",
            "value": 0.635426,
            "range": "0.635426-0.635426",
            "unit": "s",
            "extra": "Working out, against the adapter's accounting file, what became of each part of each source record."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/stage/dataset",
            "value": 0.016557,
            "range": "0.016557-0.016557",
            "unit": "s",
            "extra": "Putting the facts about a record into the store its queries run over."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/stage/index",
            "value": 0.663682,
            "range": "0.663682-0.663682",
            "unit": "s",
            "extra": "Building the lookup structure that compiled queries read, from a record's triples. A query the general evaluator answers needs none."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/stage/mapping",
            "value": 1.075451,
            "range": "1.075451-1.075451",
            "unit": "s",
            "extra": "Running the adapter's mapping queries on each record, without the time of building their lookup structure. (Specification stage: transform.)"
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/stage/records",
            "value": 0.209348,
            "range": "0.209348-0.209348",
            "unit": "s",
            "extra": "Turning what the mapping produced into each record's graph: naming its versions, keeping one where the mapping wrote several, and writing their arrivals. (Specification stage: name versions, write arrivals.)"
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/stage/output-validation",
            "value": 3.750201,
            "range": "3.750201-3.750201",
            "unit": "s",
            "extra": "Checking each record's output against the vocabulary's SHACL shapes."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/stage/findings",
            "value": 1.066802,
            "range": "1.066802-1.066802",
            "unit": "s",
            "extra": "Running the adapter's findings queries to find what the source record got wrong or left out. (Specification stage: findings.)"
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/stage/unresolved-addresses",
            "value": 0.105141,
            "range": "0.105141-0.105141",
            "unit": "s",
            "extra": "Finding the references to records the document does not hold, which keep their computed names."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/stage/detect",
            "value": 0.009379,
            "range": "0.009379-0.009379",
            "unit": "s",
            "extra": "Running the adapter's detect query, which reports whether the adapter reads this document. The Bridge converts every record whatever the answer; it never routes."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/stage/summary",
            "value": 0.029201,
            "range": "0.029201-0.029201",
            "unit": "s",
            "extra": "Counting the graph's distinct triples and the findings once every record is done."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/stage/serialising",
            "value": 1.752868,
            "range": "1.752868-1.752868",
            "unit": "s",
            "extra": "Writing the graph and the findings out as Turtle or N-Triples."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/stage/unaccounted",
            "value": 0.875183,
            "range": "0.875183-0.875183",
            "unit": "s",
            "extra": "Seconds of the profiled convert that none of the stages accounts for, such as writing the document and the import into the graph."
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/output/graph",
            "value": 41528,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph, with a SHA-256 fingerprint of the output that is the same whenever the content is the same, whatever order the engine wrote it in and whatever it named its blank nodes. sha256 8e086cb7aab88e92c913f3ca926a715c879be434ff2ea8f59b3d7677a6a406a2"
          },
          {
            "name": "fhir-r4-synthea-lifetime-ab74ec30/output/findings",
            "value": 1041849,
            "unit": "{count}",
            "extra": "Distinct triples in the findings, with a SHA-256 fingerprint of them that is the same whenever the content is the same, whatever order the engine wrote them in and whatever it named its blank nodes. sha256 a7cb12348b0c775e338f31add0447d9a2145600ae8536b5e70a116ff9fd8538a"
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/convert/first",
            "value": 63.124936,
            "range": "62.709681-63.234623",
            "unit": "s",
            "extra": "Seconds the first convert of a fresh process takes, timed without the step-by-step breakdown. A caller that opens an adapter and converts once also waits for the engine to start (the boot rows) and the adapter to load (the load rows), which this does not count. The lifetime Synthea record of Dona Okuneva, a 56 MB FHIR R4 Bundle of 19,728 entries. It is one of the three records of the pod of 42,784 entries, with Xochitl Corkery's and Cole Gaylord's; the three are unrelated people. sha256 4a4a24b9c8a052cc60465fffe5da9f34ab71101f3479ebdcc794e61e6d76d7f2, 56150281 bytes"
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/convert/second",
            "value": 60.41678,
            "range": "60.079551-60.540283",
            "unit": "s",
            "extra": "Seconds a second convert of the same document takes in the same process, timed without the step-by-step breakdown: the warm cost."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/convert/profiled",
            "value": 63.645457,
            "range": "63.645457-63.645457",
            "unit": "s",
            "extra": "Seconds of the one convert run with the step-by-step breakdown on, from which the stage and query rows are read. The breakdown makes it slower than a plain convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/wasm-memory/first",
            "value": 2552430592,
            "range": "2552430592-2552430592",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the first convert is done."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/wasm-memory/second",
            "value": 2552430592,
            "range": "2552430592-2552430592",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the second convert is done."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/rss",
            "value": 3068207104,
            "range": "3060424704-3079499776",
            "unit": "By",
            "extra": "Peak memory the whole process held in RAM, in bytes: the engine's start, the adapter's load and both converts all count."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/records",
            "value": 19728,
            "unit": "{count}",
            "extra": "Source records the Bridge found in the document and converted. The lifetime Synthea record of Dona Okuneva, a 56 MB FHIR R4 Bundle of 19,728 entries. It is one of the three records of the pod of 42,784 entries, with Xochitl Corkery's and Cole Gaylord's; the three are unrelated people."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/triples",
            "value": 283762,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/findings",
            "value": 198775,
            "unit": "{count}",
            "extra": "Findings the conversion wrote."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/coding-system-table.rq",
            "value": 0.726404,
            "range": "0.726404-0.726404",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/document-table.rq",
            "value": 0.067343,
            "range": "0.067343-0.067343",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/lab-result-table.rq",
            "value": 0.093706,
            "range": "0.093706-0.093706",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/medication-table.rq",
            "value": 0.078505,
            "range": "0.078505-0.078505",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/note-or-document-table.rq",
            "value": 0.039825,
            "range": "0.039825-0.039825",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/organization-table.rq",
            "value": 0.109414,
            "range": "0.109414-0.109414",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/variant-interpretation-table.rq",
            "value": 0.046499,
            "range": "0.046499-0.046499",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/variant-table.rq",
            "value": 0.047385,
            "range": "0.047385-0.047385",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/absent-reasons.rq",
            "value": 0.568272,
            "range": "0.568272-0.568272",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/allergy-intolerance.rq",
            "value": 0.089308,
            "range": "0.089308-0.089308",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/attachments.rq",
            "value": 0.119304,
            "range": "0.119304-0.119304",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/condition.rq",
            "value": 0.108649,
            "range": "0.108649-0.108649",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/immunization.rq",
            "value": 0.072705,
            "range": "0.072705-0.072705",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/lab-result-codes.rq",
            "value": 0.402558,
            "range": "0.402558-0.402558",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/lab-result-identifiers.rq",
            "value": 0.113616,
            "range": "0.113616-0.113616",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/lab-result-performer.rq",
            "value": 0.027757,
            "range": "0.027757-0.027757",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/lab-result-quantities.rq",
            "value": 0.35668,
            "range": "0.35668-0.35668",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/lab-result.rq",
            "value": 0.46665,
            "range": "0.46665-0.46665",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/medication-dispense-prescription.rq",
            "value": 0.092695,
            "range": "0.092695-0.092695",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/medication-dispense.rq",
            "value": 0.063901,
            "range": "0.063901-0.063901",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/medication-supply-and-dosages.rq",
            "value": 0.085701,
            "range": "0.085701-0.085701",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/medication.rq",
            "value": 0.123927,
            "range": "0.123927-0.123927",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/narrative-text.rq",
            "value": 0.39066,
            "range": "0.39066-0.39066",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/note-or-document.rq",
            "value": 0.115652,
            "range": "0.115652-0.115652",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/organization-from-server.rq",
            "value": 0.04093,
            "range": "0.04093-0.04093",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/organization.rq",
            "value": 0.099845,
            "range": "0.099845-0.099845",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/patient.rq",
            "value": 0.060731,
            "range": "0.060731-0.060731",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/procedure.rq",
            "value": 0.17264,
            "range": "0.17264-0.17264",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/record.rq",
            "value": 1.936192,
            "range": "1.936192-1.936192",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/report-and-note-performer.rq",
            "value": 0.029215,
            "range": "0.029215-0.029215",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/report-results.rq",
            "value": 0.354453,
            "range": "0.354453-0.354453",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/report.rq",
            "value": 0.218447,
            "range": "0.218447-0.218447",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/variant-interpretation.rq",
            "value": 0.077684,
            "range": "0.077684-0.077684",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/variant.rq",
            "value": 0.068028,
            "range": "0.068028-0.068028",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/absent-reason-findings.rq",
            "value": 0.291341,
            "range": "0.291341-0.291341",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/category-text-findings.rq",
            "value": 0.112754,
            "range": "0.112754-0.112754",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/code-findings.rq",
            "value": 0.112775,
            "range": "0.112775-0.112775",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/date-findings.rq",
            "value": 0.294429,
            "range": "0.294429-0.294429",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/document-version-findings.rq",
            "value": 0.012063,
            "range": "0.012063-0.012063",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/lab-result-findings.rq",
            "value": 0.541936,
            "range": "0.541936-0.541936",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/medication-dispense-findings.rq",
            "value": 0.064216,
            "range": "0.064216-0.064216",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/medication-dispense-prescription-findings.rq",
            "value": 0.074032,
            "range": "0.074032-0.074032",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/medication-findings.rq",
            "value": 0.91665,
            "range": "0.91665-0.91665",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/narrative-findings.rq",
            "value": 0.063661,
            "range": "0.063661-0.063661",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/observation-component-findings.rq",
            "value": 0.19183,
            "range": "0.19183-0.19183",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/observation-findings.rq",
            "value": 0.364756,
            "range": "0.364756-0.364756",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/organization-findings.rq",
            "value": 0.065448,
            "range": "0.065448-0.065448",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/patient-findings.rq",
            "value": 0.05804,
            "range": "0.05804-0.05804",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/patient-reference-findings.rq",
            "value": 0.305156,
            "range": "0.305156-0.305156",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/recorded-date-findings.rq",
            "value": 0.069607,
            "range": "0.069607-0.069607",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/reference-display-findings.rq",
            "value": 0.292805,
            "range": "0.292805-0.292805",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/reference-without-server-findings.rq",
            "value": 0.12446,
            "range": "0.12446-0.12446",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/repeated-id-findings.rq",
            "value": 0.152215,
            "range": "0.152215-0.152215",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/report-and-note-findings.rq",
            "value": 0.119989,
            "range": "0.119989-0.119989",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/report-result-findings.rq",
            "value": 0.159345,
            "range": "0.159345-0.159345",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/resource-type-findings.rq",
            "value": 0.112832,
            "range": "0.112832-0.112832",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/variant-findings.rq",
            "value": 0.129942,
            "range": "0.129942-0.129942",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/variant-interpretation-findings.rq",
            "value": 0.079285,
            "range": "0.079285-0.079285",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/query-time/detect.rq",
            "value": 0.00931,
            "range": "0.00931-0.00931",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/stage/document-table",
            "value": 1.704027,
            "range": "1.704027-1.704027",
            "unit": "s",
            "extra": "Running the adapter's document-table query on every record of the document, to link a record to any other record, in the document or not. (Specification stage: link records.)"
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/stage/lift",
            "value": 5.764107,
            "range": "5.764107-5.764107",
            "unit": "s",
            "extra": "Reading the source document into the triples the adapter's queries can ask about, one record at a time. (Specification stage: read and chunk.)"
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/stage/source-validation",
            "value": 9.610869,
            "range": "9.610869-9.610869",
            "unit": "s",
            "extra": "Checking each source record, and the document as a whole where the adapter gives a document schema, against the adapter's schema (XSD or JSON Schema). A failure is a finding, never a refusal. (Specification stage: check, validate.)"
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/stage/accounting",
            "value": 1.823191,
            "range": "1.823191-1.823191",
            "unit": "s",
            "extra": "Working out, against the adapter's accounting file, what became of each part of each source record."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/stage/dataset",
            "value": 0.075859,
            "range": "0.075859-0.075859",
            "unit": "s",
            "extra": "Putting the facts about a record into the store its queries run over."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/stage/index",
            "value": 2.808652,
            "range": "2.808652-2.808652",
            "unit": "s",
            "extra": "Building the lookup structure that compiled queries read, from a record's triples. A query the general evaluator answers needs none."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/stage/mapping",
            "value": 6.474099,
            "range": "6.474099-6.474099",
            "unit": "s",
            "extra": "Running the adapter's mapping queries on each record, without the time of building their lookup structure. (Specification stage: transform.)"
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/stage/records",
            "value": 1.359981,
            "range": "1.359981-1.359981",
            "unit": "s",
            "extra": "Turning what the mapping produced into each record's graph: naming its versions, keeping one where the mapping wrote several, and writing their arrivals. (Specification stage: name versions, write arrivals.)"
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/stage/output-validation",
            "value": 18.964582,
            "range": "18.964582-18.964582",
            "unit": "s",
            "extra": "Checking each record's output against the vocabulary's SHACL shapes."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/stage/findings",
            "value": 6.131473,
            "range": "6.131473-6.131473",
            "unit": "s",
            "extra": "Running the adapter's findings queries to find what the source record got wrong or left out. (Specification stage: findings.)"
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/stage/unresolved-addresses",
            "value": 0.333942,
            "range": "0.333942-0.333942",
            "unit": "s",
            "extra": "Finding the references to records the document does not hold, which keep their computed names."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/stage/detect",
            "value": 0.009451,
            "range": "0.009451-0.009451",
            "unit": "s",
            "extra": "Running the adapter's detect query, which reports whether the adapter reads this document. The Bridge converts every record whatever the answer; it never routes."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/stage/summary",
            "value": 0.127266,
            "range": "0.127266-0.127266",
            "unit": "s",
            "extra": "Counting the graph's distinct triples and the findings once every record is done."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/stage/serialising",
            "value": 5.083861,
            "range": "5.083861-5.083861",
            "unit": "s",
            "extra": "Writing the graph and the findings out as Turtle or N-Triples."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/stage/unaccounted",
            "value": 3.374097,
            "range": "3.374097-3.374097",
            "unit": "s",
            "extra": "Seconds of the profiled convert that none of the stages accounts for, such as writing the document and the import into the graph."
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/output/graph",
            "value": 283762,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph, with a SHA-256 fingerprint of the output that is the same whenever the content is the same, whatever order the engine wrote it in and whatever it named its blank nodes. sha256 5241acfb6f7cb6de9f7a1d1e19b5c9196c248ef9cd96d821dd26d0b158ad15b4"
          },
          {
            "name": "fhir-r4-synthea-lifetime-3541f47c/output/findings",
            "value": 3011148,
            "unit": "{count}",
            "extra": "Distinct triples in the findings, with a SHA-256 fingerprint of them that is the same whenever the content is the same, whatever order the engine wrote them in and whatever it named its blank nodes. sha256 d9f890a80fc30b79b1fa0345608967aa80eecf5b9acb2c4b52c51e090c1993a9"
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/convert/first",
            "value": 19.929599,
            "range": "19.866986-20.16451",
            "unit": "s",
            "extra": "Seconds the first convert of a fresh process takes, timed without the step-by-step breakdown. A caller that opens an adapter and converts once also waits for the engine to start (the boot rows) and the adapter to load (the load rows), which this does not count. The lifetime Synthea record of Cole Gaylord, a 19 MB FHIR R4 Bundle of 6,223 entries. It is one of the three records of the pod of 42,784 entries, with Xochitl Corkery's and Dona Okuneva's; the three are unrelated people. sha256 843940715d3b2edbdc883697a79ebf312a7754df290d28b0cd89e228549fa678, 19009113 bytes"
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/convert/second",
            "value": 18.988415,
            "range": "18.945963-19.098687",
            "unit": "s",
            "extra": "Seconds a second convert of the same document takes in the same process, timed without the step-by-step breakdown: the warm cost."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/convert/profiled",
            "value": 20.25199,
            "range": "20.25199-20.25199",
            "unit": "s",
            "extra": "Seconds of the one convert run with the step-by-step breakdown on, from which the stage and query rows are read. The breakdown makes it slower than a plain convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/wasm-memory/first",
            "value": 723714048,
            "range": "723714048-723714048",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the first convert is done."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/wasm-memory/second",
            "value": 723714048,
            "range": "723714048-723714048",
            "unit": "By",
            "extra": "Bytes of memory the engine holds once the second convert is done."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/rss",
            "value": 1066934272,
            "range": "1059004416-1100308480",
            "unit": "By",
            "extra": "Peak memory the whole process held in RAM, in bytes: the engine's start, the adapter's load and both converts all count."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/records",
            "value": 6223,
            "unit": "{count}",
            "extra": "Source records the Bridge found in the document and converted. The lifetime Synthea record of Cole Gaylord, a 19 MB FHIR R4 Bundle of 6,223 entries. It is one of the three records of the pod of 42,784 entries, with Xochitl Corkery's and Dona Okuneva's; the three are unrelated people."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/triples",
            "value": 79183,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/findings",
            "value": 72909,
            "unit": "{count}",
            "extra": "Findings the conversion wrote."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/coding-system-table.rq",
            "value": 0.264336,
            "range": "0.264336-0.264336",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/document-table.rq",
            "value": 0.022792,
            "range": "0.022792-0.022792",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/lab-result-table.rq",
            "value": 0.028759,
            "range": "0.028759-0.028759",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/medication-table.rq",
            "value": 0.025822,
            "range": "0.025822-0.025822",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/note-or-document-table.rq",
            "value": 0.013245,
            "range": "0.013245-0.013245",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/organization-table.rq",
            "value": 0.035483,
            "range": "0.035483-0.035483",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/variant-interpretation-table.rq",
            "value": 0.014383,
            "range": "0.014383-0.014383",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/variant-table.rq",
            "value": 0.014602,
            "range": "0.014602-0.014602",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/absent-reasons.rq",
            "value": 0.179717,
            "range": "0.179717-0.179717",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/allergy-intolerance.rq",
            "value": 0.027784,
            "range": "0.027784-0.027784",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/attachments.rq",
            "value": 0.039814,
            "range": "0.039814-0.039814",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/condition.rq",
            "value": 0.040112,
            "range": "0.040112-0.040112",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/immunization.rq",
            "value": 0.02586,
            "range": "0.02586-0.02586",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/lab-result-codes.rq",
            "value": 0.0975,
            "range": "0.0975-0.0975",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/lab-result-identifiers.rq",
            "value": 0.031413,
            "range": "0.031413-0.031413",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/lab-result-performer.rq",
            "value": 0.008784,
            "range": "0.008784-0.008784",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/lab-result-quantities.rq",
            "value": 0.091459,
            "range": "0.091459-0.091459",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/lab-result.rq",
            "value": 0.116372,
            "range": "0.116372-0.116372",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/medication-dispense-prescription.rq",
            "value": 0.027838,
            "range": "0.027838-0.027838",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/medication-dispense.rq",
            "value": 0.01957,
            "range": "0.01957-0.01957",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/medication-supply-and-dosages.rq",
            "value": 0.03209,
            "range": "0.03209-0.03209",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/medication.rq",
            "value": 0.034112,
            "range": "0.034112-0.034112",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/narrative-text.rq",
            "value": 0.128005,
            "range": "0.128005-0.128005",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/note-or-document.rq",
            "value": 0.042651,
            "range": "0.042651-0.042651",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/organization-from-server.rq",
            "value": 0.012327,
            "range": "0.012327-0.012327",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/organization.rq",
            "value": 0.030102,
            "range": "0.030102-0.030102",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/patient.rq",
            "value": 0.018664,
            "range": "0.018664-0.018664",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/procedure.rq",
            "value": 0.056859,
            "range": "0.056859-0.056859",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/record.rq",
            "value": 0.560615,
            "range": "0.560615-0.560615",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/report-and-note-performer.rq",
            "value": 0.009475,
            "range": "0.009475-0.009475",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/report-results.rq",
            "value": 0.095245,
            "range": "0.095245-0.095245",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/report.rq",
            "value": 0.070675,
            "range": "0.070675-0.070675",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/variant-interpretation.rq",
            "value": 0.023759,
            "range": "0.023759-0.023759",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/variant.rq",
            "value": 0.020511,
            "range": "0.020511-0.020511",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/absent-reason-findings.rq",
            "value": 0.085249,
            "range": "0.085249-0.085249",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/category-text-findings.rq",
            "value": 0.035098,
            "range": "0.035098-0.035098",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/code-findings.rq",
            "value": 0.036914,
            "range": "0.036914-0.036914",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/date-findings.rq",
            "value": 0.085667,
            "range": "0.085667-0.085667",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/document-version-findings.rq",
            "value": 0.003312,
            "range": "0.003312-0.003312",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/lab-result-findings.rq",
            "value": 0.134849,
            "range": "0.134849-0.134849",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/medication-dispense-findings.rq",
            "value": 0.020092,
            "range": "0.020092-0.020092",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/medication-dispense-prescription-findings.rq",
            "value": 0.023739,
            "range": "0.023739-0.023739",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/medication-findings.rq",
            "value": 0.282678,
            "range": "0.282678-0.282678",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/narrative-findings.rq",
            "value": 0.019958,
            "range": "0.019958-0.019958",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/observation-component-findings.rq",
            "value": 0.061225,
            "range": "0.061225-0.061225",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/observation-findings.rq",
            "value": 0.100529,
            "range": "0.100529-0.100529",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/organization-findings.rq",
            "value": 0.019408,
            "range": "0.019408-0.019408",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/patient-findings.rq",
            "value": 0.018241,
            "range": "0.018241-0.018241",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/patient-reference-findings.rq",
            "value": 0.091627,
            "range": "0.091627-0.091627",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/recorded-date-findings.rq",
            "value": 0.022263,
            "range": "0.022263-0.022263",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/reference-display-findings.rq",
            "value": 0.086314,
            "range": "0.086314-0.086314",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/reference-without-server-findings.rq",
            "value": 0.038161,
            "range": "0.038161-0.038161",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/repeated-id-findings.rq",
            "value": 0.042672,
            "range": "0.042672-0.042672",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/report-and-note-findings.rq",
            "value": 0.039989,
            "range": "0.039989-0.039989",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/report-result-findings.rq",
            "value": 0.044748,
            "range": "0.044748-0.044748",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/resource-type-findings.rq",
            "value": 0.034999,
            "range": "0.034999-0.034999",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/variant-findings.rq",
            "value": 0.037724,
            "range": "0.037724-0.037724",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/variant-interpretation-findings.rq",
            "value": 0.024251,
            "range": "0.024251-0.024251",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/query-time/detect.rq",
            "value": 0.009285,
            "range": "0.009285-0.009285",
            "unit": "s",
            "extra": "Seconds one of the adapter's queries took, summed over every record it ran on, in the profiled convert."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/stage/document-table",
            "value": 0.587364,
            "range": "0.587364-0.587364",
            "unit": "s",
            "extra": "Running the adapter's document-table query on every record of the document, to link a record to any other record, in the document or not. (Specification stage: link records.)"
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/stage/lift",
            "value": 1.955602,
            "range": "1.955602-1.955602",
            "unit": "s",
            "extra": "Reading the source document into the triples the adapter's queries can ask about, one record at a time. (Specification stage: read and chunk.)"
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/stage/source-validation",
            "value": 3.088957,
            "range": "3.088957-3.088957",
            "unit": "s",
            "extra": "Checking each source record, and the document as a whole where the adapter gives a document schema, against the adapter's schema (XSD or JSON Schema). A failure is a finding, never a refusal. (Specification stage: check, validate.)"
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/stage/accounting",
            "value": 0.665776,
            "range": "0.665776-0.665776",
            "unit": "s",
            "extra": "Working out, against the adapter's accounting file, what became of each part of each source record."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/stage/dataset",
            "value": 0.024823,
            "range": "0.024823-0.024823",
            "unit": "s",
            "extra": "Putting the facts about a record into the store its queries run over."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/stage/index",
            "value": 0.912677,
            "range": "0.912677-0.912677",
            "unit": "s",
            "extra": "Building the lookup structure that compiled queries read, from a record's triples. A query the general evaluator answers needs none."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/stage/mapping",
            "value": 1.908964,
            "range": "1.908964-1.908964",
            "unit": "s",
            "extra": "Running the adapter's mapping queries on each record, without the time of building their lookup structure. (Specification stage: transform.)"
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/stage/records",
            "value": 0.377013,
            "range": "0.377013-0.377013",
            "unit": "s",
            "extra": "Turning what the mapping produced into each record's graph: naming its versions, keeping one where the mapping wrote several, and writing their arrivals. (Specification stage: name versions, write arrivals.)"
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/stage/output-validation",
            "value": 5.774221,
            "range": "5.774221-5.774221",
            "unit": "s",
            "extra": "Checking each record's output against the vocabulary's SHACL shapes."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/stage/findings",
            "value": 1.846732,
            "range": "1.846732-1.846732",
            "unit": "s",
            "extra": "Running the adapter's findings queries to find what the source record got wrong or left out. (Specification stage: findings.)"
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/stage/unresolved-addresses",
            "value": 0.117742,
            "range": "0.117742-0.117742",
            "unit": "s",
            "extra": "Finding the references to records the document does not hold, which keep their computed names."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/stage/detect",
            "value": 0.009411,
            "range": "0.009411-0.009411",
            "unit": "s",
            "extra": "Running the adapter's detect query, which reports whether the adapter reads this document. The Bridge converts every record whatever the answer; it never routes."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/stage/summary",
            "value": 0.03873,
            "range": "0.03873-0.03873",
            "unit": "s",
            "extra": "Counting the graph's distinct triples and the findings once every record is done."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/stage/serialising",
            "value": 1.868879,
            "range": "1.868879-1.868879",
            "unit": "s",
            "extra": "Writing the graph and the findings out as Turtle or N-Triples."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/stage/unaccounted",
            "value": 1.075098,
            "range": "1.075098-1.075098",
            "unit": "s",
            "extra": "Seconds of the profiled convert that none of the stages accounts for, such as writing the document and the import into the graph."
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/output/graph",
            "value": 79183,
            "unit": "{count}",
            "extra": "Distinct triples in the output graph, with a SHA-256 fingerprint of the output that is the same whenever the content is the same, whatever order the engine wrote it in and whatever it named its blank nodes. sha256 4a0efd931e29be8bc7f18e54577b1a79251cdec785961503cfb14c2add98830d"
          },
          {
            "name": "fhir-r4-synthea-lifetime-4f159b52/output/findings",
            "value": 1105859,
            "unit": "{count}",
            "extra": "Distinct triples in the findings, with a SHA-256 fingerprint of them that is the same whenever the content is the same, whatever order the engine wrote them in and whatever it named its blank nodes. sha256 02fa7bd59c0fabcdf16fc0660d673e69298201cfbf1187ffefbe446446487e26"
          },
          {
            "name": "fhir-r4/load",
            "value": 0.877391,
            "range": "0.847202-0.902472",
            "unit": "s",
            "extra": "Seconds to read the adapter's and the vocabulary's files, describe the adapter and prepare it, with the engine already started."
          },
          {
            "name": "fhir-r4/load/read-files",
            "value": 0.012167,
            "range": "0.011376-0.017272",
            "unit": "s",
            "extra": "Seconds of the load spent reading files from disk."
          },
          {
            "name": "fhir-r4/load/prepare",
            "value": 0.865456,
            "range": "0.834777-0.890326",
            "unit": "s",
            "extra": "Seconds of the load spent describing the adapter and preparing it: parsing and compiling its queries, schemas and shapes."
          },
          {
            "name": "fhir-r4/boot/read-wasm",
            "value": 0.004367,
            "range": "0.004081-0.004555",
            "unit": "s",
            "extra": "Seconds to read the engine's compiled code from disk."
          },
          {
            "name": "fhir-r4/boot/compile-wasm",
            "value": 0.021792,
            "range": "0.021461-0.022428",
            "unit": "s",
            "extra": "Seconds to turn the engine's compiled code into something the machine can run."
          },
          {
            "name": "fhir-r4/boot/instantiate-wasm",
            "value": 0.00157,
            "range": "0.001428-0.002074",
            "unit": "s",
            "extra": "Seconds to start the compiled code, which runs its start-up work."
          }
        ]
      }
    ]
  }
}