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
        specification (ReferenceProgramConfig): Reference program configuration.
        synthesis (SynthesisConfig): Synthesis algorithm configuration.
        constraint (ConstraintConfig): Constraints for synthesis.
    """
    meta: MetaConfig
    library: LibraryConfig
    sleigh: SleighConfig
    specification: ReferenceProgramConfig
    synthesis: SynthesisConfig
    constraint: ConstraintConfig

    def run(self):
        j = self.model_dump_json()
        config = _internal.crackers.CrackersConfig.from_json(j)
        resolved = config.resolve_config()

        # Separate custom state constraints into precondition and postcondition
        precondition_state_constraints = []
        postcondition_state_constraints = []
        if self.constraint.precondition:
            precondition_state_constraints = [
                c for c in self.constraint.precondition if getattr(c, 'type', None) == 'custom_state'
            ]
        if self.constraint.postcondition:
            postcondition_state_constraints = [
                c for c in self.constraint.postcondition if getattr(c, 'type', None) == 'custom_state'
            ]

        custom_transition_constraints = []
        if self.constraint.transition:
            custom_transition_constraints = [
                c for c in self.constraint.transition if getattr(c, 'type', None) == 'custom_transition'
            ]

        for c in precondition_state_constraints:
            resolved.add_precondition(c._code)
        for c in postcondition_state_constraints:
            resolved.add_postcondition(c._code)
        for c in custom_transition_constraints:
            resolved.add_transition_constraint(c._code)

        print(resolved.run())
