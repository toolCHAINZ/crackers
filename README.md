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

There are many options to configure in this file. An example is below:

```toml
# Location to find a Ghidra installation. This is only used for
# SLEIGH architecture definitions
[sleigh]
ghidra_path = "/Applications/ghidra"

[synthesis]
# crackers separates the ROP chain synthesis problem into a gadget assignment and gadget validation problem.
# This field determines the strategy used for the assignment problem:
# * "sat" is a simple boolean SAT problem encoding gadget assignments
# * "optimize" is a weighted SAT problem, giving preference to shorter gadgets
# Optimize tends to perform better when only one validation worker is present and SAT scales better with more workers
strategy = "sat"
# The maximum number of candidates considered for each sub-slice of the specification
# If you don't want to cap this, just set it arbitrarily high. Might make it optional later
max_candidates_per_slot = 50
# The number of chain validation workers to use
parallel = 8

# crackers works by taking in an "example" computation and synthesizing a compatible chain
# Right now, it does not support specifications with control flow
[specification]
# The path at which to find the raw binary containing the bytes of the specification computation
path = "bin/execve_instrs.bin"
# The number of assembly instructions in the specification
max_instructions = 5

# Settings involving the file from which to pull gadgets
[library]
# The path to the file. It can be any type of object file that gimli_object can parse (e.g., ELF, PE)
path = "bin/libc_wrapper"
# The maximum length of gadget to extract. Raising this number increases both the complexity of the gadgets
# that are reasoned about and the total number of found gadgets
max_gadget_length = 4
# Optionally randomly sample the set of parsed gadgets to a given size
random_sample_size = 20000
# Optionally use a set seed for gadget selection
# random_sample_seed = 0x234

# From this point on are constraints that we put on the synthesis
# These are fairly self-explanatory
[constraint.precondition.register]
RAX = 0
RCX = 0x440f30
RDX = 0x7fffffffe608
RBX = 0x400538
RSP = 0x7fffffffe3b8
RBP = 0x403af0
RSI = 0x7fffffffe5f8
RDI = 1
R8 = 0
R9 = 6
R10 = 0x36f8
R11 = 0x206
R12 = 0x403b90
R13 = 0x0
R14 = 0x4ae018
R15 = 0x400538
GS = 0
FS = 0

[constraint.postcondition.register]
RAX = 0x3b
RSI = 0
RDX = 0

# This constraint enforces that the value pointed to by this register
# must be equal to the given string
[constraint.postcondition.pointer]
RDI = "/bin/sh"

# ANY pointer access, read or write, must fall in this range
# Might separate read/write later
[constraint.pointer]
min = 0x7fffffffde00
max = 0x7ffffffff000
```

Note that using the CLI, a successful synthesis will print out a listing of the gadgets that were selected,
but not the memory model found in synthesis.

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
@inproceedings{denhoed2025synthesis,
  title={Synthesis of ${Code-Reuse}$ Attacks from p-code Programs},
  author={DenHoed, Mark and Melham, Tom},
  booktitle={34th USENIX Security Symposium (USENIX Security 25)},
  pages={395--411},
  year={2025}
}
```
