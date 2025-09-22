from z3 import BoolRef, BoolVal
from pydantic import ValidationError

from crackers.config import MetaConfig, LibraryConfig, SleighConfig, \
    ReferenceProgramConfig, SynthesisConfig, ConstraintConfig, CrackersConfig
from crackers.config.constraint import RegisterValuation, \
    RegisterStringValuation, MemoryValuation, PointerRange, \
    CustomStateConstraint, CustomTransitionConstraint, PointerRangeRole
from crackers.config.log_level import LogLevel
from crackers.config.synthesis import SynthesisStrategy
from crackers.jingle_types import State, ModeledBlock


# Custom state constraint example
def my_constraint(s: State, _addr: int) -> BoolRef:
    rdi = s.read_register("RDI")
    rcx = s.read_register("RCX")
    return rdi == (rcx ^ 0x5a5a5a5a5a5a5a5a)


# Custom transition constraint example
def my_transition_constraint(block: ModeledBlock) -> BoolRef:
    # Dummy: always true
    return BoolVal(True)


meta = MetaConfig(log_level=LogLevel.INFO, seed=42)
library = LibraryConfig(max_gadget_length=8, path="libz.so.1", sample_size=None,
                        base_address=None)
sleigh = SleighConfig(ghidra_path="/Applications/ghidra")
reference_program = ReferenceProgramConfig(path="sample.o", max_instructions=8, base_address=library.base_address)
synthesis = SynthesisConfig(strategy=SynthesisStrategy.SAT, max_candidates_per_slot=200, parallel=8, combine_instructions=True)
constraint = ConstraintConfig(
    precondition=[
        RegisterValuation(name="rdi", value=0xdeadbeef),
        MemoryValuation(space="ram", address=0x1000, size=4, value=0x41),
        MemoryValuation(space="ram", address=0x1000, size=4, value=0x41),
        RegisterStringValuation(reg="rsi", value="/bin/sh"),
        CustomStateConstraint(code=my_constraint)
    ],
    postcondition=[
        RegisterValuation(name="rax", value=0x1337),
        CustomStateConstraint(code=my_constraint)
    ],
    transition=[
        PointerRange(role=PointerRangeRole.READ, min=0x2000, max=0x3000),
        CustomTransitionConstraint(code=my_transition_constraint)
    ]
)
try:
    config = CrackersConfig(meta=meta, library=library, sleigh=sleigh, specification=reference_program, synthesis=synthesis, constraint=constraint)
    config.run()
except ValidationError as e:
    print("Validation error:")
    print(e)
    print("Errors:", e.errors())
    print("JSON:", e.json(indent=2))
    raise
