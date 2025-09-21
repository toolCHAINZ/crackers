<div align="center">

<img src="https://raw.githubusercontent.com/toolCHAINZ/crackers/refs/heads/main/crackers.svg" width="350"/>

</div>


# `crackers`: A Tool for Synthesizing Code-Reuse Attacks from `p-code` Programs

[![Build](https://github.com/toolCHAINZ/crackers/actions/workflows/build.yml/badge.svg)](https://github.com/toolCHAINZ/crackers/actions/workflows/build.yml)
[![docs.rs](https://docs.rs/crackers/badge.svg)](https://docs.rs/crackers)


This repo contains the source code of `crackers`, a procedure for synthesizing
code-reuse attacks (e.g. ROP) build around the Z3 SMT Solver and Ghidra's SLEIGH code translator.

## How does it work?

`crackers` takes as input a "reference program", usually
written in an assembly language, a binary (of the same architecture) in which to look
for gadgets, and user-provided constraints to enforce on synthesized chains. It will always
return an answer (though there is no strict bound to runtime), reporting either that the problem
is UNSAT, or providing an assignment of gadgets that meet all constraints, and a model
of the memory state of the PCODE virtual machine at every step in the chain. This memory model can
then be used to derive the inputs necessary to realize the ROP chain.

`crackers` itself makes _no_ assumptions about the layout of memory in the target program, nor the extent of an attacker's
control over it: it assumes that _all_ system state is usable unless explicitly prohibited through a constraint.
This approach allows for using it with more practical vulnerablities, with the drawback of requiring more human-guided
configuration that many other ROP tools.

To validate chains, `crackers` builds a mathematical model of the execution of a candidate chain and makes assertions on it
against a model generated from a specification (itself expressed as a sequence of PCODE operations). When this verification
returns SAT, `crackers` returns the Z3 model of the memory state of the chain at every point of its execution. This
memory model may be used to derive the contents of memory needed to invoke the chain, and transitively the input needed to
provide to the program to realize it.

`crackers` is available to use as a command-line tool, as a Rust crate, or as a Python package.

### This software is still in alpha and may change at any time

## How do I use it?

You have three options:

### Rust CLI Interface

The simplest way to use it is through its CLI interface. You can install it from `crates.io` by running:

```sh
cargo install --all-features crackers
```

You can then run

```sh
crackers new my_config.toml
```

To generate a new configuration for the tool at `my_config.toml`. This config file can be adjusted
for your use-case and then used with

```sh
crackers synth my_config.toml
```

There's a lot of knobs to turn in this config file. An example file below:

```toml
# location to find a ghidra installation. This is only used for
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
# the maximum number of candidates that are considered for each sub-slice of the specification
# if you don't want to cap this, just set it arbitrarily high. Might make it optional later
max_candidates_per_slot = 50
# The number of chain validation workers to use
parallel = 8

# crackers works by taking in an "example" computation and synthesizing a compatible chain
# right now, it does not support specifications with controlflow
[specification]
# the path at which to find the raw binary containing the bytes of the specification computation
path = "bin/execve_instrs.bin"
# the number of assembly instructions in the specification
max_instructions = 5

# settings involving the file from which to pull gadgets
[library]
# the path to the file. It can be any type of object file that gimli_object can parse (e.g. ELF, PE)
path = "bin/libc_wrapper"
# the maximum length of gadget to extract. Raising this number increases both the complexity of the gadgets
# that are reasoned about and the total number of found gadgets
max_gadget_length = 4
# optionally randomly sample the set of parsed gadgets to a given size
random_sample_size = 20000
# optionally use a set seed for gadget selection
# random_sample_seed = 0x234

# from this point on are constraints that we put on the synthesis
# these are fairly self-explanatory
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

# this constraint enforces that the value pointed to by this register
# must be equal to the given string
[constraint.postcondition.pointer]
RDI = "/bin/sh"

# ANY pointer access, read or write must fall in this range
# might separate read/write later
[constraint.pointer]
min = 0x7fffffffde00
max = 0x7ffffffff000
```

Note that using the CLI, a successful synthesis will print out a listing of the gadgets that were selected,
but not the memory model found in synthesis.

### Rust Crate Usage

[![Crates.io](https://img.shields.io/crates/v/crackers.svg)](https://crates.io/crates/crackers)

`crackers` is on `crates.io` and can be added to your project with:

```sh
cargo add crackers
```

API documentation can be found on [docs.rs](https://docs.rs/crackers/latest/crackers/).

** The API is unstable and largely undocumented at this time. **

### Python Package Usage

[![PyPI](https://img.shields.io/pypi/v/crackers)](https://pypi.org/project/crackers/)

`crackers` is on [pypi](https://pypi.org/project/crackers/)! For every release, we provide wheels for \[MacOS, Windows, Linux\] x \[3.10, 3.11, 3.12, 3.13\].


# Research Paper

`crackers` was developed in support of our research paper _Synthesis of Code-Reuse Attacks from `p-code` Programs_.
You can find the author accepted manuscript [here](https://ora.ox.ac.uk/objects/uuid:906d32ca-407c-4cab-beab-b90200f81d65).
This work has been accepted to [Usenix Security 2025](https://www.usenix.org/conference/usenixsecurity25/presentation/denhoed).

You can cite this work with the following BibTex:

```bibtex
@inproceedings{denhoed2025synthesis,
  title={Synthesis of $\{$Code-Reuse$\}$ Attacks from p-code Programs},
  author={DenHoed, Mark and Melham, Tom},
  booktitle={34th USENIX Security Symposium (USENIX Security 25)},
  pages={395--411},
  year={2025}
}
```
