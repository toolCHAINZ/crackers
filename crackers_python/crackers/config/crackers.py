import json

from pydantic import BaseModel

from crackers import _internal
from crackers.config.constraint import (
    ConstraintConfig,
    CustomStateConstraint,
    CustomTransitionConstraint,
)
from crackers.config.library import LibraryConfig
from crackers.config.meta import MetaConfig
from crackers.config.sleigh import SleighConfig
from crackers.config.specification import ReferenceProgramConfig
from crackers.config.synthesis import SynthesisConfig
from crackers.crackers import DecisionResult


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

    def run(self) -> DecisionResult:
        # Dump to a Python dict so we can transform the `specification` into
        # the enum-shaped representation the Rust side expects (serde enum).
        data = self.model_dump(mode="json")
        spec = data.get("specification")
        if spec is not None and isinstance(spec, dict):
            # Our Python discriminated union uses `type` == "binary" | "raw".
            # Rust expects the enum to be represented as {"BinaryFile": {...}} or
            # {"RawPcode": "<string>"} in the JSON that `from_json` consumes.
            t = spec.get("type")
            if t == "binary":
                data["specification"] = {
                    "BinaryFile": {
                        "path": spec.get("path"),
                        "max_instructions": spec.get("max_instructions"),
                        "base_address": spec.get("base_address"),
                    }
                }
            elif t == "raw":
                data["specification"] = {"RawPcode": spec.get("raw_pcode")}
            else:
                raise ValueError(f"Unknown specification type: {t!r}")
        j = json.dumps(data)
        config = _internal.crackers.CrackersConfig.from_json(j)
        resolved = config.resolve_config()

        # Separate custom state constraints into precondition and postcondition
        precondition_state_constraints: list[CustomStateConstraint] = []
        postcondition_state_constraints: list[CustomStateConstraint] = []
        if self.constraint.precondition:
            precondition_state_constraints = [
                c  # type: ignore
                for c in self.constraint.precondition
                if getattr(c, "type", None) == "custom_state"
            ]
        if self.constraint.postcondition:
            postcondition_state_constraints = [
                c  # type: ignore
                for c in self.constraint.postcondition
                if getattr(c, "type", None) == "custom_state"
            ]

        custom_transition_constraints: list[CustomTransitionConstraint] = []
        if self.constraint.pointer:
            custom_transition_constraints = [
                c  # type: ignore
                for c in self.constraint.pointer
                if getattr(c, "type", None) == "custom_transition"
            ]

        for c in precondition_state_constraints:
            resolved.add_precondition(c._code)
        for c in postcondition_state_constraints:
            resolved.add_postcondition(c._code)
        for d in custom_transition_constraints:
            resolved.add_transition_constraint(d._code)

        return resolved.run()
