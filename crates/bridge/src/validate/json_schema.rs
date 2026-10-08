// Draft-06 only, every validation keyword but `format`, which is not asserted. A
// keyword that applies subschemas is no finding of its own: the keyword that fails
// inside is, except that `anyOf`, `oneOf`, `not` and `contains` fail as themselves,
// and one whose subschema is `false` fails as itself where it stands.
use super::{Schema, SchemaFinding};
use crate::error::{Error, Result};
use crate::json::{self, Node, Value};
use crate::resolver::Resolver;
use crate::terms::BRIDGE_SCHEMA_RULE_UNNAMED;
use oxiri::Iri;
use regress::Regex;
use std::cell::RefCell;
use std::collections::HashMap;

const DRAFT_06: &str = "http://json-schema.org/draft-06/schema";

const VALIDATION: &str =
    "https://datatracker.ietf.org/doc/html/draft-wright-json-schema-validation-01";

/// In the order of their sections, 6.1 onwards.
const KEYWORDS: [&str; 29] = [
    "multipleOf",
    "maximum",
    "exclusiveMaximum",
    "minimum",
    "exclusiveMinimum",
    "maxLength",
    "minLength",
    "pattern",
    "items",
    "additionalItems",
    "maxItems",
    "minItems",
    "uniqueItems",
    "contains",
    "maxProperties",
    "minProperties",
    "required",
    "properties",
    "patternProperties",
    "additionalProperties",
    "dependencies",
    "propertyNames",
    "enum",
    "const",
    "type",
    "allOf",
    "anyOf",
    "oneOf",
    "not",
];

/// A `$ref` chain longer than this never reaches the instance's next node.
const DEPTH: usize = 1024;

pub(crate) struct JsonSchema {
    /// By the IRI each was read at and by each `$id` in it, with the base that
    /// `$id` resolves against.
    documents: HashMap<String, (String, Node)>,
    root: String,
    patterns: HashMap<String, Regex>,
    /// By the base a `$ref` was resolved against, then by the `$ref`.
    located: RefCell<HashMap<String, HashMap<String, Located>>>,
}

/// Where a `$ref` led: the node at `positions` in the document read as `document`,
/// and the base its own `$id` resolves against.
struct Located {
    outer: String,
    document: String,
    positions: Vec<usize>,
}

fn body(keyword: Option<&str>) -> String {
    match keyword.and_then(|keyword| KEYWORDS.iter().position(|named| *named == keyword)) {
        Some(index) => format!("{VALIDATION}#section-6.{}", index + 1),
        None => BRIDGE_SCHEMA_RULE_UNNAMED.to_owned(),
    }
}

fn member<'a>(node: &'a Node, name: &str) -> Option<&'a Node> {
    match &node.value {
        Value::Object(members) => members
            .iter()
            .find(|(named, _)| named == name)
            .map(|(_, member)| member),
        _ => None,
    }
}

fn text(node: &Node) -> Option<&str> {
    match &node.value {
        Value::String(text) => Some(text),
        _ => None,
    }
}

fn without_fragment(iri: &str) -> &str {
    iri.split('#').next().unwrap_or(iri)
}

fn resolved(base: &str, reference: &str) -> Result<String> {
    let base = Iri::parse(base).map_err(|e| Error::adapter(format!("{base}: {e}")))?;
    Ok(base
        .resolve(reference)
        .map_err(|e| Error::adapter(format!("{base} names {reference}: {e}")))?
        .into_inner())
}

/// Keywords whose value is an instance, never a schema.
const VALUES: [&str; 4] = ["enum", "const", "default", "examples"];

/// Keywords whose value names its schemas, so a member's name there is no keyword.
const NAMING: [&str; 4] = [
    "properties",
    "patternProperties",
    "dependencies",
    "definitions",
];

