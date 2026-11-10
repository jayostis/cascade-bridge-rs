use crate::fixtures::{
    annotations, says, tiny, tiny_with_vocabularies, violations, BRIDGE, OA, SH,
};
use crate::load::load_adapter;
use crate::measure::{Skip, Stage};
use crate::query::Profile;
use crate::resolver::Resolver;
use crate::run::{convert, convert_with, prepare, prepare_skipping, Conversion, Prepared, Source};
use oxrdf::Quad;
use std::time::{Duration, Instant};

const EX: &str = "urn:example:catalog#";

fn from_the_shapes(findings: &[Quad]) -> usize {
    annotations(findings)
        .iter()
        .map(|annotation| says(findings, annotation, &format!("{OA}hasBody")))
        .filter(|body| body.starts_with(SH) || *body == format!("{BRIDGE}predicateNotDeclared"))
        .count()
}

/// The accounting's findings name the same gaps, but carry the value they found.
fn from_the_queries(findings: &[Quad]) -> usize {
    annotations(findings)
        .iter()
        .filter(|annotation| says(findings, annotation, &format!("{SH}value")).is_empty())
        .filter(|annotation| says(findings, annotation, &format!("{OA}hasBody")).starts_with(EX))
        .count()
}

fn prepared_skipping(resolver: &dyn Resolver, skipping: &[Skip]) -> Prepared {
    let adapter = load_adapter(resolver).expect("adapter");
    prepare_skipping(&adapter, resolver, skipping).expect("prepared")
}

struct Input {
    iri: String,
    bytes: Vec<u8>,
}

impl Input {
    fn of(resolver: &dyn Resolver, name: &str) -> Self {
        let iri = format!("{}fixtures/in/{name}", resolver.root());
        let bytes = resolver.read(&iri).expect("input");
        Self { iri, bytes }
    }

    fn source(&self) -> Source<'_> {
        Source {
            iri: &self.iri,
            envelope: None,
            bytes: &self.bytes,
            facts: None,
        }
    }
}

fn converted_skipping(resolver: &dyn Resolver, name: &str, skipping: &[Skip]) -> Conversion {
    convert(
        &prepared_skipping(resolver, skipping),
        Input::of(resolver, name).source(),
    )
    .expect("conversion")
}

struct Left {
    skip: Skip,
    vocabularies: bool,
    input: &'static str,
    kind: fn(&[Quad]) -> usize,
    loaded: fn(&Prepared) -> bool,
}

impl Left {
    fn adapter(&self) -> Box<dyn Resolver> {
        if self.vocabularies {
            Box::new(tiny_with_vocabularies())
        } else {
            Box::new(tiny())
        }
    }
}

const LEFT_OUT: [Left; 3] = [
    Left {
        skip: Skip::Schema,
        vocabularies: false,
        input: "invalid.xml",
        kind: |findings| violations(findings).len(),
        loaded: |prepared| !prepared.source_schemas.compiled.is_empty(),
    },
    Left {
        skip: Skip::Shapes,
        vocabularies: true,
        input: "output-fails-a-shape.xml",
        kind: from_the_shapes,
        loaded: |prepared| prepared.vocabulary.is_some(),
    },
    Left {
        skip: Skip::Findings,
        vocabularies: false,
        input: "two.xml",
        kind: from_the_queries,
        loaded: |prepared| !prepared.findings_queries.is_empty(),
    },
];

fn profiled(prepared: &Prepared, input: &Input, kept: bool) -> (Duration, Profile) {
    let profile = Profile::new(Default::default(), kept, Default::default());
    let started = Instant::now();
    convert_with(prepared, input.source(), &profile).expect("conversion");
    (started.elapsed(), profile)
}

mod tests {
    use super::*;

    /// Measuring a load and a convert · load_skipping(), Loaded::convert_profiled(), Stage, Skip
    mod measuring_a_load_and_a_convert {
        use super::*;

