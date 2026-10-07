use super::xsd::{compile, Xsd};
use super::{Schema, SchemaFinding};
use crate::fixtures::files;
use crate::library::{Files, Named};
use crate::lift::xml::Step;
use crate::resolver::{Maps, Resolver};
use quick_xml::events::Event;
use quick_xml::name::ResolveResult;
use quick_xml::NsReader;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

const ROOT: &str = "https://example.org/cda/";
const V3: &str = "urn:hl7-org:v3";

fn cda() -> &'static Files {
    static CDA: OnceLock<Files> = OnceLock::new();
    CDA.get_or_init(|| files("cda"))
}

fn schema() -> &'static Mutex<Xsd> {
    static SCHEMA: OnceLock<Mutex<Xsd>> = OnceLock::new();
    SCHEMA.get_or_init(|| {
        let maps = Maps {
            adapter: Named {
                iri: ROOT,
                files: cda(),
            },
            vocabulary: None,
        };
        Mutex::new(
            compile(
                &format!("{ROOT}schema/infrastructure/cda/CDA_SDTC.xsd"),
                &maps,
            )
            .expect("HL7's CDA schema compiles"),
        )
    })
}

fn document(path: &str) -> String {
    let maps = Maps {
        adapter: Named {
            iri: ROOT,
            files: cda(),
        },
        vocabulary: None,
    };
    String::from_utf8(maps.read(&format!("{ROOT}{path}")).expect(path)).expect(path)
}

fn names(directory: &str) -> Vec<String> {
    let mut names: Vec<String> = cda()
        .keys()
        .filter(|path| path.starts_with(directory))
        .cloned()
        .collect();
    names.sort();
    names
}

fn errors(xml: &str) -> Vec<SchemaFinding> {
    schema()
        .lock()
        .expect("no test panicked while validating")
        .errors(xml)
        .expect("a well-formed document")
}

fn rule(finding: &SchemaFinding) -> &str {
    finding.body().rsplit_once('#').map_or("", |(_, rule)| rule)
}

