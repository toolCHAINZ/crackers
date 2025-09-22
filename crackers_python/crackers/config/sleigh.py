from pydantic import BaseModel


class SleighConfig(BaseModel):
    """
    Configuration for SLEIGH definitions. These are assumed to exist inside a ghidra installation.

    Attributes:
        ghidra_path (str): Filesystem path to the Ghidra installation. This is used for SLEIGH definitions..
    """

    ghidra_path: str
