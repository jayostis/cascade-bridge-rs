use crate::annotation::Minted;
use crate::budget::{Budgets, Watch};
use crate::compiled::{self, Compiled, Dataset, Executor, Fail};
use crate::error::{Error, ErrorKind, Result};
use crate::measure::{Stage, StageTime, Stopwatch};
use crate::resolver::Resolver;
use oxigraph::model::{GraphName, Quad};
use oxigraph::sparql::{QueryEvaluationError, QueryResults, SparqlEvaluator};
use oxigraph::store::Store;
use spargebra::algebra::{AggregateExpression, Expression, GraphPattern, OrderExpression};
use std::cell::RefCell;
use std::time::Duration;
// std's clock panics on wasm32-unknown-unknown, where this one asks the host.
use web_time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryProfile {
    pub iri: String,
    pub executions: usize,
    pub rows: usize,
    pub elapsed: Duration,
    /// Why the compiler did not take the query; none where it did, and where it did not run.
    pub fallback: Option<String>,
    /// Executions a compiled plan gave up on, which the general evaluator then answered.
    pub give_ups: usize,
    pub stage: Option<Stage>,
}

/// A query of an adapter that the compiler did not take, so stock spareval evaluates it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fallback {
    pub query: String,
    pub reason: String,
}

/// A query of an adapter and the plan the compiler makes of it, printed, or why it makes none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Planned {
    pub query: String,
    pub plan: std::result::Result<String, String>,
}

/// How a run executes its queries, what it may spend, and what it keeps of them, in
/// the order each query first ran.
#[derive(Default)]
pub(crate) struct Profile {
    executor: Executor,
    kept: bool,
    pub(crate) watch: Watch,
    queries: RefCell<Vec<QueryProfile>>,
    stopwatch: Stopwatch,
}

impl Profile {
    pub(crate) fn new(executor: Executor, kept: bool, budgets: Budgets) -> Self {
        Self {
            executor,
            kept,
            watch: Watch::new(budgets),
            queries: RefCell::default(),
            stopwatch: Stopwatch::new(kept),
        }
    }

    pub(crate) fn during<T>(&self, stage: Stage, work: impl FnOnce() -> T) -> T {
        self.stopwatch.during(stage, work)
    }

    fn indexing(&self) -> Duration {
        self.stopwatch.spent_in(Stage::Index)
    }

    pub(crate) fn stages(&self) -> Vec<StageTime> {
        self.stopwatch.times()
    }

    fn add(&self, query: &Query, rows: usize, elapsed: Duration, gave_up: bool) {
        let mut profiles = self.queries.borrow_mut();
        match profiles.iter_mut().find(|profile| profile.iri == query.iri) {
            Some(profile) => {
                profile.executions += 1;
                profile.rows += rows;
                profile.elapsed += elapsed;
                profile.give_ups += usize::from(gave_up);
            }
            None => profiles.push(QueryProfile {
                iri: query.iri.clone(),
                executions: 1,
                rows,
                elapsed,
                fallback: match self.executor {
                    Executor::Spareval => None,
                    Executor::Compiled | Executor::Both => query.compiled.as_ref().err().cloned(),
                },
                give_ups: usize::from(gave_up),
                stage: self.stopwatch.current(),
            }),
        }
    }

    pub(crate) fn queries(self) -> Vec<QueryProfile> {
        self.queries.into_inner()
    }
}

/// Reads no clock unless a profile is kept. Building the index is the `Index` stage's
/// time, not the query's.
struct Timing<'a> {
    kept: Option<(&'a Profile, Instant, Duration)>,
}

impl<'a> Timing<'a> {
    fn start(profile: &'a Profile) -> Self {
        Self {
            kept: profile
                .kept
                .then(|| (profile, Instant::now(), profile.indexing())),
        }
    }

