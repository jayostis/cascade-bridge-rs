use super::xsd::{compile, within, Xsd};
use super::{Schema, SchemaFinding};
use crate::fixtures::files;
use crate::library::{Files, Named};
use crate::lift::xml::Step;
use crate::resolver::Maps;
use quick_xml::events::Event;
use quick_xml::name::ResolveResult;
use quick_xml::NsReader;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::OnceLock;

const ROOT: &str = "https://example.org/cda/";
const V3: &str = "urn:hl7-org:v3";

fn cda() -> &'static Files {
    static CDA: OnceLock<Files> = OnceLock::new();
    CDA.get_or_init(|| files("cda"))
}

thread_local! {
    static SCHEMA: Xsd = compile(
        &format!("{ROOT}schema/infrastructure/cda/CDA_SDTC.xsd"),
        &Maps {
            adapter: Named {
                iri: ROOT,
                files: cda(),
            },
            vocabulary: None,
        },
    )
    .expect("HL7's CDA schema compiles");
}

fn document(path: &str) -> String {
    String::from_utf8(cda()[path].clone()).expect(path)
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
    SCHEMA
        .with(|schema| schema.errors(xml))
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

fn first_v3(local: &str) -> String {
    Step {
        local: local.to_owned(),
        namespace: Some(V3.to_owned()),
        position: 1,
    }
    .write(true)
}

fn address_of_the_element_starting_at(xml: &str, offset: usize) -> String {
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
            return within(&steps).unwrap_or_default();
        }
        if empty {
            steps.pop();
        } else {
            siblings.push(HashMap::new());
        }
    }
}

