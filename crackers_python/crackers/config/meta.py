from pydantic import BaseModel

from crackers.config.log_level import LogLevel


class MetaConfig(BaseModel):
    """
    Overall settings for the synthesis algorithm that don't
    effect its actual operation.

    Attributes:
        log_level (LogLevel): The logging level to use for the application.
        seed (int): The random seed for reproducibility. Used in gadget sampling.
    """

    log_level: LogLevel
    seed: int

    model_config = {"json_encoders": {LogLevel: lambda v: v.value}}
