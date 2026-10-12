use super::{Names, Reads};
use spargebra::algebra::{Expression, GraphPattern};
use spargebra::term::{NamedNodePattern, TermPattern, TriplePattern};

impl Names {
    pub(crate) fn variables_of(&mut self, pattern: &GraphPattern) -> Reads {
        let mut into = Reads::new();
        self.variables(pattern, &mut into);
        into
    }

    fn variables(&mut self, pattern: &GraphPattern, into: &mut Reads) {
        match pattern {
            GraphPattern::Bgp { patterns } => self.triple_variables(patterns, into),
            GraphPattern::Join { left, right }
            | GraphPattern::Union { left, right }
            | GraphPattern::Minus { left, right }
            | GraphPattern::Lateral { left, right } => {
                self.variables(left, into);
                self.variables(right, into);
            }
            GraphPattern::LeftJoin {
                left,
                right,
                expression,
            } => {
                self.variables(left, into);
                self.variables(right, into);
                if let Some(expression) = expression {
                    into.extend(self.read_by(expression));
                }
            }
            GraphPattern::Filter { expr, inner } => {
                into.extend(self.read_by(expr));
                self.variables(inner, into);
            }
            GraphPattern::Extend {
                inner,
                variable,
                expression,
            } => {
                into.insert(self.slot(variable));
                into.extend(self.read_by(expression));
                self.variables(inner, into);
            }
            GraphPattern::Values { variables, .. } | GraphPattern::Project { variables, .. } => {
                into.extend(variables.iter().map(|v| self.slot(v)));
            }
            GraphPattern::Group {
                variables,
                aggregates,
                ..
            } => {
                into.extend(variables.iter().map(|v| self.slot(v)));
                into.extend(aggregates.iter().map(|(v, _)| self.slot(v)));
            }
            GraphPattern::Distinct { inner }
            | GraphPattern::Reduced { inner }
            | GraphPattern::Slice { inner, .. }
            | GraphPattern::OrderBy { inner, .. }
            | GraphPattern::Graph { inner, .. }
            | GraphPattern::Service { inner, .. } => self.variables(inner, into),
            GraphPattern::Path { .. } => {}
        }
    }

    pub(crate) fn triple_variables(&mut self, patterns: &[TriplePattern], into: &mut Reads) {
        for triple in patterns {
            for term in [&triple.subject, &triple.object] {
                if let TermPattern::Variable(v) = term {
                    into.insert(self.slot(v));
                }
            }
            if let NamedNodePattern::Variable(v) = &triple.predicate {
                into.insert(self.slot(v));
            }
        }
    }

    /// Every variable the expression reads, those inside an EXISTS too.
    pub(crate) fn read_by(&mut self, expression: &Expression) -> Reads {
        let mut read = Reads::new();
        self.collect_reads(expression, &mut read, &|_| true);
        read
    }

    /// The variables that must be bound before a conjunct of a filter can be placed:
    /// those it reads, but of an EXISTS only the ones the filtered group holds, since
    /// the others are the EXISTS' own.
    pub(crate) fn needed_to_place(&mut self, conjunct: &Expression, filtered: &Reads) -> Reads {
        let mut needed = Reads::new();
        self.collect_reads(conjunct, &mut needed, &|slot| filtered.contains(slot));
        needed
    }

    fn collect_reads(
        &mut self,
        expression: &Expression,
        into: &mut Reads,
        inside_exists: &dyn Fn(usize) -> bool,
    ) {
        match expression {
            Expression::Variable(v) | Expression::Bound(v) => {
                into.insert(self.slot(v));
            }
            Expression::Exists(body) => {
                let inside = self.variables_of(body);
                into.extend(inside.into_iter().filter(|&slot| inside_exists(slot)));
            }
            Expression::Or(a, b)
            | Expression::And(a, b)
            | Expression::Equal(a, b)
            | Expression::SameTerm(a, b)
            | Expression::Greater(a, b)
            | Expression::GreaterOrEqual(a, b)
            | Expression::Less(a, b)
            | Expression::LessOrEqual(a, b)
            | Expression::Add(a, b)
            | Expression::Subtract(a, b)
            | Expression::Multiply(a, b)
            | Expression::Divide(a, b) => {
                self.collect_reads(a, into, inside_exists);
                self.collect_reads(b, into, inside_exists);
            }
            Expression::UnaryPlus(a) | Expression::UnaryMinus(a) | Expression::Not(a) => {
                self.collect_reads(a, into, inside_exists);
            }
            Expression::In(a, list) => {
                self.collect_reads(a, into, inside_exists);
                for e in list {
                    self.collect_reads(e, into, inside_exists);
                }
            }
            Expression::If(a, b, c) => {
                for e in [a, b, c] {
                    self.collect_reads(e, into, inside_exists);
                }
            }
            Expression::Coalesce(list) | Expression::FunctionCall(_, list) => {
                for e in list {
                    self.collect_reads(e, into, inside_exists);
                }
            }
            Expression::NamedNode(_) | Expression::Literal(_) => {}
        }
    }
}
