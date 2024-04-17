use std::cmp::{max, min};

use z3::ast::Bool;

use crate::synthesis::assignment_problem::pcode_theory::ConflictClause;
use crate::synthesis::assignment_problem::Decision;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TheoryStage {
    UnitSemantics,
    CombinedSemantics,
    Consistency,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConjunctiveConstraint<'ctx> {
    pub decisions: Vec<Decision>,
    boolean: Bool<'ctx>,
    constraint_type: TheoryStage,
}

impl<'ctx> ConjunctiveConstraint<'ctx> {
    pub fn new(decisions: &[Decision], boolean: Bool<'ctx>, t: TheoryStage) -> Self {
        Self {
            decisions: decisions.to_vec(),
            boolean,
            constraint_type: t,
        }
    }
    pub fn get_bool(&self) -> &Bool<'ctx> {
        &self.boolean
    }

    pub fn get_type(&self) -> TheoryStage {
        self.constraint_type
    }

    pub fn gen_conflict_clause(&self) -> ConflictClause {
        match self.constraint_type {
            TheoryStage::UnitSemantics => ConflictClause::Unit(self.decisions[0].clone()),
            _ => ConflictClause::Conjunction(self.decisions.clone()),
        }
    }
}

pub(crate) fn gen_conflict_clauses(constraints: &[&ConjunctiveConstraint]) -> Vec<ConflictClause> {
    let mut result = Vec::new();
    let mut combined_semantics = Vec::new();
    let mut consistency = Vec::new();
    for x in constraints {
        match x.constraint_type {
            TheoryStage::UnitSemantics => {
                result.push(x.gen_conflict_clause());
            }
            TheoryStage::CombinedSemantics => {
                combined_semantics.push(x.gen_conflict_clause());
            }
            TheoryStage::Consistency => {
                consistency.push(x.gen_conflict_clause());
            }
        }
    }

    if combined_semantics.len() > 0 {
        let clause = ConflictClause::combine(combined_semantics.as_slice());
        result.push(clause);
    }

    let branch = consistency.into_iter().reduce(|a, b| {
        let max_a = a.decisions().iter().max().unwrap();
        let max_b = b.decisions().iter().max().unwrap();
        let min_a = a.decisions().iter().min().unwrap();
        let min_b = b.decisions().iter().min().unwrap();
        ConflictClause::Conjunction(vec![min(min_a, min_b).clone(), max(max_a, max_b).clone()])
    });
    if let Some(b) = branch {
        result.push(b)
    }
    result
}
