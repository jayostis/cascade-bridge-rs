use crate::annotation::Minted;
use crate::error::{Error, Result};
use crate::resolver::Resolver;
use oxigraph::model::{GraphName, Quad};
use oxigraph::sparql::{PreparedSparqlQuery, QueryResults, SparqlEvaluator};
use oxigraph::store::Store;
use spargebra::algebra::{AggregateExpression, Expression, GraphPattern, OrderExpression};

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
    prepared: PreparedSparqlQuery,
}

impl Query {
    pub(crate) fn read(
        resolver: &dyn Resolver,
        iri: &str,
        expected: Form,
        what: &str,
    ) -> Result<Self> {
        let text = String::from_utf8(resolver.read(iri)?)
            .map_err(|e| Error::msg(format!("{iri}: {e}")))?;
        let parsed = spargebra::SparqlParser::new()
            .with_base_iri(iri)
            .map_err(|e| Error::msg(format!("{iri}: {e}")))?
            .parse_query(&text)
            .map_err(|e| Error::msg(format!("{iri}: {e}")))?;
        let form = Form::of(&parsed);
        if form != expected {
            return Err(Error::msg(format!(
                "{what} {iri} is not a {}",
                expected.keyword()
            )));
        }
        if let Some(held) = what_fetches(&parsed) {
            return Err(Error::msg(format!(
                "{what} {iri} holds {held}; a query reads the dataset built for its unit, \n                 and nothing is fetched"
            )));
        }
        Ok(Self {
            iri: iri.to_owned(),
            prefixes: prologue_prefixes(&text),
            prepared: SparqlEvaluator::new().for_query(parsed),
        })
    }

    pub(crate) fn on(&self, store: &Store) -> Result<QueryResults<'static>> {
        // Copies the algebra already parsed; the text is not parsed again.
        Ok(self.prepared.clone().on_store(store).execute()?)
    }

    pub(crate) fn graph(&self, store: &Store) -> Result<Vec<Quad>> {
        let QueryResults::Graph(triples) = self.on(store)? else {
            return Err(Error::msg(format!(
                "{} is not a {}",
                self.iri,
                Form::Construct.keyword()
            )));
        };
        let mut quads = Vec::new();
        for triple in triples {
            let triple = triple?;
            quads.push(Quad::new(
                triple.subject,
                triple.predicate,
                triple.object,
                GraphName::DefaultGraph,
            ));
        }
        Ok(Minted::default().apart(quads))
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
