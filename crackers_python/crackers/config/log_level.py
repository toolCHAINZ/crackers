from enum import Enum


class CrackersLogLevelWrapper(str, Enum):
    Debug = "DEBUG"
    Error = "ERROR"
    Info = "INFO"
    Trace = "TRACE"
    Warn = "WARN"
