use std::vec;
use z3::ast::Bool;

use crate::synthesis::pcode_theory::conflict_clause::ConflictClause;
use crate::synthesis::Decision;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TheoryStage {
    CombinedSemantics,
    Consistency,
    Branch,
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

    pub fn gen_conflict_clause(&self) -> ConflictClause {
        match self.constraint_type {
            TheoryStage::Branch => ConflictClause::Unit(self.decisions[0]),
            _ => ConflictClause::Conjunction(self.decisions.clone()),
        }
    }
}

pub(crate) fn gen_conflict_clauses(
    constraints: &[&ConjunctiveConstraint],
) -> Option<ConflictClause> {
    let mut result = Vec::new();
    let mut combined_semantics = Vec::new();
    let mut branch = Vec::new();
    let mut concat = Vec::new();
    for x in constraints {
        match x.constraint_type {
            TheoryStage::CombinedSemantics => {
                combined_semantics.push(x.gen_conflict_clause());
            }
            TheoryStage::Branch => {
                branch.push(x.gen_conflict_clause());
            }
            TheoryStage::Consistency => concat.push(x.gen_conflict_clause()),
        }
    }

    if !combined_semantics.is_empty() {
        let clause = ConflictClause::combine(combined_semantics.as_slice());
        result.push(clause);
    }

    if !branch.is_empty() {
        let clause = ConflictClause::combine(branch.as_slice());
        result.push(clause);
    }

    if !concat.is_empty() && result.is_empty() {
        let clause = ConflictClause::combine(concat.as_slice());
        result.push(clause);
    }
    if result.len() == 0 {
        None
    } else {
        Some(ConflictClause::combine(result.as_slice()))
    }
}
