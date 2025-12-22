<div align="center">

<img src="https://raw.githubusercontent.com/toolCHAINZ/crackers/refs/heads/main/crackers.svg" width="350"/>

</div>


# `crackers`: A Tool for Synthesizing Code-Reuse Attacks from `p-code` Programs

[![Build](https://github.com/toolCHAINZ/crackers/actions/workflows/build.yml/badge.svg)](https://github.com/toolCHAINZ/crackers/actions/workflows/build.yml)
[![docs.rs](https://docs.rs/crackers/badge.svg)](https://docs.rs/crackers)

This repository contains the source code for `crackers`, a tool for synthesizing
code-reuse attacks (e.g., ROP) built around the Z3 SMT Solver and Ghidra's SLEIGH code translator.

## How does it work?

`crackers` takes as input a "reference program," usually
written in an assembly language, a binary (of the same architecture) in which to look
for gadgets, and user-provided constraints to enforce on synthesized chains. It will always
return an answer (though there is no strict bound on runtime), reporting either that the problem
is UNSAT or providing an assignment of gadgets that meet all constraints, along with a model
of the memory state of the PCODE virtual machine at every step in the chain. This memory model can
then be used to derive the inputs necessary to realize the ROP chain.

`crackers` itself makes _no_ assumptions about the layout of memory in the target program, nor the extent of an attacker's
control over it: it assumes that _all_ system state is usable unless explicitly prohibited through a constraint.
This approach increases flexibility, with the drawback of requiring more human-guided
configuration than many other ROP tools.

To validate chains, `crackers` builds a mathematical model of the execution of a candidate chain and makes assertions on it
against a model generated from a specification (itself expressed as a sequence of PCODE operations). When this verification
returns SAT, `crackers` returns the Z3 model of the memory state of the chain at every point of its execution. This
memory model may be used to derive the contents of memory needed to invoke the chain, and transitively the input needed to
provide to the program to realize it.

`crackers` is available as a command-line tool, a Rust crate, or a Python package.

### This software is still in alpha and may change at any time

## How do I use it?

You have three options:

### Python Package

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
    BinaryFileSpecification,
    ConstraintConfig,
    CrackersConfig,
    LibraryConfig,
    MetaConfig,
    ReferenceProgramConfig,
    SleighConfig,
    SynthesisConfig,
)
from crackers.config.constraint import (
    CustomStateConstraint,
    CustomTransitionConstraint,
    MemoryValuation,
    PointerRange,
    PointerRangeRole,
    RegisterStringValuation,
    RegisterValuation,
)
from crackers.config.log_level import LogLevel
from crackers.config.specification import BinaryFileSpecification, RawPcodeSpecification
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


pcode = """
RBX = COPY 0x1337:8
BRANCH *[ram]0xdeadbeef:8
"""

meta = MetaConfig(log_level=LogLevel.DEBUG, seed=42)
library = LibraryConfig(
    max_gadget_length=8, path="libnscgi.so", sample_size=None, base_address=None
)
sleigh = SleighConfig(ghidra_path="/Applications/ghidra")
reference_program = RawPcodeSpecification(raw_pcode=pcode)
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
            print(f"{name} = {hex(simplify(bv).as_long())}")

```

### Rust CLI

You can install the `crackers` CLI from `crates.io` by running:

```sh
cargo install --all-features crackers
```

You can then run:

```sh
crackers new my_config.toml
```

to generate a new configuration for the tool at `my_config.toml`. This config file can be adjusted
for your use case and then used with:

```sh
crackers synth my_config.toml
```

There are many options to configure in this file. An example (created with `crackers new`) is below:

```toml
[meta]
seed = -6067361534454702534
log_level = "INFO"

[specification]
RawPcode = """
              EDI = COPY 0xdeadbeef:4
              ESI = COPY 0x40:4
              EDX = COPY 0x7b:4
              EAX = COPY 0xfacefeed:4
              BRANCH 0xdeadbeef:1
              """

[library]
max_gadget_length = 5
path = "libc.so.6"

[sleigh]
ghidra_path = "/Applications/ghidra"

[synthesis]
strategy = "sat"
max_candidates_per_slot = 200
parallel = 6
combine_instructions = true

[constraint.precondition.register]
RSP = 0x80000000

[constraint.postcondition]

[[constraint.pointer.read]]
min = 0x7fffff80
max = 0x80000080

[[constraint.pointer.write]]
min = 0x7fffff80
max = 0x80000080

```

#### CLI Output

When synthesis succeeds, the CLI will print:

1. **A listing of selected gadgets** - The assembly instructions for each gadget in the chain
2. **Assignment Model Details** - A detailed breakdown including:
   - **Inputs (Locations Read)** - All register and memory locations read by each gadget, along with their evaluated values from the model
   - **Outputs (Locations Written)** - All register and memory locations written by each gadget, along with their evaluated values at the end of the chain

_Note: The models produced through the CLI only represent the transitions within a chain. They do not constrain the 
system state to redirect execution to the chain. 
If you need to encode constraints for redirecting execution to your chain, consider using the Rust or Python API._

### Rust Crate

[![Crates.io](https://img.shields.io/crates/v/crackers.svg)](https://crates.io/crates/crackers)

`crackers` is on `crates.io` and can be added to your project with:

```sh
cargo add crackers
```

API documentation can be found on [docs.rs](https://docs.rs/crackers/latest/crackers/).

** The API is unstable and largely undocumented at this time. **

# Research Paper

`crackers` was initially developed in support of our research paper, _Synthesis of Code-Reuse Attacks from `p-code` Programs_,
presented at [Usenix Security 2025](https://www.usenix.org/conference/usenixsecurity25/presentation/denhoed).

If you found the paper or the implementation useful, you can cite it with the following BibTeX:

```bibtex
@inproceedings{10.5555/3766078.3766099,
author = {DenHoed, Mark and Melham, Tom},
title = {Synthesis of code-reuse attacks from p-code programs},
year = {2025},
isbn = {978-1-939133-52-6},
publisher = {USENIX Association},
address = {USA},
booktitle = {Proceedings of the 34th USENIX Conference on Security Symposium},
articleno = {21},
numpages = {17},
location = {Seattle, WA, USA},
series = {SEC '25}
}
```
