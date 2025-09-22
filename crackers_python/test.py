from z3 import BoolRef
from pydantic import ValidationError

from crackers.config import MetaConfig, LibraryConfig, SleighConfig, \
    ReferenceProgramConfig, SynthesisConfig, ConstraintConfig, CrackersConfig
from crackers.config.constraint import RegisterValuation, CustomStateConstraint
from crackers.config.log_level import LogLevel
from crackers.config.synthesis import SynthesisStrategy
from crackers.jingle_types import State


def my_constraint(s: State, _addr: int) -> BoolRef:
    rdi = s.read_register("rdi")
    return rdi.eq(rdi)

meta = MetaConfig(log_level=LogLevel.INFO, seed=42)
library = LibraryConfig(max_gadget_length=8, path="libz.so.1", sample_size=None,
                        base_address=None)
sleigh = SleighConfig(ghidra_path="/Applications/ghidra")
reference_program = ReferenceProgramConfig(path="sample.o", max_instructions=8, base_address=library.base_address)
synthesis = SynthesisConfig(strategy=SynthesisStrategy.SAT, max_candidates_per_slot=200, parallel=8, combine_instructions=True)
constraint = ConstraintConfig(precondition=[RegisterValuation(name="rdi", value=0xdeadbeef), CustomStateConstraint(code=my_constraint)],)
try:
    config = CrackersConfig(meta=meta, library=library, sleigh=sleigh, specification=reference_program, synthesis=synthesis, constraint=constraint)
    config.run()
except ValidationError as e:
    print("Validation error:")
    print(e)
    print("Errors:", e.errors())
    print("JSON:", e.json(indent=2))
    raise

