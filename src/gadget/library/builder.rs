use derive_builder::Builder;
use std::collections::HashSet;

use jingle::sleigh::context::SleighContext;
use jingle::sleigh::OpCode;
use jingle::JingleError;
use rand::random;

use crate::gadget::library::GadgetLibrary;

#[derive(Clone, Debug, Builder)]
#[builder(default)]
pub struct GadgetLibraryParams {
    pub max_gadget_length: usize,
    pub operation_blacklist: HashSet<OpCode>,
    pub seed: i64,
}

impl GadgetLibraryParams {
    pub fn build(&self, sleigh: &SleighContext) -> Result<GadgetLibrary, JingleError> {
        GadgetLibrary::build_from_image(sleigh, self)
    }
}

impl Default for GadgetLibraryParams {
    fn default() -> Self {
        Self {
            max_gadget_length: 4,
            operation_blacklist: HashSet::from([
                // Unlikely to be in any useful chains that we're currently considering
                // While call is potentially going to exist in certain cases (e.g. mmap), we
                // can just as easily redirect to such functions with an indirect jump, so we still remove
                // it from consideration
                OpCode::CPUI_BRANCH,
                OpCode::CPUI_CALL,
                OpCode::CPUI_CBRANCH,
                // The following operations are not yet modeled by jingle, so let's save some trees
                // and not even try to model them for the time being
                OpCode::CPUI_FLOAT_ADD,
                OpCode::CPUI_FLOAT_ABS,
                OpCode::CPUI_FLOAT_CEIL,
                OpCode::CPUI_FLOAT_DIV,
                OpCode::CPUI_FLOAT_EQUAL,
                OpCode::CPUI_FLOAT_FLOAT2FLOAT,
                OpCode::CPUI_FLOAT_FLOOR,
                OpCode::CPUI_FLOAT_INT2FLOAT,
                OpCode::CPUI_FLOAT_LESS,
                OpCode::CPUI_FLOAT_LESSEQUAL,
                OpCode::CPUI_FLOAT_MULT,
                OpCode::CPUI_FLOAT_NAN,
                OpCode::CPUI_FLOAT_NEG,
                OpCode::CPUI_FLOAT_NOTEQUAL,
                OpCode::CPUI_FLOAT_ROUND,
                OpCode::CPUI_FLOAT_SQRT,
                OpCode::CPUI_FLOAT_SUB,
                OpCode::CPUI_FLOAT_TRUNC,
                OpCode::CPUI_CPOOLREF,
                OpCode::CPUI_CAST,
                OpCode::CPUI_MULTIEQUAL,
            ]),
            seed: random(),
        }
    }
}