#[test]
fn holds_each_third_party_file_as_published() {
    let published = [
        (
            "conformance/allergies-section.xml",
            "67b1bce6d308ab7c8eaec8e37fa4d2da93bfe5a94d79dbd00757f703d2bfac1f",
        ),
        (
            "conformance/cerner-summarization.xml",
            "36a0b8ae6b64ba8ca36a536c128a6ce191bf96e4e4dd43a42885f0ab3bf585d2",
        ),
        (
            "conformance/epic-summarization.xml",
            "e6e498ee4c8c1e6198c668f957706f939580faa26c8cb6e1b26770b3e3ea47a2",
        ),
        (
            "conformance/full-summarization.xml",
            "91e9e2aadd457b040582ad8b8fd4a680fb2fe4742c015755c8cb6611d8e54289",
        ),
        (
            "conformance/immunizations-section.xml",
            "bc6c2d9b603f0dcbba34a168494cbd9f075a1313fe8d7cbdee81162a01f10d0a",
        ),
        (
            "conformance/labs-section.xml",
            "2fd607d402a15bae83560ce2f1472070e1373c901de6c794b14038d2d5268f1b",
        ),
        (
            "conformance/narrative-only-section.xml",
            "c26c277a741f9842d2424640ff31b01d8b671b7aba98b45a69edb935422a8681",
        ),
        (
            "hl7-examples/ccd-1.xml",
            "9f75d7df96fb711841c8ce8d71da901e132185ac83290a00bf3bdd4eea008783",
        ),
        (
            "hl7-examples/ccd-2.xml",
            "c5c60ef2281f66a69581ea7671188adb0bc3585c37828470eeb565c778a5970e",
        ),
        (
            "schema/infrastructure/cda/CDA_SDTC.xsd",
            "d596141f0a457b7b31c1a5b4e97ae55d16bedf8c08356e644475c339263d76e7",
        ),
        (
            "schema/infrastructure/cda/POCD_MT000040_SDTC.xsd",
            "a9d1169721efe71124f2c5f7a7d53441a2129424742336e8324f27bce80e1eaf",
        ),
        (
            "schema/infrastructure/cda/SDTC.xsd",
            "3a16dbaa0526005850eaf32f4f061ba4db63ee1f9e1267783c8f734feaf73a58",
        ),
        (
            "schema/processable/coreschemas/NarrativeBlock.xsd",
            "92a9ec2c6c00d10cd40a9afdf4d70f18c823bdec15db9e8b116cb5076d11f66e",
        ),
        (
            "schema/processable/coreschemas/datatypes-base_SDTC.xsd",
            "832527e03eac5cb671880b87c9515e55c2089e8ef3fc82634e69c7337adb440f",
        ),
        (
            "schema/processable/coreschemas/datatypes.xsd",
            "0238ba379eec458d9989ff2c2d9012da2964c9d29d5177cf2d74d4c17a7a6be2",
        ),
        (
            "schema/processable/coreschemas/infrastructureRoot.xsd",
            "dff44f710386745645ffe96c1d46629062e07d4f03b82ef26e9ba180082432c9",
        ),
        (
            "schema/processable/coreschemas/voc.xsd",
            "63bacc8e6c0a662fe630b3377950a1bad8fa659242021a5db0d4778762ae8099",
        ),
    ];
    let held: Vec<(String, String)> = names("")
        .into_iter()
        .filter(|path| path != "NOTICE" && path != ".gitattributes")
        .map(|path| {
            let digest: String = Sha256::digest(&cda()[&path])
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            (path, digest)
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
    let component = first_v3("component");
    let documents = names("conformance/");
    assert_eq!(documents.len(), 7);
    for path in documents {
        assert_eq!(
            shown(&errors(&document(&path))),
            vec![
                ("cvc-complex-type".to_owned(), component.clone()),
                ("cvc-elt".to_owned(), component.clone()),
                ("cvc-complex-type".to_owned(), String::new()),
            ],
            "{path}"
        );
    }
}

#[test]
fn reports_each_fault_of_a_ccd_by_its_rule_within_the_element_that_has_it() {
    let ccd = document("hl7-examples/ccd-1.xml");
    let faults = [
        (
            "a PQ whose value is not a number",
            r#"<value xsi:type="PQ" value="57" unit="a" />"#,
            r#"<value xsi:type="PQ" value="fifty" unit="a" />"#,
            vec![("cvc-simple-type", None)],
        ),
        (
            "a PQ with a child PQ lacks",
            r#"<value xsi:type="PQ" value="57" unit="a" />"#,
            r#"<value xsi:type="PQ" value="57" unit="a"><originalText>x</originalText></value>"#,
            vec![
                ("cvc-complex-type", Some("originalText")),
                ("cvc-elt", Some("originalText")),
            ],
        ),
        (
            "a PQ with an attribute PQ lacks",
            r#"<value xsi:type="PQ" value="57" unit="a" />"#,
            r#"<value xsi:type="PQ" value="57" unit="a" code="x" />"#,
            vec![("cvc-complex-type", None)],
        ),
        (
            "a CD with an attribute CD lacks",
            r#"<value xsi:type="CD" code="304253006""#,
            r#"<value xsi:type="CD" unit="mg" code="304253006""#,
            vec![("cvc-complex-type", None)],
        ),
        (
            "an xsi:type the schema does not define",
            r#"<value xsi:type="PQ" value="57" unit="a" />"#,
            r#"<value xsi:type="NOPE" value="57" unit="a" />"#,
            vec![
                ("cvc-elt", None),
                ("cvc-complex-type", None),
                ("cvc-complex-type", None),
            ],
        ),
        (
            "a value of the abstract ANY, with no xsi:type",
            r#"<value xsi:type="PQ" value="57" unit="a" />"#,
            r#"<value value="57" unit="a" />"#,
            vec![
                ("cvc-type", None),
                ("cvc-complex-type", None),
                ("cvc-complex-type", None),
            ],
        ),
        (
            "an xsi:type not derived from the element's type",
            r#"<observation classCode="OBS" moodCode="EVN">"#,
            r#"<observation classCode="OBS" moodCode="EVN" xsi:type="POCD_MT000040.Act">"#,
            vec![("cvc-elt", None)],
        ),
        (
            "an IVL_TS with a child IVL_TS lacks",
            r#"<effectiveTime xsi:type="IVL_TS">"#,
            r#"<effectiveTime xsi:type="IVL_TS"><bogus/>"#,
            vec![
                ("cvc-complex-type", Some("bogus")),
                ("cvc-elt", Some("bogus")),
            ],
        ),
        (
            "a classCode outside its vocabulary",
            r#"<observation classCode="OBS" moodCode="EVN">"#,
            r#"<observation classCode="BOGUS" moodCode="EVN">"#,
            vec![("cvc-simple-type", None)],
        ),
        (
            "an element narrative text does not allow",
            "<paragraph>Father (deceased)</paragraph>",
            "<paragraph>Father <div>x</div> (deceased)</paragraph>",
            vec![("cvc-complex-type", Some("div")), ("cvc-elt", Some("div"))],
        ),
        (
            "a narrative styleCode outside its list",
            "<paragraph>Father (deceased)</paragraph>",
            "<paragraph styleCode=\"Not A Style!\">Father (deceased)</paragraph>",
            vec![("cvc-datatype-valid", None)],
        ),
        (
            "a narrative ID already used",
            r#"<content ID="immi1" />"#,
            r#"<content ID="immunSect" />"#,
            vec![("cvc-id", None)],
        ),
        (
            "an element the SDTC extensions do not define",
            r#"<sdtc:birthTime value="19750501" />"#,
            r#"<sdtc:bogus value="19750501" />"#,
            vec![("cvc-complex-type", None), ("cvc-elt", None)],
        ),
        (
            "an sdtc:birthTime that is not a timestamp",
            r#"<sdtc:birthTime value="19750501" />"#,
            r#"<sdtc:birthTime value="May 1975" />"#,
            vec![("cvc-pattern-valid", None)],
        ),
    ];
    for (fault, from, to, expected) in faults {
        let offset = ccd.find(from).unwrap_or_else(|| panic!("{fault}: {from}"));
        let faulty = ccd.replacen(from, to, 1);
        let element = address_of_the_element_starting_at(&faulty, offset);
        let expected: Vec<(String, String)> = expected
            .into_iter()
            .map(|(rule, child)| {
                let within = child.map_or(element.clone(), |child| {
                    format!("{element}/{}", first_v3(child))
                });
                (rule.to_owned(), within)
            })
            .collect();
        assert_eq!(shown(&errors(&faulty)), expected, "{fault}");
    }
}
