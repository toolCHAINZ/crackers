
from pydantic import BaseModel


class SleighConfigWrapper(BaseModel):
    """
        ghidra_path: the installation path of ghidra
    """
    ghidra_path: str