from typing import Optional, Union, Callable, Iterable
from .jingle_types import State, ModeledBlock, ResolvedVarNode
from z3 import z3  # type: ignore

__all__ = [
    "AssignmentModel",
    "ConstraintConfig",
    "CrackersConfig",
    "CrackersLogLevel",
    "DecisionResult",
    "GadgetLibraryConfig",
    "MemoryEqualityConstraint",
    "MetaConfig",
    "PointerRange",
    "PointerRangeConstraints",
    "SleighConfig",
    "SpecificationConfig",
    "StateEqualityConstraint",
    "SynthesisConfig",
    "SynthesisParams",
    "SynthesisSelectionStrategy",
    "DecisionResultType",
    "StateConstraintGenerator",
    "TransitionConstraintGenerator",
]

class AssignmentModel:
    def eval_bv(self, bv: z3.BitVecRef, model_completion: bool) -> z3.BitVecRef: ...
    def initial_state(self) -> Optional[State]: ...
    def final_state(self) -> Optional[State]: ...
    def gadgets(self) -> list[ModeledBlock]: ...
    def inputs(self) -> Iterable[ResolvedVarNode]: ...
    def outputs(self) -> Iterable[ResolvedVarNode]: ...
    def input_summary(self, model_completion: bool):
        Iterable[tuple[str, z3.BitVecRef]]
    def output_summary(self, model_completion: bool):
        Iterable[tuple[str, z3.BitVecRef]]

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

class CrackersLogLevel:
    Debug: int
    Error: int
    Info: int
    Trace: int
    Warn: int

class GadgetLibraryConfig:
    max_gadget_length: int
    path: str
    sample_size: Optional[int]
    base_address: Optional[int]

class MemoryEqualityConstraint:
    space: str
    address: int
    size: int
    value: int

class MetaConfig:
    seed: int
    log_level: CrackersLogLevel

class PointerRange:
    min: int
    max: int

class PointerRangeConstraints:
    read: Optional[list[PointerRange]]

class SleighConfig:
    ghidra_path: str

class SpecificationConfig:
    path: str
    max_instructions: int
    base_address: Optional[int]

class StateEqualityConstraint:
    register: Optional[dict[str, int]]
    pointer: Optional[dict[str, str]]
    memory: Optional[MemoryEqualityConstraint]

class SynthesisSelectionStrategy:
    SatStrategy: int
    OptimizeStrategy: int

class SynthesisConfig:
    strategy: SynthesisSelectionStrategy
    max_candidates_per_slot: int
    parallel: int
    combine_instructions: bool

class PythonDecisionResult_AssignmentFound(DecisionResult):
    _0: AssignmentModel
    __match_args__ = ("_0",)

class SelectionFailure:
    indices: list[int]

class PythonDecisionResult_Unsat(DecisionResult):
    _0: SelectionFailure
    pass

class DecisionResult:
    AssignmentFound: PythonDecisionResult_AssignmentFound
    Unsat: PythonDecisionResult_Unsat

DecisionResultType = Union[
    "PythonDecisionResult_AssignmentFound", "PythonDecisionResult_Unsat"
]

StateConstraintGenerator = Callable[[State, int], z3.BitVecRef]
TransitionConstraintGenerator = Callable[[ModeledBlock], z3.BitVecRef]

class SynthesisParams:
    def run(self) -> "DecisionResultType": ...
    def add_precondition(self, fn: StateConstraintGenerator): ...
    def add_postcondition(self, fn: StateConstraintGenerator): ...
    def add_transition_constraint(self, fn: TransitionConstraintGenerator): ...
    pass
