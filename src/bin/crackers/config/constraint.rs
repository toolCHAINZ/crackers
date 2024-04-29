use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Constraint{
    precondition: Option<StateEqualityConstraint>,
    postcondition: Option<StateEqualityConstraint>,
    pointer: Option<PointerRangeConstraint>,
}


#[derive(Debug, Deserialize)]
pub struct StateEqualityConstraint {
    register: Option<HashMap<String, u64>>,
    memory: Option<MemoryEqualityConstraint>
}

#[derive(Debug, Deserialize)]
pub struct MemoryEqualityConstraint{
    space: String,
    address: u64,
    size: u64
}

#[derive(Debug, Deserialize)]
pub struct PointerRangeConstraint {
    min: u64,
    max: u64
}