/// Every regular expression the schema writes, in a `pattern` or as a
/// `patternProperties` name, anywhere but inside an instance.
fn patterns_written<'a>(schema: &'a Node, into: &mut Vec<&'a str>) {
    for (keyword, value) in schema.children() {
        match &*keyword {
            "pattern" => into.extend(text(value)),
            keyword if VALUES.contains(&keyword) => continue,
            _ => {}
        }
        if NAMING.contains(&&*keyword) {
            if keyword == "patternProperties" {
                if let Value::Object(members) = &value.value {
                    into.extend(members.iter().map(|(pattern, _)| pattern.as_str()));
                }
            }
            for (_, named) in value.children() {
                patterns_written(named, into);
            }
        } else {
            patterns_written(value, into);
        }
    }
}

/// A subschema's `$id`, unless a `$ref` beside it has every other member ignored.
fn own_id(node: &Node) -> Option<&str> {
    if member(node, "$ref").is_some() {
        return None;
    }
    member(node, "$id").and_then(text)
}

/// Every object in the document, with the base its own `$id` and `$ref` resolve against.
fn objects<'a>(node: &'a Node, outer: String, into: &mut Vec<(String, &'a Node)>) -> Result<()> {
    let inner = match own_id(node) {
        Some(id) => without_fragment(&resolved(&outer, id)?).to_owned(),
        None => outer.clone(),
    };
    if matches!(node.value, Value::Object(_)) {
        into.push((outer, node));
    }
    for (_, child) in node.children() {
        objects(child, inner.clone(), into)?;
    }
    Ok(())
}

pub(crate) fn compile(iri: &str, resolver: &dyn Resolver) -> Result<JsonSchema> {
    let mut documents: HashMap<String, (String, Node)> = HashMap::new();
    let mut patterns = HashMap::new();
    let mut pending = vec![without_fragment(iri).to_owned()];
    while let Some(location) = pending.pop() {
        if documents.contains_key(&location) {
            continue;
        }
        let bytes = resolver.read(&location).map_err(|e| {
            e.reworded(|said| format!("{iri} names a schema this Bridge did not read: {said}"))
        })?;
        let node = json::decode(&bytes)
            .and_then(json::parse)
            .map_err(|e| Error::adapter(format!("{location}: {e}")))?;
        if let Some(draft) = member(&node, "$schema").and_then(text) {
            if without_fragment(draft) != DRAFT_06 {
                return Err(Error::adapter(format!(
                    "{location} is written in {draft}; this Bridge validates against draft-06 \
                     of JSON Schema alone"
                )));
            }
        }
        let mut scoped = Vec::new();
        objects(&node, location.clone(), &mut scoped)?;
        documents.insert(location.clone(), (location.clone(), node.clone()));
        for (outer, object) in scoped {
            if let Some(id) = own_id(object) {
                let identified = resolved(&outer, id)?;
                documents
                    .entry(without_fragment(&identified).to_owned())
                    .or_insert_with(|| (outer.clone(), object.clone()));
                if let Some((_, name)) = identified.split_once('#') {
                    if !name.is_empty() && !name.starts_with('/') {
                        documents.insert(identified.clone(), (outer.clone(), object.clone()));
                    }
                }
            }
            if let Some(reference) = member(object, "$ref").and_then(text) {
                pending.push(without_fragment(&resolved(&outer, reference)?).to_owned());
            }
        }
        let mut written = Vec::new();
        patterns_written(&node, &mut written);
        for pattern in written {
            if !patterns.contains_key(pattern) {
                let compiled = Regex::with_flags(pattern, "u").map_err(|e| {
                    Error::adapter(format!(
                        "{location}: the pattern {pattern} is not read: {e}"
                    ))
                })?;
                patterns.insert(pattern.to_owned(), compiled);
            }
        }
    }
    let schema = JsonSchema {
        documents,
        root: iri.to_owned(),
        patterns,
        located: RefCell::default(),
    };
    schema.root()?;
    Ok(schema)
}

struct Failure {
    keyword: Option<&'static str>,
    at: Vec<String>,
}

/// Whether one failure is enough: a subschema tried by `anyOf`, `oneOf`, `not` or
/// `contains` only says whether it holds.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Asked {
    Every,
    Whether,
}

struct Run<'s> {
    schema: &'s JsonSchema,
    asked: Asked,
    failures: Vec<Failure>,
}

