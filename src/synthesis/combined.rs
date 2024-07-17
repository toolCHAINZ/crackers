use crate::error::CrackersError;
use crate::synthesis::AssignmentSynthesis;

pub struct CombinedAssignmentSynthesis<'a>{
    synth_iterator: Box<dyn  Iterator<Item = Result<AssignmentSynthesis<'a>, CrackersError>>>
}