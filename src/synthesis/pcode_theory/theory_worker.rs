use std::sync::mpsc::{Receiver, Sender};

use tracing::{event, instrument, Level};
use z3::Context;

use crate::error::CrackersError;
use crate::synthesis::pcode_theory::builder::PcodeTheoryBuilder;
use crate::synthesis::pcode_theory::conflict_clause::ConflictClause;
use crate::synthesis::pcode_theory::PcodeTheory;
use crate::synthesis::slot_assignments::SlotAssignments;

pub struct TheoryWorkerResponse {
    pub idx: usize,
    pub assignment: SlotAssignments,
    pub theory_result: Result<Option<Vec<ConflictClause>>, CrackersError>,
}

pub struct TheoryWorker<'ctx> {
    id: usize,
    sender: Sender<TheoryWorkerResponse>,
    receiver: Receiver<SlotAssignments>,
    theory: PcodeTheory<'ctx>,
}

impl<'ctx> TheoryWorker<'ctx> {
    pub fn new(
        z3: &'ctx Context,
        id: usize,
        sender: Sender<TheoryWorkerResponse>,
        receiver: Receiver<SlotAssignments>,
        builder: PcodeTheoryBuilder<'ctx>,
    ) -> Result<Self, CrackersError> {
        Ok(Self {
            id,
            sender,
            receiver,
            theory: builder.build(z3)?,
        })
    }

    #[instrument(skip_all)]
    pub fn run(&self) {
        event!(
            Level::TRACE,
            "Worker {} about to wait for messages",
            self.id
        );
        for assignment in self.receiver.iter() {
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
                    return;
                }
            }
        }
    }
}
