use std::cmp::Ordering;

use crate::gadget::Gadget;
use jingle::modeling::ModeledBlock;
use jingle::sleigh::{
    ArchInfoProvider, GeneralizedVarNode, IndirectVarNode, Instruction, SpaceType, VarNode,
};
use tracing::trace;

#[derive(Clone, Debug)]
pub struct GadgetSignature {
    outputs: Vec<GeneralizedVarNode>,
}

impl GadgetSignature {
    /// For now this is very naive; just want a very rough filter to make sure we aren't
    /// throwing completely pointless work at z3
    pub fn covers(&self, other: &GadgetSignature) -> bool {
        trace!("{:?} vs {:?}", self.outputs, other.outputs);
        varnode_set_covers(&self.outputs, &other.outputs)
    }

    #[allow(unused)]
    fn has_indirect_output(&self) -> bool {
        self.outputs
            .iter()
            .any(|o| matches!(o, GeneralizedVarNode::Indirect(_)))
    }
}

impl PartialEq<GadgetSignature> for GadgetSignature {
    fn eq(&self, other: &GadgetSignature) -> bool {
        self.outputs
            .iter()
            .all(|f| other.outputs.iter().any(|o| f.eq(o)))
    }
}

impl PartialOrd<GadgetSignature> for GadgetSignature {
    fn partial_cmp(&self, other: &GadgetSignature) -> Option<Ordering> {
        match (self.covers(other), other.covers(self)) {
            (true, true) => Some(Ordering::Equal),
            (true, false) => Some(Ordering::Greater),
            (false, true) => Some(Ordering::Less),
            (false, false) => None,
        }
    }
}
impl GadgetSignature {
    pub(crate) fn from_instr<T: ArchInfoProvider>(value: &Instruction, t: &T) -> Self {
        let mut outputs = Vec::new();

        for op in &value.ops {
            if let Some(op) = op.output() {
                if let GeneralizedVarNode::Direct(v) = &op {
                    // todo: fix this once the new syntax is in stable
                    #[allow(clippy::collapsible_if)]
                    if let Some(h) = t.get_space_info(v.space_index) {
                        if h._type == SpaceType::IPTR_PROCESSOR {
                            outputs.push(op);
                        }
                    }
                } else {
                    outputs.push(op);
                }
            }
        }
        Self { outputs }
    }
}

impl From<&ModeledBlock<'_>> for GadgetSignature {
    fn from(value: &ModeledBlock) -> Self {
        let mut outputs = Vec::new();
        let mut inputs = Vec::new();
        for x in &value.instructions {
            for op in &x.ops {
                if let Some(op) = op.output() {
                    outputs.push(op);
                }
                inputs.extend(op.inputs())
            }
        }
        Self { outputs }
    }
}

impl From<&Gadget> for GadgetSignature {
    fn from(value: &Gadget) -> Self {
        let mut outputs = Vec::new();
        for op in value.instructions.iter().flat_map(|i| &i.ops) {
            if let Some(op) = op.output() {
                if let GeneralizedVarNode::Direct(v) = &op {
                    // todo: fix this once the new syntax is in stable
                    #[allow(clippy::collapsible_if)]
                    if let Some(h) = value.spaces.get(v.space_index) {
                        if h._type == SpaceType::IPTR_PROCESSOR {
                            outputs.push(op);
                        }
                    }
                } else {
                    outputs.push(op);
                }
            }
        }
        Self { outputs }
    }
}

fn varnode_set_covers(our_set: &[GeneralizedVarNode], other_set: &[GeneralizedVarNode]) -> bool {
    let self_direct: Vec<&VarNode> = our_set
        .iter()
        .filter_map(|i| match i {
            GeneralizedVarNode::Direct(d) => Some(d),
            GeneralizedVarNode::Indirect(_) => None,
        })
        .collect();
    let self_indirect: Vec<&IndirectVarNode> = our_set
        .iter()
        .filter_map(|i| match i {
            GeneralizedVarNode::Indirect(d) => Some(d),
            GeneralizedVarNode::Direct(_) => None,
        })
        .collect();
    for other_output in other_set {
        match other_output {
            GeneralizedVarNode::Direct(d) => {
                if !self_direct.iter().any(|dd| dd.covers(d)) {
                    return false;
                }
            }
            GeneralizedVarNode::Indirect(i) => {
                if !self_indirect.iter().any(|ii| {
                    ii.pointer_location.covers(&i.pointer_location)
                        && ii.access_size_bytes >= i.access_size_bytes
                }) {
                    return false;
                }
            }
        }
    }
    true
}
#[cfg(test)]
mod tests {
    use jingle::sleigh::GeneralizedVarNode::Direct;
    use jingle::sleigh::VarNode;

    use crate::gadget::signature::GadgetSignature;

    #[test]
    fn test_complete_overlap() {
        let o1 = GadgetSignature {
            outputs: vec![Direct(VarNode {
                size: 4,
                space_index: 0,
                offset: 0,
            })],
        };
        let o2 = GadgetSignature {
            outputs: vec![Direct(VarNode {
                size: 4,
                space_index: 0,
                offset: 0,
            })],
        };
        assert!(o1.covers(&o2));
        assert!(o2.covers(&o1));
        assert!(o1 >= o2);
        assert!(o1 <= o2);
        assert_eq!(o1, o2);
    }

    #[test]
    fn test_partial_overlap() {
        let o1 = GadgetSignature {
            outputs: vec![Direct(VarNode {
                size: 4,
                space_index: 0,
                offset: 0,
            })],
        };
        let o2 = GadgetSignature {
            outputs: vec![Direct(VarNode {
                size: 4,
                space_index: 0,
                offset: 3,
            })],
        };
        assert_ne!(o1, o2);
        assert!(!o1.covers(&o2));
        assert!(!o2.covers(&o1));
    }

    #[test]
    fn test_non_overlap() {
        let o1 = GadgetSignature {
            outputs: vec![Direct(VarNode {
                size: 4,
                space_index: 0,
                offset: 0,
            })],
        };
        let o2 = GadgetSignature {
            outputs: vec![Direct(VarNode {
                size: 4,
                space_index: 0,
                offset: 4,
            })],
        };
        assert!(!o1.covers(&o2));
        assert!(!o2.covers(&o1));
    }

    #[test]
    fn test_different_lengths() {
        let o1 = GadgetSignature {
            outputs: vec![Direct(VarNode {
                size: 2,
                space_index: 0,
                offset: 7,
            })],
        };
        let o2 = GadgetSignature {
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
        assert!(o2.covers(&o1));
        assert!(!o1.covers(&o2));
    }
}
