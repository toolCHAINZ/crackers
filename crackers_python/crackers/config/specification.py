from pydantic import BaseModel


class ReferenceProgramConfig(BaseModel):
    """
    Configuration for the Reference Program representing the ROP chain's goal.
    This should be a simple program with no branches ending in either
    a function call or a system call. The code should be at a symbol named
    "_start".
    Attributes:
        path (str): Filesystem path to the reference program binary. This should be an ELF/PE with a symbol named "_start".
        max_instructions (int): Maximum number of instructions to use.
        base_address (int): Base address for loading the reference program.
    """

    path: str
    max_instructions: int
    base_address: int | None
