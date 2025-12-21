from __future__ import annotations

from typing import Annotated, Literal, Optional, Union

from pydantic import BaseModel, Field

"""
Reference program specification models.

This module offers two explicit variants that map to the Rust enum:

    enum SpecificationConfig {
        BinaryFile(BinaryFileSpecification),
        RawPcode(String),
    }

Use a discriminated union as the top-level type for Pydantic models:
- BinaryFileSpecification : carries `path`, `max_instructions`, and optional `base_address`
- RawPcodeSpecification : carries `raw_pcode`

When constructing a config, provide the `type` field as either "binary" or "raw".
Examples:
    BinaryFileSpecification(type="binary", path="prog.o", max_instructions=16)
    RawPcodeSpecification(type="raw", raw_pcode="(pcode text...)")

The exported name `ReferenceProgramConfig` is a typing alias suitable for use
as a field in other Pydantic models (it is a discriminated union).
"""


class BinaryFileSpecification(BaseModel):
    """
    Binary-file variant of the reference program specification.
    """

    type: Literal["binary"] = "binary"
    path: str
    max_instructions: int
    base_address: Optional[int] = None

    class Config:
        # For pydantic v2 compatibility this is ignored; kept for clarity.
        arbitrary_types_allowed = True

    def __repr__(self) -> str:  # pragma: no cover - simple helper
        return (
            f"BinaryFileSpecification(path={self.path!r}, "
            f"max_instructions={self.max_instructions!r}, "
            f"base_address={self.base_address!r})"
        )


class RawPcodeSpecification(BaseModel):
    """
    Raw p-code variant of the reference program specification.
    """

    type: Literal["raw"] = "raw"
    raw_pcode: str

    def __repr__(self) -> str:  # pragma: no cover - simple helper
        return f"RawPcodeSpecification(raw_pcode={self.raw_pcode!r})"


# Discriminated union type used by other Pydantic models (e.g. CrackersConfig).
# Pydantic will use the `type` field to discriminate between the variants.
ReferenceProgramConfig = Annotated[
    Union[BinaryFileSpecification, RawPcodeSpecification],
    Field(discriminator="type"),
]
