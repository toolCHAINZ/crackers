from typing import Any, Union, List, Optional, Iterable, Callable
import z3  # Assuming Z3 is imported for type annotations


class SpaceInfo:
    # Placeholder for SpaceInfo class
    ...

class VarNode:
    def __init__(self, space_index: int, offset: int, size: int) -> None: ...

class RawVarNodeDisplay:
    def __init__(self, offset: int, size: int, space_info: SpaceInfo) -> None: ...

class VarNodeDisplay:
    def __init__(self, raw: RawVarNodeDisplay = ..., register: tuple[str, VarNode] = ...) -> None: ...
    # Represents the enum variants Raw and Register
    raw: RawVarNodeDisplay
    register: tuple[str, VarNode]


class ResolvedIndirectVarNode:
    def __init__(self, pointer: Any, pointer_space_info: SpaceInfo, access_size_bytes: int) -> None: ...

    def pointer_bv(self) -> z3.BitVecRef: ...
    def space_name(self) -> str: ...
    def access_size(self) -> int: ...


class ResolvedVarNode:
    """
    Represents the PythonResolvedVarNode enum with two variants:
    - Direct: Contains a VarNodeDisplay
    - Indirect: Contains a ResolvedIndirectVarNode
    """
    def __init__(self, value: Union[VarNodeDisplay, ResolvedIndirectVarNode]) -> None: ...
    value: Union[VarNodeDisplay, ResolvedIndirectVarNode]


class PcodeOperation:
    pass


class Instruction:
    """
    Represents a Python wrapper for a Ghidra instruction.
    """
    disassembly: str
    def pcode(self) -> List[PcodeOperation]: ...

class State:
    def __init__(self, jingle: JingleContext) -> State: ...

    def varnode(self, varnode: ResolvedVarNode) -> z3.BitVecRef: ...
    def register(self, name: str) -> z3.BitVecRef: ...
    def ram(self, offset: int, length: int) -> z3.BitVecRef: ...

class ModeledInstruction:
    original_state: State
    final_state: State

    def get_input_vns(self) -> Iterable[ResolvedVarNode]: ...
    def get_output_vns(self) -> Iterable[ResolvedVarNode]: ...

class ModeledBlock:
    instructions: list[Instruction]
    original_state: State
    final_state: State
    def get_input_vns(self) -> Iterable[ResolvedVarNode]: ...
    def get_output_vns(self) -> Iterable[ResolvedVarNode]: ...

class JingleContext:
    def __init__(self, binary_path: str, ghidra: str) -> JingleContext: ...
    def model_instruction_at(self, offset: int) -> ModeledInstruction: ...
    def model_block_at(self, offset: int, max_instrs: int) -> ModeledBlock: ...

class SleighContext:
    """
    Represents a Sleigh context in python.
    """
    def __init__(self, binary_path: str, ghidra: str) -> SleighContext: ...
    base_address: int
    def instruction_at(self, offset: int) -> Optional[Instruction]: ...
    def make_jingle_context(self) -> JingleContext: ...


"""
Begin crackers types
"""

class CrackersLogLevel:
    Debug = CrackersLogLevel.Debug
    Error = CrackersLogLevel.Error
    Info = CrackersLogLevel.Info
    Trace = CrackersLogLevel.Trace
    Warn = CrackersLogLevel.Warn

class MetaConfig:
    seed: int
    log_level: CrackersLogLevel

class SpecificationConfig:
    path: str
    max_instructions: int
    base_address: Optional[int]

class GadgetLibraryConfig:
    max_gadget_length: int
    path: str
    sample_size: Optional[int]
    base_address: Optional[int]

class SleighConfig:
    ghidra_path: str

class SynthesisSelectionStrategy:
    SatStrategy = SynthesisSelectionStrategy.SatStrategy
    OptimizeStrategy = SynthesisSelectionStrategy.OptimizeStrategy

class SynthesisConfig:
    strategy: SynthesisSelectionStrategy
    max_candidates_per_slot: int
    parallel: int
    combine_instructions: bool

class MemoryEqualityConstraint:
    space: str
    address: int
    size: int
    value: int

class StateEqualityConstraint:
    register: Optional[dict[str, int]]
    pointer: Optional[dict[str, str]]
    memory: Optional[MemoryEqualityConstraint]

class PointerRange:
    min: int
    max: int
class PointerRangeConstraints:
    read: Optional[list[PointerRange]]
class ConstraintConfig:
    precondition: Optional[StateEqualityConstraint]
    postcondition: Optional[StateEqualityConstraint]
    pointer: Optional[PointerRangeConstraints]


class CrackersConfig:
    meta: MetaConfig
    spec: SpecificationConfig
    library: GadgetLibraryConfig
    sleigh: SleighConfig
    synthesis: SynthesisConfig
    constraint: ConstraintConfig

    @classmethod
    def from_toml_file(cls, path: str) -> CrackersConfig: ...
    @classmethod
    def from_json(cls, j: str) -> CrackersConfig: ...

    def resolve_config(self) -> SynthesisParams: ...
    def to_json(self) -> str: ...

class AssignmentModel:
    def eval_bv(self, bv: z3.BitVecRef, model_completion: bool) -> z3.BitVecRef: ...
    def initial_state(self) -> Optional[State]: ...
    def final_state(self) -> Optional[State]: ...
    def gadgets(self) -> list[ModeledBlock]: ...
    def inputs(self) -> Iterable[ResolvedVarNode]: ...
    def outputs(self) -> Iterable[ResolvedVarNode]: ...
    def input_summary(self, model_completion: bool): Iterable[tuple[str, z3.BitVecRef]]
    def output_summary(self, model_completion: bool): Iterable[tuple[str, z3.BitVecRef]]


class PythonDecisionResult_AssignmentFound(DecisionResult):
    _0: AssignmentModel
    __match_args__ = ('_0',)


class SelectionFailure:
    indices: list[int]

class PythonDecisionResult_Unsat(DecisionResult):
    _0: SelectionFailure
    pass

class DecisionResult:
    AssignmentFound : PythonDecisionResult_AssignmentFound
    Unsat : PythonDecisionResult_Unsat

DecisionResultType = Union[DecisionResult.AssignmentFound, DecisionResult.Unsat]

type StateConstraintGenerator = Callable[[State, int], z3.BitVecRef]
type TransitionConstraintGenerator = Callable[[ModeledBlock], z3.BitVecRef]
class SynthesisParams:
    def run(self) -> DecisionResultType: ...
    def add_precondition(self, fn: StateConstraintGenerator): ...
    def add_postcondition(self, fn: StateConstraintGenerator): ...
    def add_transition_constraint(self, fn: TransitionConstraintGenerator): ...
    pass