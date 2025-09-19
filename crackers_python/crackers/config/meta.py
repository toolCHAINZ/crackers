from pydantic import BaseModel

from crackers.config.log_level import CrackersLogLevelWrapper


class MetaConfigWrapper(BaseModel):
    """
    log_level: the level of logging to use in crackers
    seed: the random seed to use when randomly selecting gadgets
    """

    log_level: CrackersLogLevelWrapper
    seed: int
