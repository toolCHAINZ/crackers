use std::collections::HashSet;

use crate::synthesis::Decision;

#[derive(Debug, Clone)]
pub struct ConflictClause {
    decisions: Vec<Decision>,
    pub precondition: bool,
    pub postcondition: bool,
}

impl ConflictClause {
    pub fn combine(clauses: &[ConflictClause]) -> Self {
        let mut decisions = HashSet::new();
        let mut precondition = false;
        let mut postcondition = false;
        for x in clauses {
            for decision in &x.decisions {
                decisions.insert(*decision);
            }
            precondition |= x.precondition;
            postcondition |= x.postcondition;
        }
        Self {
            decisions: decisions.into_iter().collect(),
            precondition,
            postcondition,
        }
    }

    pub fn decisions(&self) -> &[Decision] {
        self.decisions.as_slice()
    }

    pub fn includes_index(&self, d: usize) -> bool {
        self.decisions.iter().any(|i| i.index == d)
    }
}

impl<'a, T: Iterator<Item = &'a Decision>> From<T> for ConflictClause {
    fn from(value: T) -> Self {
        Self {
            decisions: value.cloned().collect(),
            precondition: false,
            postcondition: false,
        }
    }
}
impl From<Decision> for ConflictClause {
    fn from(value: Decision) -> Self {
        Self {
            decisions: vec![value],
            precondition: false,
            postcondition: false,
        }
    }
}
