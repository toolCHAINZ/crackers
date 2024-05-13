use std::slice;

use crate::synthesis::Decision;

#[derive(Debug, Clone)]
pub enum ConflictClause {
    Unit(Decision),
    Conjunction(Vec<Decision>),
}

impl ConflictClause {
    pub fn combine(clauses: &[ConflictClause]) -> Self {
        let mut result = vec![];
        for x in clauses {
            match x {
                ConflictClause::Conjunction(v) => result.extend(v.clone()),
                ConflictClause::Unit(d) => result.push(*d),
            }
        }
        ConflictClause::Conjunction(result)
    }

    pub fn decisions(&self) -> &[Decision] {
        match self {
            ConflictClause::Unit(decision) => slice::from_ref(decision),
            ConflictClause::Conjunction(d) => d.as_slice(),
        }
    }
}
