use z3::ast::Bool;

use crate::synthesis::assignment_problem::Decision;
use crate::synthesis::assignment_problem::pcode_theory::ConflictClause;

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
