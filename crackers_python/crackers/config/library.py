from pydantic import BaseModel


class LoadedLibraryConfig(BaseModel):
    """
    Represents an additional library to load alongside the main library.

    Attributes:
        path (str): Filesystem path of the target binary.
        base_address (int | None): Optional base address for loading the library (may be adjusted/aligned on the Rust side).
    """

    path: str
    base_address: int | None


class LibraryConfig(BaseModel):
    """
    Configuration for a binary library used in analysis or exploitation.

    Attributes:
        max_gadget_length (int): Maximum length of gadgets to consider measured in assembly instructions.
        path (str): Filesystem path of the target binary.
        sample_size (int | None): Maximum number of gadgets to randomly sample (None to use all gadgets).
        base_address (int | None): Base address for loading the library, or None if not specified.
        loaded_libraries (list[LoadedLibraryConfig] | None): Optional additional libraries to load alongside the primary one.
    """

    max_gadget_length: int
    path: str
    sample_size: int | None
    base_address: int | None
    loaded_libraries: list[LoadedLibraryConfig] | None = None
