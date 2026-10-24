use super::run::Compiled;
use super::{Aggregate, Kind, Note, Pattern, Place, Plan, Step};
use std::fmt::{self, Display, Formatter};

pub(super) struct Noted {
    pub(super) source: String,
    pub(super) exists: Vec<Nested>,
}

pub(super) struct Nested {
    memo: Option<usize>,
    lines: Vec<String>,
}

impl Nested {
    pub(super) fn of(memo: Option<usize>, plan: &Plan, labels: &[String]) -> Self {
        Self {
            memo,
            lines: lines(plan, labels),
        }
    }
}

/// A plan made to be printed: it keeps what the plan to run forgets, each variable's
/// label and each expression's text.
pub(crate) struct Explained {
    pub(super) compiled: Compiled,
    pub(super) labels: Vec<String>,
}

impl Display for Explained {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for line in lines(&self.compiled.plan, &self.labels) {
            writeln!(f, "{line}")?;
        }
        writeln!(f, "construct")?;
        for template in &self.compiled.template {
            writeln!(f, "  {}", pattern(template, &self.labels))?;
        }
        Ok(())
    }
}

fn slot(slot: usize, labels: &[String]) -> String {
    format!("?{}", labels[slot])
}

fn slots(slots: &[usize], labels: &[String]) -> String {
    let named: Vec<String> = slots.iter().map(|&at| slot(at, labels)).collect();
    named.join(" ")
}

fn shown(note: &Note) -> &str {
    note.as_ref().map_or("", |noted| noted.source.as_str())
}

fn place(place: &Place, labels: &[String]) -> String {
    match place {
        Place::Const(term) => term.to_string(),
        Place::Var(at) => slot(*at, labels),
        Place::Blank(at) => format!("_:b{at}"),
        Place::Key(at, key) => format!(
            "{}[key {}; reads {}]",
            slot(*at, labels),
            shown(&key.note),
            slots(&key.reads, labels)
        ),
    }
}

fn pattern(pattern: &Pattern, labels: &[String]) -> String {
    format!(
        "{} {} {}",
        place(&pattern.subject, labels),
        place(&pattern.predicate, labels),
        place(&pattern.object, labels)
    )
}

pub(super) fn lines(plan: &Plan, labels: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for each in plan {
        step(each, labels, &mut out);
    }
    out
}

fn indented(inner: Vec<String>, by: usize, out: &mut Vec<String>) {
    out.extend(inner.into_iter().map(|line| format!("{:by$}{line}", "")));
}

fn exists(note: &Note, out: &mut Vec<String>) {
    for nested in note.iter().flat_map(|noted| &noted.exists) {
        out.push(match nested.memo {
            Some(memo) => format!("  exists, remembered as memo {memo}:"),
            None => "  exists, run for each row:".to_owned(),
        });
        indented(nested.lines.clone(), 4, out);
    }
}

fn step(step: &Step, labels: &[String], out: &mut Vec<String>) {
    match step {
        Step::Triple(each) => out.push(format!("scan {}", pattern(each, labels))),
        Step::Filter(_, note) => {
            out.push(format!("filter {}", shown(note)));
            exists(note, out);
        }
        Step::Bind(at, _, note) => {
            out.push(format!("bind {} := {}", slot(*at, labels), shown(note)));
            exists(note, out);
        }
        Step::Agree(at, _, note) => {
            out.push(format!("agree {} with {}", slot(*at, labels), shown(note)));
            exists(note, out);
        }
        Step::Values(at, table) => out.push(format!(
            "values ({}) {} row(s)",
            slots(at, labels),
            table.len()
        )),
        Step::Union(branches) => {
            out.push("union".to_owned());
            for (at, branch) in branches.iter().enumerate() {
                out.push(format!("  branch {at}"));
                indented(lines(branch, labels), 4, out);
            }
        }
        Step::Optional(inner, condition, note) => {
            out.push(match condition {
                Some(_) => format!("optional when {}", shown(note)),
                None => "optional".to_owned(),
            });
            exists(note, out);
            indented(lines(inner, labels), 2, out);
        }
        Step::Group(group) => {
            out.push(format!("group by ({})", slots(&group.keys, labels)));
            for aggregate in &group.aggregates {
                out.push(format!(
                    "  {} := {}",
                    slot(aggregate.slot, labels),
                    kind(aggregate, labels)
                ));
                exists(&aggregate.note, out);
            }
            indented(lines(&group.inner, labels), 2, out);
        }
        Step::Choice(edges) => {
            out.push("one of".to_owned());
            for edge in edges {
                out.push(format!(
                    "  scan {}{}",
                    pattern(&edge.pattern, labels),
                    if edge.backwards { ", backwards" } else { "" }
                ));
            }
        }
        Step::Reach(walk) => out.push(format!(
            "reach {} to {} over {}, skipping [{}]",
            place(&walk.from, labels),
            place(&walk.to, labels),
            if walk.from_itself {
                "zero or more edges"
            } else {
                "one or more edges"
            },
            walk.skipped
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ")
        )),
    }
}

fn kind(aggregate: &Aggregate, labels: &[String]) -> String {
    let argument = shown(&aggregate.note);
    match &aggregate.kind {
        Kind::Count { of: None, .. } => "count(*)".to_owned(),
        Kind::Count {
            of: Some(at),
            distinct,
        } => format!(
            "count({}{})",
            if *distinct { "distinct " } else { "" },
            slot(*at, labels)
        ),
        Kind::Sum(_) => format!("sum({argument})"),
        Kind::Min(_) => format!("min({argument})"),
        Kind::Max(_) => format!("max({argument})"),
        Kind::Sample(_) => format!("sample({argument})"),
    }
}