fn shown(findings: &[SchemaFinding]) -> Vec<(String, String)> {
    findings
        .iter()
        .map(|finding| {
            (
                rule(finding).to_owned(),
                finding.within().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

/// The address a finding gives the element whose start tag begins at `offset`.
fn address_at(xml: &str, offset: usize) -> String {
    let mut reader = NsReader::from_str(xml);
    let mut steps: Vec<Step> = Vec::new();
    let mut siblings: Vec<HashMap<(String, Option<String>), usize>> = vec![HashMap::new()];
    loop {
        let at = usize::try_from(reader.buffer_position()).expect("an offset");
        let (namespace, event) = reader.read_resolved_event().expect("well-formed");
        let (start, empty) = match event {
            Event::Start(start) => (start, false),
            Event::Empty(start) => (start, true),
            Event::End(_) => {
                steps.pop();
                siblings.pop();
                continue;
            }
            Event::Eof => panic!("no element starts at {offset}"),
            _ => continue,
        };
        let namespace = match namespace {
            ResolveResult::Bound(namespace) => {
                Some(String::from_utf8_lossy(namespace.as_ref()).into_owned())
            }
            _ => None,
        };
        let local = String::from_utf8_lossy(start.local_name().as_ref()).into_owned();
        let seen = siblings
            .last_mut()
            .expect("a parent")
            .entry((local.clone(), namespace.clone()))
            .or_default();
        *seen += 1;
        steps.push(Step {
            local,
            namespace,
            position: *seen,
        });
        if at == offset {
            return steps[1..]
                .iter()
                .map(|step| step.write(true))
                .collect::<Vec<String>>()
                .join("/");
        }
        if empty {
            steps.pop();
        } else {
            siblings.push(HashMap::new());
        }
    }
}

#[test]
fn holds_hl7_s_cda_schema_with_the_sdtc_extensions_as_published() {
    let published = [
        (
            "infrastructure/cda/CDA_SDTC.xsd",
            "d596141f0a457b7b31c1a5b4e97ae55d16bedf8c08356e644475c339263d76e7",
        ),
        (
            "infrastructure/cda/POCD_MT000040_SDTC.xsd",
            "a9d1169721efe71124f2c5f7a7d53441a2129424742336e8324f27bce80e1eaf",
        ),
        (
            "infrastructure/cda/SDTC.xsd",
            "3a16dbaa0526005850eaf32f4f061ba4db63ee1f9e1267783c8f734feaf73a58",
        ),
        (
            "processable/coreschemas/NarrativeBlock.xsd",
            "92a9ec2c6c00d10cd40a9afdf4d70f18c823bdec15db9e8b116cb5076d11f66e",
        ),
        (
            "processable/coreschemas/datatypes-base_SDTC.xsd",
            "832527e03eac5cb671880b87c9515e55c2089e8ef3fc82634e69c7337adb440f",
        ),
        (
            "processable/coreschemas/datatypes.xsd",
            "0238ba379eec458d9989ff2c2d9012da2964c9d29d5177cf2d74d4c17a7a6be2",
        ),
        (
            "processable/coreschemas/infrastructureRoot.xsd",
            "dff44f710386745645ffe96c1d46629062e07d4f03b82ef26e9ba180082432c9",
        ),
        (
            "processable/coreschemas/voc.xsd",
            "63bacc8e6c0a662fe630b3377950a1bad8fa659242021a5db0d4778762ae8099",
        ),
    ];
    let held: Vec<(String, String)> = names("schema/")
        .into_iter()
        .map(|path| {
            let digest: String = Sha256::digest(&cda()[&path])
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            (path["schema/".len()..].to_owned(), digest)
        })
        .collect();
    let mut expected: Vec<(String, String)> = published
        .iter()
        .map(|(path, digest)| ((*path).to_owned(), (*digest).to_owned()))
        .collect();
    expected.sort();
    assert_eq!(held, expected);
}

#[test]
fn finds_nothing_in_hl7_s_example_ccds() {
    let examples = names("hl7-examples/");
    assert_eq!(examples.len(), 2);
    for example in examples {
        assert_eq!(
            shown(&errors(&document(&example))),
            Vec::<(String, String)>::new(),
            "{example}"
        );
    }
}

#[test]
fn reports_a_conformance_document_with_no_author_or_custodian_where_its_body_begins() {
    let component = Step {
        local: "component".to_owned(),
        namespace: Some(V3.to_owned()),
        position: 1,
    }
    .write(true);
    let documents = names("conformance/");
    assert_eq!(documents.len(), 7);
    for path in documents {
        let findings = shown(&errors(&document(&path)));
        assert!(
            findings
                .iter()
                .any(|(rule, within)| rule == "cvc-elt" && *within == component),
            "{path}: {findings:?}"
        );
    }
}

#[test]
fn reports_each_fault_of_a_ccd_by_its_rule_within_the_element_that_has_it() {
    let ccd = document("hl7-examples/ccd-1.xml");
    let faults: [(&str, &str, &str, &str); 14] = [
        (
            "a PQ whose value is not a number",
            r#"<value xsi:type="PQ" value="57" unit="a" />"#,
            r#"<value xsi:type="PQ" value="fifty" unit="a" />"#,
            "cvc-simple-type",
        ),
        (
            "a PQ with a child PQ lacks",
            r#"<value xsi:type="PQ" value="57" unit="a" />"#,
            r#"<value xsi:type="PQ" value="57" unit="a"><originalText>x</originalText></value>"#,
            "cvc-complex-type",
        ),
        (
            "a PQ with an attribute PQ lacks",
            r#"<value xsi:type="PQ" value="57" unit="a" />"#,
            r#"<value xsi:type="PQ" value="57" unit="a" code="x" />"#,
            "cvc-complex-type",
        ),
        (
            "a CD with an attribute CD lacks",
            r#"<value xsi:type="CD" code="304253006""#,
            r#"<value xsi:type="CD" unit="mg" code="304253006""#,
            "cvc-complex-type",
        ),
        (
            "an xsi:type the schema does not define",
            r#"<value xsi:type="PQ" value="57" unit="a" />"#,
            r#"<value xsi:type="NOPE" value="57" unit="a" />"#,
            "cvc-elt",
        ),
        (
            "a value of the abstract ANY, with no xsi:type",
            r#"<value xsi:type="PQ" value="57" unit="a" />"#,
            r#"<value value="57" unit="a" />"#,
            "cvc-type",
        ),
        (
            "an xsi:type not derived from the element's type",
            r#"<observation classCode="OBS" moodCode="EVN">"#,
            r#"<observation classCode="OBS" moodCode="EVN" xsi:type="POCD_MT000040.Act">"#,
            "cvc-elt",
        ),
        (
            "an IVL_TS with a child IVL_TS lacks",
            r#"<effectiveTime xsi:type="IVL_TS">"#,
            r#"<effectiveTime xsi:type="IVL_TS"><bogus/>"#,
            "cvc-complex-type",
        ),
        (
            "a classCode outside its vocabulary",
            r#"<observation classCode="OBS" moodCode="EVN">"#,
            r#"<observation classCode="BOGUS" moodCode="EVN">"#,
            "cvc-simple-type",
        ),
        (
            "an element narrative text does not allow",
            "<paragraph>Father (deceased)</paragraph>",
            "<paragraph>Father <div>x</div> (deceased)</paragraph>",
            "cvc-complex-type",
        ),
        (
            "a narrative styleCode outside its list",
            "<paragraph>Father (deceased)</paragraph>",
            "<paragraph styleCode=\"Not A Style!\">Father (deceased)</paragraph>",
            "cvc-datatype-valid",
        ),
        (
            "a narrative ID already used",
            r#"<content ID="immi1" />"#,
            r#"<content ID="immunSect" />"#,
            "cvc-id",
        ),
        (
            "an element the SDTC extensions do not define",
            r#"<sdtc:birthTime value="19750501" />"#,
            r#"<sdtc:bogus value="19750501" />"#,
            "cvc-complex-type",
        ),
        (
            "an sdtc:birthTime that is not a timestamp",
            r#"<sdtc:birthTime value="19750501" />"#,
            r#"<sdtc:birthTime value="May 1975" />"#,
            "cvc-pattern-valid",
        ),
    ];
    for (fault, from, to, expected) in faults {
        let offset = ccd.find(from).unwrap_or_else(|| panic!("{fault}: {from}"));
        let faulty = ccd.replacen(from, to, 1);
        let element = address_at(&faulty, offset);
        let findings = shown(&errors(&faulty));
        assert!(
            findings.iter().any(|(rule, _)| rule == expected),
            "{fault}: {findings:?}"
        );
        assert!(
            findings.iter().all(|(_, within)| *within == element
                || within.starts_with(&format!("{element}/"))),
            "{fault}, at {element}: {findings:?}"
        );
    }
}
