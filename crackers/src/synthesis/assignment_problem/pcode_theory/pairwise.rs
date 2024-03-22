use z3::ast::Bool;

use crate::synthesis::assignment_problem::Decision;
use crate::synthesis::assignment_problem::pcode_theory::ConflictClause;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairwiseConstraint<'ctx> {
    decision_a: Decision,
    decision_b: Decision,
    boolean: Bool<'ctx>,
}

impl<'ctx> PairwiseConstraint<'ctx> {
    pub fn new(decision_a: Decision, decision_b: Decision, boolean: Bool<'ctx>) -> Self {
        Self {
            decision_a,
            decision_b,
            boolean,
        }
    }
    pub fn get_bool(&self) -> &Bool<'ctx> {
        &self.boolean
    }

    pub fn gen_conflict_clause(&self) -> ConflictClause {
        ConflictClause::Conjunction(vec![self.decision_a.clone(), self.decision_b.clone()])
    }
}
