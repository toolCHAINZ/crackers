use z3::ast::Bool;

use crate::synthesis::assignment_problem::pcode_theory::ConflictClause;
use crate::synthesis::assignment_problem::Decision;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConjunctiveConstraint<'ctx> {
    pub decisions: Vec<Decision>,
    boolean: Bool<'ctx>,
}

impl<'ctx> ConjunctiveConstraint<'ctx> {
    pub fn new(decisions: &[Decision], boolean: Bool<'ctx>) -> Self {
        Self {
            decisions: decisions.to_vec(),
            boolean,
        }
    }
    pub fn get_bool(&self) -> &Bool<'ctx> {
        &self.boolean
    }

    pub fn gen_conflict_clause(&self) -> ConflictClause {
        ConflictClause::Conjunction(self.decisions.clone())
    }

    pub fn is_unit(&self) -> bool {
        self.decisions.len() == 1
    }
}
