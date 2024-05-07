use z3::ast::Bool;

use crate::synthesis::pcode_theory::ConflictClause;
use crate::synthesis::Decision;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TheoryStage {
    UnitSemantics,
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

    pub fn get_type(&self) -> TheoryStage {
        self.constraint_type
    }

    pub fn gen_conflict_clause(&self) -> ConflictClause {
        match self.constraint_type {
            TheoryStage::UnitSemantics => ConflictClause::Unit(self.decisions[0].clone()),
            TheoryStage::Branch => ConflictClause::Unit(self.decisions[0].clone()),
            _ => ConflictClause::Conjunction(self.decisions.clone()),
        }
    }
}

pub(crate) fn gen_conflict_clauses(constraints: &[&ConjunctiveConstraint]) -> Vec<ConflictClause> {
    let mut result = Vec::new();
    let mut combined_semantics = Vec::new();
    let mut branch = Vec::new();
    for x in constraints {
        match x.constraint_type {
            TheoryStage::UnitSemantics => {
                result.push(x.gen_conflict_clause());
            }
            TheoryStage::CombinedSemantics => {
                combined_semantics.push(x.gen_conflict_clause());
            }
            TheoryStage::Branch => {
                branch.push(x.gen_conflict_clause());
            }
            _ => {}
        }
    }

    if combined_semantics.len() > 0 {
        let clause = ConflictClause::combine(combined_semantics.as_slice());
        result.push(clause);
    }

    if branch.len() > 0 {
        let clause = ConflictClause::combine(branch.as_slice());
        result.push(clause);
    }
    result
}
