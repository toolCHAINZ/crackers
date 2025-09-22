import json

from pydantic import BaseModel

from crackers.config.constraint import ConstraintConfigWrapper
from crackers.config.library import LibraryConfig
from crackers.config.meta import MetaConfig
from crackers.config.sleigh import SleighConfig
from crackers.config.specification import ReferenceProgramConfig
from crackers.config.synthesis import SynthesisConfig
from crackers.crackers_types import CrackersConfig


class CrackersConfigWrapper(BaseModel):
    meta: MetaConfig
    library: LibraryConfig
    sleigh: SleighConfig
    specification: ReferenceProgramConfig
    synthesis: SynthesisConfig
    constraint: ConstraintConfigWrapper

    def translate(self):
        j = self.model_dump()
        if self.constraint.precondition is not None:
            precondition_fixup = self.constraint.precondition.fixup()
            j["constraint"]["precondition"] = precondition_fixup
        if self.constraint.postcondition is not None:
            postcondition_fixup = self.constraint.postcondition.fixup()
            j["constraint"]["postcondition"] = postcondition_fixup
        return CrackersConfig.from_json(json.dumps(j))
