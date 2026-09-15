// A finding member is the lexical form of the term its variable is bound to:
// an IRI's own characters, or a literal's lexical form without its datatype or
// language tag. An unbound variable or a blank node has no such string and is
// an error in the query (the specification's engine/sparql.md, step 4). The
// tiny adapter's findings query is rewritten so ?sourceField is bound each way.
use cascade_bridge::{convert, load_adapter, prepare, DirectoryResolver, Resolver};
use std::path::PathBuf;

/// The tiny adapter with the line binding ?sourceField in its findings query
/// replaced.
struct SourceField {
    directory: DirectoryResolver,
    binding: &'static str,
}

impl Resolver for SourceField {
    fn root(&self) -> &str {
        self.directory.root()
    }

    fn read(&self, iri: &str) -> cascade_bridge::Result<Vec<u8>> {
        let bytes = self.directory.read(iri)?;
        if !iri.ends_with("mapping/item-findings.rq") {
            return Ok(bytes);
        }
        let text = String::from_utf8(bytes).expect("utf-8");
        let line = r#"BIND("item/title" AS ?sourceField)"#;
        assert!(text.contains(line), "the findings query binds {line}");
        Ok(text.replace(line, self.binding).into_bytes())
    }
}

/// Each finding's sourceField, from the tiny adapter's two-item input, where
/// one item has no title and so draws one finding.
fn source_fields(binding: &'static str) -> cascade_bridge::Result<Vec<String>> {
    let resolver = SourceField {
        directory: DirectoryResolver::new(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/tiny-adapter"),
        )
        .expect("resolver"),
        binding,
    };
    let adapter = load_adapter(&resolver).expect("adapter");
    let prepared = prepare(&adapter, &resolver).expect("prepared");
    let xml = resolver
        .read(&format!("{}fixtures/in/two.xml", resolver.root()))
        .expect("input");
    let conversion = convert(&prepared, &xml)?;
    Ok(conversion
        .findings
        .into_iter()
        .map(|finding| finding.source_field)
        .collect())
}

#[test]
fn writes_a_member_bound_to_an_iri_as_the_iri() {
    assert_eq!(
        source_fields("BIND(xyz:title AS ?sourceField)").expect("findings"),
        ["http://sparql.xyz/facade-x/data/title"]
    );
}

#[test]
fn writes_a_literal_member_without_its_language_tag() {
    assert_eq!(
        source_fields(r#"BIND("item/title"@en AS ?sourceField)"#).expect("findings"),
        ["item/title"]
    );
}

#[test]
fn refuses_a_member_bound_to_a_blank_node() {
    let error = source_fields("BIND(BNODE() AS ?sourceField)").expect_err("refused");
    assert!(error.to_string().contains("?sourceField"), "{error}");
}

#[test]
fn refuses_a_member_left_unbound() {
    let error = source_fields("").expect_err("refused");
    assert!(error.to_string().contains("?sourceField"), "{error}");
}
