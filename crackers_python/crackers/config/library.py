from pydantic import BaseModel


class LibraryConfig(BaseModel):
    """
    Configuration for a binary library used in analysis or exploitation.

    Attributes:
        max_gadget_length (int): Maximum length of gadgets to consider measured in assembly instructions.
        path (str): Filesystem path of the target binary.
        sample_size (int | None): Maximum number of gadgets to randomly sample (None to use all gadgets).
        base_address (int | None): Base address for loading the library, or None if not specified.
    """

    max_gadget_length: int
    path: str
    sample_size: int | None
    base_address: int | None
