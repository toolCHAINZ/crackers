from enum import Enum

from pydantic import BaseModel


class SynthesisSelectionStrategyWrapper(str, Enum):
    """
        Determines the encoding used for gadget selection.

        Sat is the "default" and should generally be used.
        Optimize uses an alternative encoding which biases the
        algorithm to try to select shorter gadgets. This can have a
        negative performance impact on the synthesis loop.
    """
    Sat = "sat"
    Optimize = "optimize"


class SynthesisConfigWrapper(BaseModel):
    """
        strategy: the gadget selection strategy to use
        max_candidates_per_slot: the number of gadgets to collect for each step of the reference program. A higher number gives more choices but increases algorithm runtime.
        parallel: the number of worker threads to use for evaluating candidate chains
        combine_instructions: whether to allow synthesis of shorter gadget chains
    """
    strategy: SynthesisSelectionStrategyWrapper
    max_candidates_per_slot: int
    parallel: int
    combine_instructions: bool