fn named_keyword(name: &str) -> &'static str {
    KEYWORDS
        .iter()
        .find(|keyword| **keyword == name)
        .expect("a draft-06 keyword")
}

/// A number's value as its sign, its significant digits and the power of ten they
/// are scaled by, so that `1.0` and `1` are one value.
fn decimal(written: &str) -> (bool, String, i64) {
    let (mantissa, exponent) = match written.find(['e', 'E']) {
        Some(at) => (
            &written[..at],
            written[at + 1..].parse::<i64>().unwrap_or(0),
        ),
        None => (written, 0),
    };
    let negative = mantissa.starts_with('-');
    let mantissa = mantissa.trim_start_matches('-');
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let digits = format!("{whole}{fraction}");
    let mut exponent = exponent - i64::try_from(fraction.len()).unwrap_or(i64::MAX);
    let digits = digits.trim_start_matches('0');
    let significant = digits.trim_end_matches('0');
    exponent += i64::try_from(digits.len() - significant.len()).unwrap_or(0);
    if significant.is_empty() {
        return (false, String::new(), 0);
    }
    (negative, significant.to_owned(), exponent)
}

fn number(node: &Node) -> Option<f64> {
    match &node.value {
        Value::Number(written) => written.parse().ok(),
        _ => None,
    }
}

fn is_integer(written: &str) -> bool {
    decimal(written).2 >= 0
}

fn equal(one: &Node, other: &Node) -> bool {
    match (&one.value, &other.value) {
        (Value::Number(a), Value::Number(b)) => decimal(a) == decimal(b),
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(x, y)| equal(x, y))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter().all(|(name, value)| {
                    b.iter()
                        .any(|(other, theirs)| other == name && equal(value, theirs))
                })
        }
        (a, b) => a == b,
    }
}

fn is_type(instance: &Node, named: &str) -> bool {
    match (named, &instance.value) {
        ("null", Value::Null)
        | ("boolean", Value::Bool(_))
        | ("object", Value::Object(_))
        | ("array", Value::Array(_))
        | ("string", Value::String(_))
        | ("number", Value::Number(_)) => true,
        ("integer", Value::Number(written)) => is_integer(written),
        _ => false,
    }
}

impl<'s> Run<'s> {
    fn done(&self) -> bool {
        self.asked == Asked::Whether && !self.failures.is_empty()
    }

    fn fail(&mut self, keyword: Option<&'static str>, at: &[String]) {
        self.failures.push(Failure {
            keyword,
            at: at.to_vec(),
        });
    }

    /// Whether `instance` holds against `schema`, recording nothing.
    fn holds(
        &self,
        schema: &'s Node,
        base: &'s str,
        instance: &Node,
        depth: usize,
    ) -> Result<bool> {
        let mut trial = Run {
            schema: self.schema,
            asked: Asked::Whether,
            failures: Vec::new(),
        };
        trial.validate(schema, base, instance, &mut Vec::new(), depth + 1)?;
        Ok(trial.failures.is_empty())
    }

    /// A subschema applied by `keyword`: `false` fails as the keyword, at `at`.
    #[allow(clippy::too_many_arguments)]
    fn apply(
        &mut self,
        keyword: &'static str,
        schema: &'s Node,
        base: &'s str,
        instance: &Node,
        at: &mut Vec<String>,
        depth: usize,
        from: &[String],
    ) -> Result<()> {
        match schema.value {
            Value::Bool(false) => {
                self.fail(Some(keyword), from);
                Ok(())
            }
            _ => self.validate(schema, base, instance, at, depth + 1),
        }
    }