        /// `convert_profiled(document)`
        mod convert_profiled {
            use super::*;

            /// times each stage within the convert, where a plain convert times none
            #[test]
            fn times_each_stage_within_the_convert_where_a_plain_convert_times_none() {
                let resolver = tiny_with_vocabularies();
                let prepared = prepared_skipping(&resolver, &[]);
                let input = Input::of(&resolver, "two.xml");
                let (took, profile) = profiled(&prepared, &input, true);
                let stages = profile.stages();
                let reached: Vec<Stage> = stages.iter().map(|time| time.stage).collect();
                for stage in [
                    Stage::Lift,
                    Stage::SourceValidation,
                    Stage::Mapping,
                    Stage::OutputValidation,
                    Stage::Findings,
                    Stage::UnresolvedAddresses,
                    Stage::Detect,
                ] {
                    assert!(reached.contains(&stage), "{stage:?} in {reached:?}");
                }
                let timed: Duration = stages.iter().map(|time| time.elapsed).sum();
                assert!(timed <= took, "{timed:?} of {took:?}");
                let queries = profile.queries();
                for query in &queries {
                    let stage = query.stage.expect("a query run in a stage");
                    let spent = stages.iter().find(|time| time.stage == stage);
                    assert!(
                        spent.is_some_and(|time| query.elapsed <= time.elapsed),
                        "{} of {stage:?}: {spent:?}",
                        query.iri
                    );
                }
                assert!(profiled(&prepared, &input, false).1.stages().is_empty());
                assert!(
                    prepare(&load_adapter(&resolver).expect("adapter"), &resolver)
                        .expect("prepared")
                        .loading
                        .is_empty()
                );
            }
        }

        /// `load_skipping(adapter, vocabulary, skips)`
        mod load_skipping {
            use super::*;

            /// leaves out a skipped stage's findings, and keeps the graph and every other finding
            #[test]
            fn leaves_out_a_skipped_stage_s_findings_and_keeps_the_graph_and_every_other_finding() {
                for left in &LEFT_OUT {
                    let adapter = left.adapter();
                    let whole = converted_skipping(adapter.as_ref(), left.input, &[]);
                    let skipped = converted_skipping(adapter.as_ref(), left.input, &[left.skip]);
                    let name = left.skip.as_str();
                    let (before, after) =
                        ((left.kind)(&whole.findings), (left.kind)(&skipped.findings));
                    assert!(before > 0, "{name}: nothing to leave out");
                    assert_eq!(after, 0, "{name}");
                    assert_eq!(
                        annotations(&skipped.findings).len(),
                        annotations(&whole.findings).len() - before,
                        "{name}: findings of another kind changed"
                    );
                    assert_eq!(
                        skipped.triples(),
                        whole.triples(),
                        "{name}: the graph changed"
                    );
                }
            }

            /// loads nothing a skipped stage would have run, where a default load loads it
            #[test]
            fn loads_nothing_a_skipped_stage_would_have_run_where_a_default_load_loads_it() {
                for left in &LEFT_OUT {
                    let adapter = left.adapter();
                    let name = left.skip.as_str();
                    assert!(
                        (left.loaded)(&prepared_skipping(adapter.as_ref(), &[])),
                        "{name}"
                    );
                    assert!(
                        !(left.loaded)(&prepared_skipping(adapter.as_ref(), &[left.skip])),
                        "{name}"
                    );
                }
            }

            /// names in `Conversion::skipped` each skip once, and nothing for a default load
            #[test]
            fn names_in_conversion_skipped_each_skip_once_and_nothing_for_a_default_load() {
                assert_eq!(prepared_skipping(&tiny(), &[]).skipped, []);
                assert_eq!(
                    prepared_skipping(&tiny(), &[Skip::Findings, Skip::Schema, Skip::Findings])
                        .skipped,
                    [Skip::Schema, Skip::Findings]
                );
            }
        }
    }
}
