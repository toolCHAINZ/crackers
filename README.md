# `crackers`: A Decision Procedure for Code Reuse Attack Synthesis

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

## Library Usage

`crackers` can also be used as a library. All of the above settings from the config correspond
to settings that can be set programmatically by API consumers.

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

## How it Works (Roughly)

### Library generation

A library is taken in and parsed. The executable sections are identified. For every executable section,
we attempt disassembly at every byte offset. If disassembly succeeds and returns a terminating basic block
within N instructions (where N is set in the config), then we call that a gadget and save it.

### Candidate selection

`crackers` works by taking in an "example" computation and synthesizing a chain that is compatible with the example.
So for instance, if you want to call execve on linux, your example computation might look like this:

```
00000000  4889c7             mov     rdi, rax
00000003  48c7c03b000000     mov     rax, 0x3b
0000000a  48c7c600000000     mov     rsi, 0x0
00000011  48c7c200000000     mov     rdx, 0x0
00000018  0f05               syscall 
```

This computation sets `rax`, `rsi`, and `rdx` to set values, and `rdi` to some indeterminate value, which we will
constrain later.

For each instruction in this computation, we identify a set of "gadget candidates". These candidates are selected out of
the library we assembled. To be a candidate for an instruction a gadget must pass the following checks:
* If the specification instruction contains a jump, the gadget must have terminating control flow that is capable of branching
  to the same destination.
* The gadget must write to every direct address that the specification instruction writes to.
* For every indirect access, the gadget must also make an indirect access using the same pointer storage, of at least
  as many bytes.
* Taken in isolation, the gadget must be able to have the same effect as the specification instruction:
  (e.g. `mov eax, ebx` can stand in for `mov eax, 0`, but `mov eax, 2` cannot).


### Decision Loop

The overall flow of the procedure is as follows:

* Ask the assignment problem for an assignment
    * If it returns UNSAT, then no possible assignment exists (under the given parameters) and we return
    * If it returns SAT, then we send that assignment to the theory solver
      * If the PCODE theory solver returns SAT, we have a valid chain
      * If the PCODE theory solver returns UNSAT, it also provides a set of conflict clauses identifying
        which `decisions` participated in the UNSAT proof. These clauses are communicated back to the assignment
        problem to allow it to outlaw that combination of `decisions`.

We introduce parallelism to this workflow by running the PCODE theory solvers in threads and generating multiple
unique assignments for each worker to solve in parallel.

A description of the assignment problem and the theory problem follow: 
### Assignment Problem Setup

Once all the candidates have been found for all instructions, we check and make sure that we have found
at least one candidate for each. If any instruction has no candidates, then we immediately return UNSAT. This
usually indicates that we just did not sample a gadget that touches the needed memory.

For each spec instruction, each candidate is assigned an index. The same gadget can exist as a candidate for multiple indices and
each copy is treated as logically separate from each other.

Using these indices, we construct a simple boolean SAT problem:
* We define a `decision` as a tuple `(i: usize, c: usize)`, indicating that index `i` is using choice `c`. A `decision`
  uniquely identifies a given gadget being used in a given slot.
* Each decision is mapped to a Z3 Bool.
* We then construct a boolean SAT problem using these variables with the following constraints:
  * For all indices `i`:
    * We make exactly 1 choice `c` (e.g. for every `i`, one AND ONLY ONE `decision` with matching `i` must be true)
* In the case of the `optimize` solver, we additionally impose a penalty on every `decision`, proportional to the 
  number of instructions in the gadget. This pressures the solver into selecting the shortest gadgets that it can. 

### PCODE Theory Problem Setup

This procedure runs operates on a single assignment of gadgets. This assignment is evaluated against
the specification computation, as well as any provided constraints.

First, we form the specification computation into a trace, by asserting state equality between the end state
of every instruction and the beginning state of its successor.

Then, we do the same for our assignment of gadgets. We tag these assertions as being `memory` assertions.

We assert all preconditions on the initial state of the gadget chain, and all postconditions on the final state of the gadget chain.
We tag these as `constraint` assertions.

For every instruction `i` and its corresponding gadget `g`:
* Assert that for all `v` in `output(i)`: `g[v]` = `i[v]`.
* If `i` has control flow, assert that the control flow of `g` branches to the same destination as `i`
* These are tagged as `semantic` assertions.

For every gadget `g1` and its successor `g2`:
* Assert that the address of `g2` is the jump target of `g1`. These are tagged as `branch` assertions. The conflict
  associated with a `branch` assertions only references `g1` instead of the conjunction of `g1` and `g2` because,
  as a heuristic, when `g1` is unable to branch to `g2` it is almost always because of some conflict in `g1`, and not
  something about the address of `g2`

We give all these assertions to `z3` using the `assert_and_track` API, which makes Z3 express the unsat core
in terms of booleans representing our varying assertions.

If z3 comes back with SAT, then the chain assignment is valid.

If it comes back UNSAT, then we analyze the UNSAT CORE:

* If the UNSAT core is composed only of `memory` and `constraint` assertions, then, as the formulae are currently tracked,
  we have no way to make strong conflicts ouf of this core. As a fallback, we simply return a clause outlawing the complete
  assignment.
* Otherwise, we form a conjunction of all participating `decisions` for all `branch` and `semantic` conflicts
  and return that to the assignment problem.