    /// The base `schema`'s own `$id` sets for what it holds.
    fn scope(&self, outer: &'s str, schema: &Node) -> Result<&'s str> {
        let Some(id) = own_id(schema) else {
            return Ok(outer);
        };
        let inner = resolved(outer, id)?;
        let inner = without_fragment(&inner);
        self.schema
            .documents
            .get_key_value(inner)
            .map(|(key, _)| key.as_str())
            .ok_or_else(|| Error::adapter(format!("the $id {inner}, a schema not read")))
    }

    /// The subschema a `$ref` names, and the base its own `$id` resolves against.
    fn reference(&self, base: &str, reference: &str) -> Result<(&'s str, &'s Node)> {
        let schema: &'s JsonSchema = self.schema;
        if let Some(found) = schema.remembered(base, reference) {
            return Ok(found);
        }
        let (located, found) = self.locate(base, reference)?;
        schema
            .located
            .borrow_mut()
            .entry(base.to_owned())
            .or_default()
            .insert(reference.to_owned(), located);
        Ok(found)
    }

    fn locate(&self, base: &str, reference: &str) -> Result<(Located, (&'s str, &'s Node))> {
        let target = resolved(base, reference)?;
        if let Some((outer, named)) = self.schema.documents.get(&target) {
            let located = Located {
                outer: outer.clone(),
                document: target,
                positions: Vec::new(),
            };
            return Ok((located, (outer.as_str(), named)));
        }
        let (document, fragment) = target.split_once('#').unwrap_or((&target, ""));
        let (outer, node) = self
            .schema
            .documents
            .get(document)
            .ok_or_else(|| Error::adapter(format!("a $ref to {target}, a schema not read")))?;
        let found = json::selected(node, fragment);
        match found.as_slice() {
            [(positions, reached)] => {
                let mut outer = outer.as_str();
                let mut passed = node;
                for &position in positions {
                    outer = self.scope(outer, passed)?;
                    passed = passed.at(&[position]).ok_or_else(|| {
                        Error::adapter(format!("a $ref to {target}, not followed"))
                    })?;
                }
                let located = Located {
                    outer: outer.to_owned(),
                    document: document.to_owned(),
                    positions: positions.clone(),
                };
                Ok((located, (outer, *reached)))
            }
            _ => Err(Error::adapter(format!(
                "a $ref to {target}, which names no one subschema"
            ))),
        }
    }

    fn validate(
        &mut self,
        schema: &'s Node,
        base: &'s str,
        instance: &Node,
        at: &mut Vec<String>,
        depth: usize,
    ) -> Result<()> {
        if depth > DEPTH {
            return Err(Error::adapter("a $ref chain that never reaches a keyword"));
        }
        let members = match &schema.value {
            Value::Object(members) => members,
            Value::Bool(true) => return Ok(()),
            Value::Bool(false) => {
                self.fail(None, at);
                return Ok(());
            }
            _ => {
                return Err(Error::adapter(
                    "a schema that is neither an object nor a boolean",
                ))
            }
        };
        if let Some(reference) = member(schema, "$ref").and_then(text) {
            let (outer, target) = self.reference(base, reference)?;
            return self.validate(target, outer, instance, at, depth + 1);
        }
        let base = self.scope(base, schema)?;
        for (name, value) in members {
            if self.done() {
                return Ok(());
            }
            self.keyword(schema, name, value, base, instance, at, depth)?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn keyword(
        &mut self,
        schema: &'s Node,
        name: &str,
        value: &'s Node,
        base: &'s str,
        instance: &Node,
        at: &mut Vec<String>,
        depth: usize,
    ) -> Result<()> {
        let bound = number(value);
        let held = match (name, &instance.value) {
            ("type", _) => match &value.value {
                Value::String(named) => is_type(instance, named),
                Value::Array(names) => names
                    .iter()
                    .filter_map(text)
                    .any(|named| is_type(instance, named)),
                _ => true,
            },
            ("enum", _) => match &value.value {
                Value::Array(allowed) => allowed.iter().any(|allowed| equal(allowed, instance)),
                _ => true,
            },
            ("const", _) => equal(value, instance),
            ("multipleOf", Value::Number(_)) => match (number(instance), bound) {
                (Some(x), Some(m)) if m > 0.0 => {
                    let quotient = x / m;
                    (quotient - quotient.round()).abs() <= 1e-9 * quotient.abs().max(1.0)
                }
                _ => true,
            },
            ("maximum", Value::Number(_)) => compare(instance, bound, |x, b| x <= b),
            ("exclusiveMaximum", Value::Number(_)) => compare(instance, bound, |x, b| x < b),
            ("minimum", Value::Number(_)) => compare(instance, bound, |x, b| x >= b),
            ("exclusiveMinimum", Value::Number(_)) => compare(instance, bound, |x, b| x > b),
            ("maxLength", Value::String(s)) => within(s.chars().count(), bound, |n, b| n <= b),
            ("minLength", Value::String(s)) => within(s.chars().count(), bound, |n, b| n >= b),
            ("pattern", Value::String(s)) => text(value)
                .and_then(|pattern| self.schema.patterns.get(pattern))
                .is_none_or(|pattern| pattern.find(s).is_some()),
            ("maxItems", Value::Array(items)) => within(items.len(), bound, |n, b| n <= b),
            ("minItems", Value::Array(items)) => within(items.len(), bound, |n, b| n >= b),
            ("uniqueItems", Value::Array(items)) => {
                value.value != Value::Bool(true)
                    || items
                        .iter()
                        .enumerate()
                        .all(|(i, item)| items[..i].iter().all(|before| !equal(before, item)))
            }
            ("maxProperties", Value::Object(members)) => {
                within(members.len(), bound, |n, b| n <= b)
            }
            ("minProperties", Value::Object(members)) => {
                within(members.len(), bound, |n, b| n >= b)
            }
            ("required", Value::Object(members)) => match &value.value {
                Value::Array(names) => names
                    .iter()
                    .filter_map(text)
                    .all(|required| members.iter().any(|(named, _)| named == required)),
                _ => true,
            },
            ("items", Value::Array(items)) => {
                let from = at.clone();
                match &value.value {
                    Value::Array(schemas) => {
                        for (index, (item, schema)) in items.iter().zip(schemas).enumerate() {
                            at.push(index.to_string());
                            self.apply("items", schema, base, item, at, depth, &from)?;
                            at.pop();
                        }
                    }
                    _ => {
                        for (index, item) in items.iter().enumerate() {
                            at.push(index.to_string());
                            self.apply("items", value, base, item, at, depth, &from)?;
                            at.pop();
                        }
                    }
                }
                true
            }
            ("additionalItems", Value::Array(items)) => {
                if let Some(Value::Array(schemas)) = member(schema, "items").map(|n| &n.value) {
                    let from = at.clone();
                    for (index, item) in items.iter().enumerate().skip(schemas.len()) {
                        at.push(index.to_string());
                        self.apply("additionalItems", value, base, item, at, depth, &from)?;
                        at.pop();
                    }
                }
                true
            }
            ("contains", Value::Array(items)) => {
                let mut any = false;
                for item in items {
                    if self.holds(value, base, item, depth)? {
                        any = true;
                        break;
                    }
                }
                any
            }
            ("properties", Value::Object(members)) => {
                let from = at.clone();
                for (named, child) in members {
                    if let Some(schema) = member(value, named) {
                        at.push(named.clone());
                        self.apply("properties", schema, base, child, at, depth, &from)?;
                        at.pop();
                    }
                }
                true
            }
            ("patternProperties", Value::Object(members)) => {
                let from = at.clone();
                if let Value::Object(patterns) = &value.value {
                    for (named, child) in members {
                        for (pattern, schema) in patterns {
                            if self
                                .schema
                                .patterns
                                .get(pattern)
                                .is_some_and(|p| p.find(named).is_some())
                            {
                                at.push(named.clone());
                                self.apply(
                                    "patternProperties",
                                    schema,
                                    base,
                                    child,
                                    at,
                                    depth,
                                    &from,
                                )?;
                                at.pop();
                            }
                        }
                    }
                }
                true
            }
            ("additionalProperties", Value::Object(members)) => {
                let from = at.clone();
                for (named, child) in members {
                    let declared = member(schema, "properties")
                        .is_some_and(|properties| member(properties, named).is_some());
                    let patterned = match member(schema, "patternProperties").map(|n| &n.value) {
                        Some(Value::Object(patterns)) => patterns.iter().any(|(pattern, _)| {
                            self.schema
                                .patterns
                                .get(pattern)
                                .is_some_and(|p| p.find(named).is_some())
                        }),
                        _ => false,
                    };
                    if !declared && !patterned {
                        at.push(named.clone());
                        self.apply("additionalProperties", value, base, child, at, depth, &from)?;
                        at.pop();
                    }
                }
                true
            }
            ("dependencies", Value::Object(members)) => {
                let mut held = true;
                if let Value::Object(dependencies) = &value.value {
                    for (named, dependency) in dependencies {
                        if !members.iter().any(|(present, _)| present == named) {
                            continue;
                        }
                        match &dependency.value {
                            Value::Array(names) => {
                                held &= names.iter().filter_map(text).all(|required| {
                                    members.iter().any(|(present, _)| present == required)
                                });
                            }
                            _ => {
                                let from = at.clone();
                                self.apply(
                                    "dependencies",
                                    dependency,
                                    base,
                                    instance,
                                    at,
                                    depth,
                                    &from,
                                )?;
                            }
                        }
                    }
                }
                held
            }
            ("propertyNames", Value::Object(members)) => {
                let mut held = true;
                for (named, _) in members {
                    let name = Node {
                        value: Value::String(named.clone()),
                        span: 0..0,
                    };
                    held &= self.holds(value, base, &name, depth)?;
                }
                held
            }
            ("allOf", _) => {
                if let Value::Array(schemas) = &value.value {
                    for schema in schemas {
                        let from = at.clone();
                        self.apply("allOf", schema, base, instance, at, depth, &from)?;
                    }
                }
                true
            }
            ("anyOf", _) => match &value.value {
                Value::Array(schemas) => {
                    let mut any = false;
                    for schema in schemas {
                        if self.holds(schema, base, instance, depth)? {
                            any = true;
                            break;
                        }
                    }
                    any
                }
                _ => true,
            },
            ("oneOf", _) => match &value.value {
                Value::Array(schemas) => {
                    let mut holding = 0;
                    for schema in schemas {
                        if self.holds(schema, base, instance, depth)? {
                            holding += 1;
                        }
                    }
                    holding == 1
                }
                _ => true,
            },
            ("not", _) => !self.holds(value, base, instance, depth)?,
            _ => true,
        };
        if !held {
            self.fail(Some(named_keyword(name)), at);
        }
        Ok(())
    }
}

fn compare(instance: &Node, bound: Option<f64>, holds: impl Fn(f64, f64) -> bool) -> bool {
    match (number(instance), bound) {
        (Some(x), Some(b)) => holds(x, b),
        _ => true,
    }
}

fn within(count: usize, bound: Option<f64>, holds: impl Fn(f64, f64) -> bool) -> bool {
    #[allow(clippy::cast_precision_loss)]
    let count = count as f64;
    bound.is_none_or(|b| holds(count, b))
}

impl JsonSchema {
    fn remembered(&self, base: &str, reference: &str) -> Option<(&str, &Node)> {
        let located = self.located.borrow();
        let located = located.get(base)?.get(reference)?;
        let (outer, _) = self.documents.get_key_value(&located.outer)?;
        let (_, document) = self.documents.get(&located.document)?;
        Some((outer.as_str(), document.at(&located.positions)?))
    }

    /// The subschema a record is validated against: the one the IRI's fragment
    /// names, its `$ref`s resolved against the whole document.
    fn root(&self) -> Result<(&str, &Node)> {
        let run = Run {
            schema: self,
            asked: Asked::Every,
            failures: Vec::new(),
        };
        run.reference(without_fragment(&self.root), &self.root)
            .map_err(|e| Error::adapter(format!("the source schema {}: {e}", self.root)))
    }
}

impl Schema for JsonSchema {
    fn errors(&self, text: &str) -> Result<Vec<SchemaFinding>> {
        let instance = json::parse(text)?;
        let (base, root) = self.root()?;
        let mut run = Run {
            schema: self,
            asked: Asked::Every,
            failures: Vec::new(),
        };
        run.validate(root, base, &instance, &mut Vec::new(), 0)?;
        Ok(run
            .failures
            .into_iter()
            .map(|failure| SchemaFinding {
                body: body(failure.keyword),
                within: (!failure.at.is_empty()).then(|| json::pointer(&failure.at)),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests;
