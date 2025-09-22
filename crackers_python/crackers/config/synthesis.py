from enum import Enum

from pydantic import BaseModel


class SynthesisStrategy(str, Enum):
    """
    Determines the encoding used for gadget selection.

    Members:
        Sat: The default strategy; should generally be used.
        Optimize: Uses an alternative encoding that biases the algorithm to select shorter gadgets, which may negatively impact synthesis performance.
    """

    SAT = "sat"
    OPTIMIZE = "optimize"


class SynthesisConfig(BaseModel):
    """
    Configuration for synthesis algorithm parameters.

    Attributes:
        strategy (SynthesisStrategy): The gadget selection strategy to use.
        max_candidates_per_slot (int): Number of gadgets to collect for each step of the reference program. Higher values provide more choices but increase runtime.
        parallel (int): Number of worker threads for evaluating candidate chains.
        combine_instructions (bool): Whether to allow synthesis of shorter gadget chains.
    """

    strategy: SynthesisStrategy
    max_candidates_per_slot: int
    parallel: int
    combine_instructions: bool
