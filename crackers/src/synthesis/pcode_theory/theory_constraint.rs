use z3::ast::Bool;

use crate::synthesis::Decision;
use crate::synthesis::pcode_theory::conflict_clause::ConflictClause;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TheoryStage {
    CombinedSemantics,
    Consistency,
    Branch,
    Precondition,
    Postcondition,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConjunctiveConstraint {
    pub decisions: Vec<Decision>,
    boolean: Bool,
    constraint_type: TheoryStage,
}

impl ConjunctiveConstraint {
    pub fn new(decisions: &[Decision], boolean: Bool, t: TheoryStage) -> Self {
        Self {
            decisions: decisions.to_vec(),
            boolean,
            constraint_type: t,
        }
    }
    pub fn get_bool(&self) -> &Bool {
        &self.boolean
    }

    pub fn gen_conflict_clause(&self) -> ConflictClause {
        let mut clause = ConflictClause::from(self.decisions.iter());
        clause.precondition = matches!(self.constraint_type, TheoryStage::Precondition);
        clause.postcondition = matches!(self.constraint_type, TheoryStage::Postcondition);
        clause
    }
}

pub(crate) fn gen_conflict_clauses(
    constraints: &[&ConjunctiveConstraint],
) -> Option<ConflictClause> {
    let mut result = Vec::new();
    let mut semantics = Vec::new();
    for x in constraints {
        result.push(x.gen_conflict_clause());
        match x.constraint_type {
            TheoryStage::CombinedSemantics | TheoryStage::Branch => {
                semantics.push(x.gen_conflict_clause());
            }
            _ => {}
        }
    }
    if result.is_empty() {
        None
    } else if !semantics.is_empty() {
        Some(ConflictClause::combine(semantics.as_slice()))
    } else {
        Some(ConflictClause::combine(result.as_slice()))
    }
}