    fn stop(self, query: &Query, rows: usize, gave_up: bool) {
        if let Some((profile, started, indexed)) = self.kept {
            let elapsed = started
                .elapsed()
                .saturating_sub(profile.indexing().saturating_sub(indexed));
            profile.add(query, rows, elapsed, gave_up);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Form {
    Select,
    Construct,
    Ask,
    Describe,
}

impl Form {
    fn keyword(self) -> &'static str {
        match self {
            Self::Select => "SELECT",
            Self::Construct => "CONSTRUCT",
            Self::Ask => "ASK",
            Self::Describe => "DESCRIBE",
        }
    }

    fn of(query: &spargebra::Query) -> Self {
        match query {
            spargebra::Query::Select { .. } => Self::Select,
            spargebra::Query::Construct { .. } => Self::Construct,
            spargebra::Query::Ask { .. } => Self::Ask,
            spargebra::Query::Describe { .. } => Self::Describe,
        }
    }
}

pub(crate) struct Query {
    pub(crate) iri: String,
    pub(crate) prefixes: Vec<(String, String)>,
    parsed: spargebra::Query,
    compiled: std::result::Result<Compiled, String>,
}

impl Query {
    pub(crate) fn read(
        resolver: &dyn Resolver,
        iri: &str,
        expected: Form,
        what: &str,
    ) -> Result<Self> {
        let text = String::from_utf8(resolver.read(iri)?)
            .map_err(|e| Error::adapter(format!("{iri}: {e}")))?;
        let parsed = spargebra::SparqlParser::new()
            .with_base_iri(iri)
            .map_err(|e| Error::adapter(format!("{iri}: {e}")))?
            .parse_query(&text)
            .map_err(|e| Error::adapter(format!("{iri}: {e}")))?;
        let form = Form::of(&parsed);
        if form != expected {
            return Err(Error::adapter(format!(
                "{what} {iri} is not a {}",
                expected.keyword()
            )));
        }
        if let Some(held) = what_fetches(&parsed) {
            return Err(Error::adapter(format!(
                "{what} {iri} holds {held}; a query reads the dataset built for its unit, \
                 and nothing is fetched"
            )));
        }
        Ok(Self {
            iri: iri.to_owned(),
            prefixes: prologue_prefixes(&text),
            compiled: compiled::compile(&parsed),
            parsed,
        })
    }

    pub(crate) fn fallback(&self) -> Option<Fallback> {
        Some(Fallback {
            query: self.iri.clone(),
            reason: self.compiled.as_ref().err()?.clone(),
        })
    }

    pub(crate) fn explained(&self) -> Planned {
        Planned {
            query: self.iri.clone(),
            plan: compiled::compile_explained(&self.parsed).map(|plan| plan.to_string()),
        }
    }

    fn failed(&self, error: QueryEvaluationError, watch: &Watch) -> Error {
        watch
            .outgrown(&self.iri)
            .unwrap_or_else(|| Error::from(error).explained_by(ErrorKind::Adapter))
    }

    // Copies the algebra already parsed; the text is not parsed again.
    fn on(&self, store: &Store, watch: &Watch) -> Result<QueryResults<'static>> {
        SparqlEvaluator::new()
            .with_cancellation_token(watch.token())
            .for_query(self.parsed.clone())
            .on_store(store)
            .execute()
            .map_err(|e| self.failed(e, watch))
    }

    fn evaluated(&self, store: &Store, watch: &Watch) -> Result<Vec<Quad>> {
        let QueryResults::Graph(triples) = self.on(store, watch)? else {
            return Err(Error::adapter(format!(
                "{} is not a {}",
                self.iri,
                Form::Construct.keyword()
            )));
        };
        let mut quads = Vec::new();
        for triple in triples {
            let triple = triple.map_err(|e| self.failed(e, watch))?;
            quads.push(Quad::new(
                triple.subject,
                triple.predicate,
                triple.object,
                GraphName::DefaultGraph,
            ));
        }
        Ok(quads)
    }

    fn constructed(&self, data: &Dataset<'_>, profile: &Profile) -> Result<(Vec<Quad>, bool)> {
        let (executor, watch) = (profile.executor, &profile.watch);
        let plan = match (&self.compiled, executor) {
            (Ok(plan), Executor::Compiled | Executor::Both) => plan,
            _ => return Ok((self.evaluated(data.store()?, watch)?, false)),
        };
        let tree = profile.during(Stage::Index, || data.tree(watch))?;
        match (plan.construct(tree), executor) {
            (Ok(quads), Executor::Both) => {
                let evaluated = self.evaluated(data.store()?, watch)?;
                compiled::agree(&self.iri, &quads, &evaluated)?;
                Ok((evaluated, false))
            }
            (Ok(quads), _) => Ok((quads, false)),
            (Err(Fail::Stopped), _) => Err(watch
                .outgrown(&self.iri)
                .unwrap_or_else(|| Error::outgrown(format!("{} was stopped", self.iri)))),
            (Err(_), _) => Ok((self.evaluated(data.store()?, watch)?, true)),
        }
    }

    pub(crate) fn graph(&self, data: &Dataset<'_>, profile: &Profile) -> Result<Vec<Quad>> {
        let timing = Timing::start(profile);
        let (quads, gave_up) = self.constructed(data, profile)?;
        timing.stop(self, quads.len(), gave_up);
        Ok(Minted::default().apart(quads))
    }

    pub(crate) fn ask(&self, store: &Store, what: &str, profile: &Profile) -> Result<bool> {
        let timing = Timing::start(profile);
        let QueryResults::Boolean(answer) = self.on(store, &profile.watch)? else {
            return Err(Error::adapter(format!(
                "{what} {} is not an {}",
                self.iri,
                Form::Ask.keyword()
            )));
        };
        timing.stop(self, usize::from(answer), false);
        Ok(answer)
    }
}

/// Skips whitespace and comments.
fn between(text: &str) -> &str {
    let mut rest = text.trim_start();
    while let Some(comment) = rest.strip_prefix('#') {
        rest = comment
            .find('\n')
            .map_or("", |end| &comment[end..])
            .trim_start();
    }
    rest
}

fn keyword<'a>(text: &'a str, word: &str) -> Option<&'a str> {
    let rest = text.get(word.len()..)?;
    (text[..word.len()].eq_ignore_ascii_case(word)
        && rest.starts_with(|c: char| c.is_whitespace() || c == '#' || c == '<'))
    .then_some(rest)
}

