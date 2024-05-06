use crate::error::CrackersError;
use crate::synthesis::pcode_theory::builder::PcodeTheoryBuilder;
use crate::synthesis::pcode_theory::{ConflictClause, PcodeTheory};
use crate::synthesis::slot_assignments::SlotAssignments;
use std::sync::mpsc::{Receiver, Sender};
use tracing::{event, Level};
use z3::{Config, Context};

pub type TheoryWorkerRequest = SlotAssignments;
pub struct TheoryWorkerResponse {
    pub idx: usize,
    pub assignment: SlotAssignments,
    pub theory_result: Result<Option<Vec<ConflictClause>>, CrackersError>,
}

pub struct TheoryWorker<'ctx> {
    z3: &'ctx Context,
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
            z3,
            id,
            sender,
            receiver,
            theory: builder.build(z3)?,
        })
    }

    pub fn run(&self) {
        for assignment in self.receiver.iter() {
            event!(
                Level::TRACE,
                "Worker {} received assignment: {:?}",
                self.id,
                assignment
            );
            let r = self.theory.check_assignment(&assignment);
            self.sender
                .send(TheoryWorkerResponse {
                    idx: self.id,
                    assignment,
                    theory_result: r,
                })
                .unwrap();
        }
    }
}
