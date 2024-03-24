use jingle::sleigh::{GeneralizedVarNode, IndirectVarNode, Instruction, VarNode};

use crate::gadget::Gadget;

#[derive(Clone, Debug)]
pub struct OutputSignature {
    outputs: Vec<GeneralizedVarNode>,
}

impl OutputSignature {
    /// For now this is very naive; just want a very rough filter to make sure we aren't
    /// throwing completely pointless work at z3
    pub fn may_cover(&self, other: &OutputSignature) -> bool {
        let self_direct: Vec<&VarNode> = self
            .outputs
            .iter()
            .filter_map(|i| match i {
                GeneralizedVarNode::Direct(d) => Some(d),
                GeneralizedVarNode::Indirect(_) => None,
            })
            .collect();
        let self_indirect: Vec<&IndirectVarNode> = self
            .outputs
            .iter()
            .filter_map(|i| match i {
                GeneralizedVarNode::Indirect(d) => Some(d),
                GeneralizedVarNode::Direct(_) => None,
            })
            .collect();
        for other_output in &other.outputs {
            match other_output {
                GeneralizedVarNode::Direct(d) => {
                    if !self_direct.iter().any(|dd| dd.overlaps(d)) {
                        return false;
                    }
                }
                GeneralizedVarNode::Indirect(i) => {
                    if !self_indirect
                        .iter()
                        .any(|ii| ii.pointer_location.overlaps(&i.pointer_location))
                    {
                        return false;
                    }
                }
            }
        }
        true
    }
}
impl From<&Instruction> for OutputSignature {
    fn from(value: &Instruction) -> Self {
        let mut outputs = Vec::new();
        for op in &value.ops {
            if let Some(op) = op.output() {
                outputs.push(op);
            }
        }
        Self { outputs }
    }
}

impl From<&Gadget> for OutputSignature {
    fn from(value: &Gadget) -> Self {
        let mut outputs = Vec::new();
        for op in value.instructions.iter().map(|i| &i.ops).flatten() {
            if let Some(op) = op.output() {
                outputs.push(op);
            }
        }
        Self { outputs }
    }
}

#[cfg(test)]
mod tests {
    use jingle::sleigh::GeneralizedVarNode::Direct;
    use jingle::sleigh::VarNode;

    use crate::gadget::signature::OutputSignature;

    #[test]
    fn test_complete_overlap() {
        let o1 = OutputSignature {
            outputs: vec![Direct(VarNode {
                size: 4,
                space_index: 0,
                offset: 0,
            })],
        };
        let o2 = OutputSignature {
            outputs: vec![Direct(VarNode {
                size: 4,
                space_index: 0,
                offset: 0,
            })],
        };
        assert!(o1.may_cover(&o2));
        assert!(o2.may_cover(&o1));
    }

    #[test]
    fn test_partial_overlap() {
        let o1 = OutputSignature {
            outputs: vec![Direct(VarNode {
                size: 4,
                space_index: 0,
                offset: 0,
            })],
        };
        let o2 = OutputSignature {
            outputs: vec![Direct(VarNode {
                size: 4,
                space_index: 0,
                offset: 3,
            })],
        };
        assert!(o1.may_cover(&o2));
        assert!(o2.may_cover(&o1));
    }

    #[test]
    fn test_non_overlap() {
        let o1 = OutputSignature {
            outputs: vec![Direct(VarNode {
                size: 4,
                space_index: 0,
                offset: 0,
            })],
        };
        let o2 = OutputSignature {
            outputs: vec![Direct(VarNode {
                size: 4,
                space_index: 0,
                offset: 4,
            })],
        };
        assert!(!o1.may_cover(&o2));
        assert!(!o2.may_cover(&o1));
    }

    #[test]
    fn test_different_lengths() {
        let o1 = OutputSignature {
            outputs: vec![Direct(VarNode {
                size: 2,
                space_index: 0,
                offset: 7,
            })],
        };
        let o2 = OutputSignature {
            outputs: vec![
                Direct(VarNode {
                    size: 4,
                    space_index: 0,
                    offset: 4,
                }),
                Direct(VarNode {
                    size: 4,
                    space_index: 0,
                    offset: 8,
                }),
                Direct(VarNode {
                    size: 4,
                    space_index: 0,
                    offset: 12,
                }),
                Direct(VarNode {
                    size: 4,
                    space_index: 0,
                    offset: 16,
                }),
            ],
        };
        assert!(o2.may_cover(&o1));
        assert!(!o1.may_cover(&o2));
    }
}
