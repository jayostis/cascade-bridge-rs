use super::*;
use crate::compiled::{Dataset, Executor};
use crate::error::{ErrorKind, Explained, Result};
use crate::query::{Form, Profile, Query};
use crate::resolver::Resolver;
use oxigraph::model::{GraphNameRef, NamedNodeRef, QuadRef};
use oxigraph::store::Store;

const QUERY: &str = "urn:t:query.rq";

struct Text;

impl Resolver for Text {
    fn root(&self) -> &str {
        "urn:t:"
    }

    fn read(&self, _: &str) -> Result<Vec<u8>> {
        Ok(b"CONSTRUCT { ?s <urn:t:seen> ?o } WHERE { ?s <urn:t:p> ?o }".to_vec())
    }
}

fn store() -> Store {
    let store = Store::new().expect("a store");
    store
        .insert(QuadRef::new(
            NamedNodeRef::new("urn:t:a").expect("an IRI"),
            NamedNodeRef::new("urn:t:p").expect("an IRI"),
            NamedNodeRef::new("urn:t:b").expect("an IRI"),
            GraphNameRef::DefaultGraph,
        ))
        .expect("inserted");
    store
}

fn stopped_in(watch: &Watch) -> String {
    watch
        .outgrown(QUERY)
        .map(|error| error.message().to_owned())
        .unwrap_or_default()
}

#[test]
fn a_convert_whose_memory_budget_the_heap_passes_is_stopped_and_names_it() {
    let watch = Watch::new(Budgets {
        memory_bytes: 3 * MEBIBYTE,
        ..Budgets::default()
    });
    let bystander = Watch::default();
    assert!(!watch.pulse());
    allocated(4 * MEBIBYTE);
    let stopped = watch.pulse();
    let message = stopped_in(&watch);
    released(4 * MEBIBYTE);
    assert!(stopped);
    assert!(watch.token().is_cancelled());
    assert_eq!(
        message,
        "urn:t:query.rq was stopped: the convert outgrew its memory budget of 3 MiB"
    );
    assert!(!bystander.pulse());
    assert!(!bystander.token().is_cancelled());
}

#[test]
fn a_memory_budget_is_a_whole_number_of_mebibytes_from_one_to_the_ceiling() {
    for (mebibytes, bytes) in [
        (0, None),
        (1, Some(MEBIBYTE)),
        (1024, Some(1024 * MEBIBYTE)),
        (Budgets::MAX_MEMORY_MIB, Some(3072 * MEBIBYTE)),
        (Budgets::MAX_MEMORY_MIB + 1, None),
        (4096, None),
        (u64::MAX, None),
    ] {
        assert_eq!(Budgets::memory_mib(mebibytes), bytes, "{mebibytes} MiB");
    }
}

#[test]
fn a_convert_past_its_time_budget_is_stopped_at_its_next_reading_of_the_clock() {
    let watch = Watch::new(Budgets {
        time: Duration::ZERO,
        ..Budgets::default()
    });
    assert!(watch.pulse());
    assert!(watch.token().is_cancelled());
    assert_eq!(
        stopped_in(&watch),
        "urn:t:query.rq was stopped: the convert outgrew its time budget of 0ns"
    );
}

#[test]
fn a_convert_within_its_budgets_is_not_stopped_and_names_nothing() {
    let watch = Watch::default();
    assert!(!(0..3 * PERIOD).any(|_| watch.pulse()));
    assert!(watch.outgrown(QUERY).is_none());
}

#[test]
fn a_stop_is_the_bridge_s_whoever_explains_the_error_after_it() {
    let spent = Watch::new(Budgets {
        time: Duration::ZERO,
        ..Budgets::default()
    });
    assert!(spent.pulse());
    let stopped: Result<()> = Err(spent.outgrown(QUERY).expect("stopped"));
    let unexplained: Result<()> = Err(Error::bridge("an unexplained fault"));
    assert_eq!(
        stopped
            .explained_by(ErrorKind::Document)
            .unwrap_err()
            .kind(),
        &ErrorKind::Bridge
    );
    assert_eq!(
        unexplained
            .explained_by(ErrorKind::Document)
            .unwrap_err()
            .kind(),
        &ErrorKind::Document
    );
}

#[test]
fn a_query_stops_under_either_executor_and_its_stop_does_not_reach_the_next_convert() {
    let query = Query::read(&Text, QUERY, Form::Construct, "mapping").expect("a query");
    let store = store();
    for executor in [Executor::Compiled, Executor::Spareval] {
        let spent = Profile::new(
            executor,
            false,
            Budgets {
                time: Duration::ZERO,
                ..Budgets::default()
            },
        );
        assert!(spent.watch.pulse(), "{executor:?}");
        let stopped = query
            .graph(&Dataset::new(&store), &spent)
            .expect_err("stopped");
        assert_eq!(stopped.kind(), &ErrorKind::Bridge, "{executor:?}");
        assert_eq!(
            stopped.message(),
            "urn:t:query.rq was stopped: the convert outgrew its time budget of 0ns",
            "{executor:?}"
        );
        let next = Profile::new(executor, false, Budgets::default());
        let answered = query
            .graph(&Dataset::new(&store), &next)
            .expect("the next convert is not stopped");
        assert_eq!(answered.len(), 1, "{executor:?}");
    }
}
