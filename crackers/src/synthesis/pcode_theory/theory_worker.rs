use std::sync::mpsc::{Receiver, Sender};

use jingle::modeling::ModeledInstruction;
use tracing::{Level, event, instrument};

use crate::error::CrackersError;
use crate::synthesis::pcode_theory::PcodeTheory;
use crate::synthesis::pcode_theory::builder::PcodeTheoryBuilder;
use crate::synthesis::pcode_theory::conflict_clause::ConflictClause;
use crate::synthesis::slot_assignments::SlotAssignments;

pub struct TheoryWorkerResponse {
    pub idx: usize,
    pub assignment: SlotAssignments,
    pub theory_result: Result<Option<ConflictClause>, CrackersError>,
}

pub struct TheoryWorker {
    id: usize,
    sender: Sender<TheoryWorkerResponse>,
    receiver: Receiver<SlotAssignments>,
    theory: PcodeTheory<ModeledInstruction>,
}

impl TheoryWorker {
    pub fn new(
        id: usize,
        sender: Sender<TheoryWorkerResponse>,
        receiver: Receiver<SlotAssignments>,
        builder: PcodeTheoryBuilder,
    ) -> Result<Self, CrackersError> {
        Ok(Self {
            id,
            sender,
            receiver,
            theory: builder.build()?,
        })
    }

    pub fn run(&self) {
        event!(
            Level::TRACE,
            "Worker {} about to wait for messages",
            self.id
        );
        for assignment in self.receiver.iter() {
            self.evaluate(assignment)
        }
        event!(Level::TRACE, "Worker {} exiting", self.id);
    }

    #[instrument(skip(self), fields(self.id, assignment = %assignment))]
    fn evaluate(&self, assignment: SlotAssignments) {
        event!(
            Level::TRACE,
            "Worker {} received assignment: {:?}",
            self.id,
            assignment
        );
        let r = self.theory.check_assignment(&assignment);
        match self.sender.send(TheoryWorkerResponse {
            idx: self.id,
            assignment,
            theory_result: r,
        }) {
            Ok(_) => {}
            Err(_) => {
                event!(Level::TRACE, "Exiting worker {}", self.id);
            }
        }
    }
}