fn declared_name(text: &str) -> Option<(&str, &str)> {
    let (name, rest) = text.split_once(':')?;
    (!name.contains(char::is_whitespace)).then_some((name, rest))
}

fn iri_ref(text: &str) -> Option<(&str, &str)> {
    let rest = text.strip_prefix('<')?;
    let end = rest.find('>')?;
    Some((&rest[..end], &rest[end + 1..]))
}

/// Read from the text: SPARQL keeps PREFIX declarations out of the algebra.
fn prologue_prefixes(text: &str) -> Vec<(String, String)> {
    let mut prefixes = Vec::new();
    let mut rest = between(text);
    loop {
        if let Some(after) = keyword(rest, "BASE") {
            let Some((_, after)) = iri_ref(between(after)) else {
                break;
            };
            rest = between(after);
        } else if let Some(after) = keyword(rest, "PREFIX") {
            let Some((name, after)) = declared_name(between(after)) else {
                break;
            };
            let Some((namespace, after)) = iri_ref(between(after)) else {
                break;
            };
            prefixes.push((name.to_owned(), namespace.to_owned()));
            rest = between(after);
        } else {
            break;
        }
    }
    prefixes
}

fn holds_a_service(pattern: &GraphPattern) -> bool {
    match pattern {
        GraphPattern::Service { .. } => true,
        GraphPattern::Bgp { .. } | GraphPattern::Path { .. } | GraphPattern::Values { .. } => false,
        GraphPattern::Join { left, right }
        | GraphPattern::Lateral { left, right }
        | GraphPattern::Union { left, right }
        | GraphPattern::Minus { left, right } => holds_a_service(left) || holds_a_service(right),
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => {
            holds_a_service(left)
                || holds_a_service(right)
                || expression.as_ref().is_some_and(asks_a_service)
        }
        GraphPattern::Filter { expr, inner } => asks_a_service(expr) || holds_a_service(inner),
        GraphPattern::Extend {
            inner, expression, ..
        } => asks_a_service(expression) || holds_a_service(inner),
        GraphPattern::OrderBy { inner, expression } => {
            holds_a_service(inner)
                || expression.iter().any(|order| match order {
                    OrderExpression::Asc(e) | OrderExpression::Desc(e) => asks_a_service(e),
                })
        }
        GraphPattern::Group {
            inner, aggregates, ..
        } => {
            holds_a_service(inner)
                || aggregates.iter().any(|(_, aggregate)| match aggregate {
                    AggregateExpression::CountSolutions { .. } => false,
                    AggregateExpression::FunctionCall { expr, .. } => asks_a_service(expr),
                })
        }
        GraphPattern::Graph { inner, .. }
        | GraphPattern::Project { inner, .. }
        | GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. } => holds_a_service(inner),
    }
}

