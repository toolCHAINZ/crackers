from z3 import BoolRef

from crackers.config import MetaConfig, LibraryConfig, SleighConfig, \
    ReferenceProgramConfig, SynthesisConfig, ConstraintConfig, CrackersConfig
from crackers.config.constraint import RegisterValuation, CustomStateConstraint
from crackers.config.log_level import LogLevel
from crackers.config.synthesis import SynthesisStrategy
from crackers.jingle_types import State


def my_constraint(s: State, _addr: int) -> BoolRef:
    rdi = s.read_register("rdi")
    rcx = s.read_register("rcx")
    return rdi.eq(rcx ^ 0x5a5a5a5a5a5a5a5a5a)

meta = MetaConfig(log_level=LogLevel.INFO, seed=42)
library = LibraryConfig(max_gadget_length=8, path="libz.so.1", sample_size=None,
                        base_address=None)
sleigh = SleighConfig(ghidra_path="/Applications/ghidra")
reference_program = ReferenceProgramConfig(path="sample.o", max_instructions=8, base_address=library.base_address)
synthesis = SynthesisConfig(strategy=SynthesisStrategy.SAT, max_candidates_per_slot=200, parallel=8, combine_instructions=True)
constraint = ConstraintConfig(precondition=[RegisterValuation(name="rdi", value=0xdeadbeef), CustomStateConstraint(code=my_constraint)],)
config = CrackersConfig(meta=meta, library=library, sleigh=sleigh, reference_program=reference_program, synthesis=synthesis, constraint=constraint)

config.run()
