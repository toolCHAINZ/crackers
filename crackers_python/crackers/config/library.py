from pydantic import BaseModel


class GadgetLibraryConfigWrapper(BaseModel):
    """
    max_gadget_length: the maximum length of a gadget in terms of assembly instructions
    path: the location of the gadget library; this should generally be an ELF or a PE.
    sample_size: the optional max number of gadgets to sample from the binary
    base_address: the base address to re-base the gadget library to
    """

    max_gadget_length: int
    path: str
    sample_size: int | None
    base_address: int | None
