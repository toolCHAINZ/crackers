use z3::ast::Bool;

use crate::synthesis::pcode_theory::conflict_clause::ConflictClause;
use crate::synthesis::Decision;

const AGGRESSIVE: bool = true;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TheoryStage {
    CombinedSemantics,
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
        let mut clause = ConflictClause::from(self.decisions.clone());
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
    let mut concat = Vec::new();
    let mut pre = Vec::new();
    let mut post = Vec::new();
    for x in constraints {
        result.push(x.gen_conflict_clause());
        match x.constraint_type {
            TheoryStage::CombinedSemantics | TheoryStage::Branch => {
                semantics.push(x.gen_conflict_clause());
            }
            TheoryStage::Consistency => concat.push(x.gen_conflict_clause()),
            TheoryStage::Precondition => pre.push(x.gen_conflict_clause()),
            TheoryStage::Postcondition => post.push(x.gen_conflict_clause()),
        }
    }

    if !semantics.is_empty() {
        if AGGRESSIVE {
            return Some(ConflictClause::combine(semantics.as_slice()));
        }
        match (pre.is_empty(), post.is_empty()) {
            (true, true) => {
                let clause = ConflictClause::combine(semantics.as_slice());
                result.push(clause)
            }
            (true, false) => {
                // only post-condition
                let max_index = semantics
                    .into_iter()
                    .map(|c| c.decisions().iter().map(|d| d.index).max().unwrap())
                    .max()
                    .unwrap();
                let clauses: Vec<ConflictClause> = concat
                    .into_iter()
                    .filter(|f| f.decisions().iter().all(|d| d.index > max_index))
                    .collect();
                result.push(ConflictClause::combine(&clauses))
            }
            (false, true) => {
                // only pre-condition
                let min_index = semantics
                    .into_iter()
                    .map(|c| c.decisions().iter().map(|d| d.index).min().unwrap())
                    .min()
                    .unwrap();
                let clauses: Vec<ConflictClause> = concat
                    .into_iter()
                    .filter(|f| f.decisions().iter().all(|d| d.index <= min_index))
                    .collect();
                result.push(ConflictClause::combine(&clauses))
            }
            (false, false) => {
                // both :(
                semantics.extend_from_slice(&concat);
                result.push(ConflictClause::combine(semantics.as_slice()));
            }
        }
        Some(ConflictClause::combine(result.as_slice()))
    } else if result.is_empty() {
        None
    } else {
        Some(ConflictClause::combine(result.as_slice()))
    }
}
