<div align="center">

<img src="https://raw.githubusercontent.com/toolCHAINZ/crackers/refs/heads/main/crackers.svg" width="350"/>

</div>


# `crackers`: A Tool for Synthesizing Code-Reuse Attacks from `p-code` Programs

This repo contains the source code of `crackers`, a procedure for synthesizing
code-reuse attacks (e.g. ROP). `crackers` takes as input a specification computation, usually
written in an assembly language, a binary (of the same architecture) in which to look
for gadgets, and user-provided constraints to enforce on synthesized chains. It will always
return an answer (though there is no strict bound to runtime), reporting either that the problem
is UNSAT, or providing an assignment of gadgets that meet all constraints, and a model
of the memory state of the PCODE virtual machine at every stage of the comptuation.

### This software is still in alpha and may change at any time

## CLI Usage

```sh
cargo install --all-features --path . 
```

This will install the `crackers` binary in your path. `crackers` takes a single command line argument,
pointing to a config file. An example file follows:

```toml
# location to find a ghidra installation. This is only used for
# locating architecture definitions
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

A successful synthesis will print out a listing of the gadgets were selected.

## Library Usage

`crackers` intended mode of use is as a library. All of the above settings from the config correspond
to settings that can be set programmatically by API consumers.

When using the API, rather than getting a listing of gadgets as an output, you get a model of the synthesized chain.
This model of the chain includes information about what gadgets were selected as well as a Z3 `Model` representing the
memory at all states of execution in the gadget chain. This model can be queried to derive the memory conditions
necessary to execute the chain.

### Constraints

Constraints work a little differently with the API. Instead of specifying registers and register equality,
`crackers` allows consumers to provide a closure of the following types:

```rust
pub type StateConstraintGenerator = dyn for<'a, 'b> Fn(&'a Context, &'b State<'a>) -> Result<Bool<'a>, CrackersError>
    + Send
    + Sync
    + 'static;
pub type PointerConstraintGenerator = dyn for<'a, 'b> Fn(
        &'a Context,
        &'b ResolvedVarnode<'a>,
        &'b State<'a>,
    ) -> Result<Option<Bool<'a>>, CrackersError>
    + Send
    + Sync
    + 'static;
```

The first type is used for asserting initial and final space constraints. These functions take a z3 context, and a handle
to the program state, returning a `Result<Bool>`. The decision procedure will automatically evaluate
provided functions and assert the booleans they return.

The second type is used for asserting read/write invariants. These functions take in a handle to z3, as well as a struct containing
the bitvector corresponding to the read/write address, as well as the state the read/write is being performed on. 
Any time a chain reads or writes from memory, the procedure will automatically call these functions and assert the returned
booleans. This can allow for setting safe/unsafe ranges of memory or even the register space.

# Research Paper

`crackers` was developed in support of our research paper _Synthesis of Code-Reuse Attacks from `p-code` Programs_.
You can find the author accepted manuscript [here](https://ora.ox.ac.uk/objects/uuid:906d32ca-407c-4cab-beab-b90200f81d65).
This work has been accepted to [Usenix Security 2025](https://www.usenix.org/conference/usenixsecurity25/presentation/denhoed).

You can cite this work with the following BibTex:

```bibtex
@inproceedings {denhoed2025synthesis,
author = {Mark DenHoed and Thomas Melham},
title = {Synthesis of Code-Reuse Attacks from p-code Programs},
booktitle = {34th USENIX Security Symposium (USENIX Security 25)},
year = {2025},
address = {Seattle, WA},
publisher = {USENIX Association},
month = aug
}
```