fn asks_a_service(expression: &Expression) -> bool {
    match expression {
        Expression::Exists(pattern) => holds_a_service(pattern),
        Expression::NamedNode(_)
        | Expression::Literal(_)
        | Expression::Variable(_)
        | Expression::Bound(_) => false,
        Expression::Or(one, two)
        | Expression::And(one, two)
        | Expression::Equal(one, two)
        | Expression::SameTerm(one, two)
        | Expression::Greater(one, two)
        | Expression::GreaterOrEqual(one, two)
        | Expression::Less(one, two)
        | Expression::LessOrEqual(one, two)
        | Expression::Add(one, two)
        | Expression::Subtract(one, two)
        | Expression::Multiply(one, two)
        | Expression::Divide(one, two) => asks_a_service(one) || asks_a_service(two),
        Expression::UnaryPlus(one) | Expression::UnaryMinus(one) | Expression::Not(one) => {
            asks_a_service(one)
        }
        Expression::In(one, many) => asks_a_service(one) || many.iter().any(asks_a_service),
        Expression::If(one, two, three) => {
            asks_a_service(one) || asks_a_service(two) || asks_a_service(three)
        }
        Expression::Coalesce(many) | Expression::FunctionCall(_, many) => {
            many.iter().any(asks_a_service)
        }
    }
}

fn what_fetches(query: &spargebra::Query) -> Option<&'static str> {
    let (dataset, pattern) = match query {
        spargebra::Query::Select {
            dataset, pattern, ..
        }
        | spargebra::Query::Construct {
            dataset, pattern, ..
        }
        | spargebra::Query::Describe {
            dataset, pattern, ..
        }
        | spargebra::Query::Ask {
            dataset, pattern, ..
        } => (dataset, pattern),
    };
    if holds_a_service(pattern) {
        return Some("a SERVICE pattern");
    }
    let dataset = dataset.as_ref()?;
    if dataset
        .named
        .as_ref()
        .is_some_and(|named| !named.is_empty())
    {
        return Some("a FROM NAMED clause");
    }
    (!dataset.default.is_empty()).then_some("a FROM clause")
}

#[cfg(test)]
mod tests {
    use super::prologue_prefixes;

    #[test]
    fn reads_a_prologue_however_its_keyword_and_spacing_are_written() {
        assert_eq!(
            prologue_prefixes(
                "prefix ex: <urn:example:catalog#>\n  PREFIX  g:<https://ns.example.org/g/v1#>\nCONSTRUCT { }"
            ),
            [
                ("ex".to_owned(), "urn:example:catalog#".to_owned()),
                ("g".to_owned(), "https://ns.example.org/g/v1#".to_owned()),
            ]
        );
    }

    #[test]
    fn reads_both_declarations_a_prologue_writes_on_one_line() {
        assert_eq!(
            prologue_prefixes(
                "PREFIX ex: <urn:example:catalog#> PREFIX v1: <https://ns.example.org/v1#>\nCONSTRUCT { }"
            ),
            [
                ("ex".to_owned(), "urn:example:catalog#".to_owned()),
                ("v1".to_owned(), "https://ns.example.org/v1#".to_owned()),
            ]
        );
    }

    #[test]
    fn reads_a_prefix_a_base_and_a_comment_stand_before() {
        assert_eq!(
            prologue_prefixes(
                "BASE <urn:example:> # where the names begin\nPREFIX ex: <urn:example:catalog#>\nCONSTRUCT { }"
            ),
            [("ex".to_owned(), "urn:example:catalog#".to_owned())]
        );
    }

    #[test]
    fn reads_no_prefix_out_of_a_comment_or_a_word_that_merely_starts_with_one() {
        assert_eq!(
            prologue_prefixes("# PREFIX ex: <urn:example:catalog#>\nPREFIXES ex: <urn:x#>"),
            Vec::new()
        );
    }
}
