import json
from pydantic import BaseModel
from crackers.config.constraint import ConstraintConfig
from crackers.config.library import LibraryConfig
from crackers.config.meta import MetaConfig
from crackers.config.sleigh import SleighConfig
from crackers.config.specification import ReferenceProgramConfig
from crackers.config.synthesis import SynthesisConfig
from crackers import _internal

class CrackersConfig(BaseModel):
    """
    Top-level configuration for the Crackers application.

    Attributes:
        meta (MetaConfig): Metadata configuration.
        library (LibraryConfig): Binary library configuration.
        sleigh (SleighConfig): Sleigh decompiler configuration.
        reference_program (ReferenceProgramConfig): Reference program configuration.
        synthesis (SynthesisConfig): Synthesis algorithm configuration.
        constraint (ConstraintConfig): Constraints for synthesis.
    """
    meta: MetaConfig
    library: LibraryConfig
    sleigh: SleighConfig
    reference_program: ReferenceProgramConfig
    synthesis: SynthesisConfig
    constraint: ConstraintConfig

    def run(self):
        j = self.model_dump_json(indent=2)
        print(j, )
        _internal.crackers.CrackersConfig.from_json(j)

