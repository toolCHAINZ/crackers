from pydantic import BaseModel


class SpecificationConfigWrapper(BaseModel):
    """
    path: the location of the compiled reference program. The chain program must
    be located in the binary with a symbol named 'entry'
    max_instructions: a debugging field for truncating reference programs: set this to a large number
    base_address: the base address to re-base the reference program to
    """

    path: str
    max_instructions: int
    base_address: int
