<div align="center">

<img src="https://raw.githubusercontent.com/toolCHAINZ/crackers/refs/heads/main/crackers.svg" width="350"/>

</div>

# `crackers`: A Tool for Synthesizing Code-Reuse Attacks from `p-code` Programs

[![Build](https://github.com/toolCHAINZ/crackers/actions/workflows/build.yml/badge.svg)](https://github.com/toolCHAINZ/crackers/actions/workflows/build.yml)
[![docs.rs](https://docs.rs/crackers/badge.svg)](https://docs.rs/crackers)

This package contains the Python bindings for `crackers`, a tool for synthesizing
code-reuse attacks (e.g., ROP) built around the Z3 SMT Solver and Ghidra's SLEIGH code translator.

For more details, please refer to the [GitHub project](https://github.com/toolCHAINZ/crackers).

## Usage

[![PyPI](https://img.shields.io/pypi/v/crackers)](https://pypi.org/project/crackers/)

The easiest way to use `crackers` is through the [PyPI](https://pypi.org/project/crackers/) package. For every release, we provide wheels for `[MacOS, Windows, Linux] x [3.10, 3.11, 3.12, 3.13]`.

A simple usage looks like the following:

```python
import logging

from crackers.crackers import DecisionResult
from crackers.jingle import ModeledBlock, State

logging.basicConfig(level=logging.INFO)

from z3 import BoolRef, BoolVal, simplify

from crackers.config import (
    MetaConfig,
    LibraryConfig,
    SleighConfig,
    ReferenceProgramConfig,
    SynthesisConfig,
    ConstraintConfig,
    CrackersConfig,
)
from crackers.config.constraint import (
    RegisterValuation,
    RegisterStringValuation,
    MemoryValuation,
    PointerRange,
    CustomStateConstraint,
    CustomTransitionConstraint,
    PointerRangeRole,
)
from crackers.config.log_level import LogLevel
from crackers.config.synthesis import SynthesisStrategy


# Custom state constraint example
def my_constraint(s: State, _addr: int) -> BoolRef:
    rdi = s.read_register("RDI")
    rcx = s.read_register("RCX")
    return rdi == (rcx ^ 0x5A5A5A5A5A5A5A5A)


# Custom transition constraint example
def my_transition_constraint(block: ModeledBlock) -> BoolRef:
    # Dummy: always true
    return BoolVal(True)


meta = MetaConfig(log_level=LogLevel.INFO, seed=42)
library = LibraryConfig(
    max_gadget_length=8, path="libz.so.1", sample_size=None, base_address=None
)
sleigh = SleighConfig(ghidra_path="/Applications/ghidra")
reference_program = ReferenceProgramConfig(
    path="sample.o", max_instructions=8, base_address=library.base_address
)
synthesis = SynthesisConfig(
    strategy=SynthesisStrategy.SAT,
    max_candidates_per_slot=200,
    parallel=8,
    combine_instructions=True,
)

constraint = ConstraintConfig(
    precondition=[
        RegisterValuation(name="RDI", value=0xDEADBEEF),
        MemoryValuation(space="ram", address=0x1000, size=4, value=0x41),
        RegisterStringValuation(reg="RSI", value="/bin/sh"),
        CustomStateConstraint.from_callable(my_constraint),
    ],
    postcondition=[
        RegisterValuation(name="RBX", value=0x1337),
    ],
    pointer=[
        PointerRange(role=PointerRangeRole.READ, min=0x80_0000, max=0x80_8000),
        CustomTransitionConstraint.from_callable(my_transition_constraint),
    ],
)
config = CrackersConfig(
    meta=meta,
    library=library,
    sleigh=sleigh,
    specification=reference_program,
    synthesis=synthesis,
    constraint=constraint,
)
r = config.run()
match r:
    case DecisionResult.AssignmentFound(a):
        for g in a.gadgets():
            for i in g.instructions:
                print(i.disassembly)
            print()
        for name, bv in a.input_summary(True):
            print(f"{name} = {simplify(bv)}")
```

# Research Paper

`crackers` was initially developed in support of our research paper, _Synthesis of Code-Reuse Attacks from `p-code` Programs_,
presented at [Usenix Security 2025](https://www.usenix.org/conference/usenixsecurity25/presentation/denhoed).

If you found the paper or the implementation useful, you can cite it with the following BibTeX:

```bibtex
@inproceedings{denhoed2025synthesis,
  title={Synthesis of ${Code-Reuse}$ Attacks from p-code Programs},
  author={DenHoed, Mark and Melham, Tom},
  booktitle={34th USENIX Security Symposium (USENIX Security 25)},
  pages={395--411},
  year={2025}
}
```
