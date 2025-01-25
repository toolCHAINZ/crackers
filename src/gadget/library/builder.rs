use std::collections::HashSet;

use derive_builder::Builder;
use jingle::sleigh::OpCode;
use serde::{Deserialize, Serialize};

use crate::config::error::CrackersConfigError;
use crate::config::object::load_sleigh;
use crate::config::sleigh::SleighConfig;
use crate::gadget::library::GadgetLibrary;

#[derive(Clone, Debug, Default, Builder, Deserialize, Serialize)]
#[builder(default)]
pub struct GadgetLibraryParams {
    pub max_gadget_length: usize,
    #[serde(skip, default = "default_blacklist")]
    pub operation_blacklist: HashSet<OpCode>,
    pub path: String,
    pub sample_size: Option<usize>,
    pub base_address: Option<u64>,
}

impl GadgetLibraryParams {
    pub fn build(&self, sleigh: &SleighConfig) -> Result<GadgetLibrary, CrackersConfigError> {
        let mut library_sleigh = load_sleigh(&self.path, sleigh)?;
        if let Some(addr) = self.base_address {
            library_sleigh.set_base_address(addr)
        }
        GadgetLibrary::build_from_image(library_sleigh, self).map_err(CrackersConfigError::Sleigh)
    }
}

fn default_blacklist() -> HashSet<OpCode> {
    HashSet::from([
        // Unlikely to be in any useful chains that we're currently considering
        // While call is potentially going to exist in certain cases (e.g. mmap), we
        // can just as easily redirect to such functions with an indirect jump, so we still remove
        // it from consideration
        OpCode::CPUI_BRANCH,
        OpCode::CPUI_CALL,
        // The following operations are not yet modeled by jingle, so let's save some trees
        // and not even try to model them for the time being
        OpCode::CPUI_CBRANCH,
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
    ])
}
