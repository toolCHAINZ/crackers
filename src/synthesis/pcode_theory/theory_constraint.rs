use z3::ast::Bool;

use crate::synthesis::pcode_theory::conflict_clause::ConflictClause;
use crate::synthesis::Decision;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TheoryStage {
    Consistency,
    Branch,
    Precondition,
    Postcondition,
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
    for x in constraints {
        result.push(x.gen_conflict_clause());
    }
    if result.is_empty() {
        None
    } else {
        let c = ConflictClause::combine(result.as_slice());
        if c.decisions().is_empty() {
            None
        } else {
            Some(c)
        }
    }
}
