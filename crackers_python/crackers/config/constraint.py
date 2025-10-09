from enum import Enum
from typing import Literal, Callable, Annotated, Union

import z3
from pydantic import BaseModel, Field, field_serializer, PrivateAttr

from crackers.jingle import State, ModeledBlock


class MemoryValuation(BaseModel):
    """
    Encodes a constraint that sets a region of memory to a fixed value (essentially a memset).

    Attributes:
        type (Literal["memory"]): Discriminator for this constraint type.
        space (str): The space name, should almost always be "ram".
        address (int): The start address of the buffer.
        size (int): The number of words (usually bytes) to set.
        value (int): The value to set them to; must be a 1-byte value.
    """

    type: Literal["memory"] = "memory"
    space: str
    address: int
    size: int
    value: int


class RegisterValuation(BaseModel):
    """
    Encodes a constraint that sets a register to a fixed integer value.

    Attributes:
        type (Literal["register_value"]): Discriminator for this constraint type.
        name (str): The name of the register.
        value (int): The value to set the register to.
    """

    type: Literal["register_value"] = "register_value"
    name: str
    value: int


class RegisterStringValuation(BaseModel):
    """
    Encodes a constraint that sets a register to point to a buffer in memory
    containing a zero-terminated ASCII copy of the given string.

    Attributes:
        type (Literal["register_string"]): Discriminator for this constraint type.
        reg (str): The name of the register.
        value (str): The string value the register encodes a pointer to.
    """

    type: Literal["register_string"] = "register_string"
    reg: str
    value: str


class PointerRangeRole(Enum):
    READ = "read"
    WRITE = "write"


class PointerRange(BaseModel):
    """
    Encodes a constraint on the usage of pointers in the ROP chain.
    If multiple PointerRange constraints are given, they are combined
    with a logical OR (e.g. the pointer must lie within _one_ of these areas).

    Attributes:
        type (Literal["pointer_range"]): Discriminator for this constraint type.
        role (PointerRangeRole): Whether the pointer is used for reading or writing.
        min (int): Minimum address in the range.
        max (int): Maximum address in the range.
    """

    type: Literal["pointer_range"] = "pointer_range"
    role: PointerRangeRole
    min: int
    max: int


class CustomStateConstraint(BaseModel):
    """
    Custom constraint on a state, defined by a callable.

    This constraint type is used when other constraint variants cannot encode the desired logic. State constraints can be applied as either preconditions (to the initial state before ROP) or postconditions (to the final state after ROP). They are applied to the first or final state of a given gadget.

    The callable provided must accept a symbolic `State` and an optional integer representing the address of the gadget in memory, and must return a `z3.BoolRef` (with `False` indicating the constraint is not satisfied).

    Note:
        - This constraint cannot be produced by deserializing a Pydantic model or JSON schema, as it requires a Python callable.
        - It must be constructed programmatically using `CustomStateConstraint.from_callable`.

    Attributes:
        type (Literal["custom_state"]): Discriminator for this constraint type.
        _code (Callable[[State, int], z3.BoolRef]): Function that generates a z3 constraint for the state.
    """

    type: Literal["custom_state"] = "custom_state"
    _code: Callable[[State, int], z3.BoolRef] = PrivateAttr()

    @classmethod
    def from_callable(cls, code: Callable[[State, int], z3.BoolRef], **kwargs):
        obj = cls(**kwargs)
        obj._code = code
        return obj


class CustomTransitionConstraint(BaseModel):
    """
    Custom constraint on a transition, defined by a callable.

    This constraint type is used when other constraint variants cannot encode the desired logic. T
    ransition constraints are applied to every gadget in the ROP chain.
    The callable must accept a symbolic `ModeledBlock`
    (containing both the starting and ending states, as well as metadata about the gadget)
    and an optional integer, and must return a `z3.BoolRef`
    (with `False` indicating the constraint is not satisfied).

    Note:
        - This constraint cannot be produced by deserializing a Pydantic model or
          JSON schema, as it requires a Python callable.
        - It must be constructed programmatically using `CustomTransitionConstraint.from_callable`.

    Attributes:
        type (Literal["custom_transition"]): Discriminator for this constraint type.
        _code (Callable[[ModeledBlock, int], z3.BoolRef]): Function that generates a z3 constraint for the transition.
    """

    type: Literal["custom_transition"] = "custom_transition"
    _code: Callable[[ModeledBlock], z3.BoolRef] = PrivateAttr()

    @classmethod
    def from_callable(cls, code: Callable[[ModeledBlock], z3.BoolRef], **kwargs):
        obj = cls(**kwargs)
        obj._code = code
        return obj


StateConstraint = Annotated[
    Union[
        MemoryValuation,
        RegisterValuation,
        RegisterStringValuation,
        CustomStateConstraint,
    ],
    Field(discriminator="type"),
]

TransitionConstraint = Annotated[
    Union[PointerRange, CustomTransitionConstraint], Field(discriminator="type")
]


class ConstraintConfig(BaseModel):
    """
    Configuration for constraints applied to the synthesis process.

    Attributes:
        precondition (list[StateConstraint] | None): Constraints on the initial state.
        postcondition (list[StateConstraint] | None): Constraints on the final state.
        pointer (list[TransitionConstraint] | None): Constraints on the transitions between states (named 'pointer' for compatibility reasons, but can express any transition constraint)
    """

    precondition: list[StateConstraint] | None = None
    postcondition: list[StateConstraint] | None = None
    pointer: list[TransitionConstraint] | None = None

    @field_serializer("precondition", "postcondition")
    def serialize_state_constraints(value, _info):
        filtered = [
            v for v in value or [] if getattr(v, "type", None) != "custom_state"
        ]
        memory_vals = [v for v in filtered if isinstance(v, MemoryValuation)]
        memory_dict = None
        if memory_vals:
            if len(memory_vals) > 1:
                import warnings

                warnings.warn(
                    "Multiple memory constraints found; only the first will be serialized."
                )
            mem = memory_vals[0]
            memory_dict = {
                "space": mem.space,
                "address": mem.address,
                "size": mem.size,
                "value": mem.value,
            }
        register_valuations = [v for v in filtered if isinstance(v, RegisterValuation)]
        if register_valuations:
            keys = [reg.name for reg in register_valuations]
            if len(keys) > len(set(keys)):
                import warnings

                warnings.warn(
                    "Multiple register valuation constraints found for a single register; only the last will be serialized."
                )
        transformed = {
            "register": {v.name: v.value for v in register_valuations},
            "memory": memory_dict,
            "pointer": {
                v.reg: v.value
                for v in filtered
                if isinstance(v, RegisterStringValuation)
            },
        }
        return transformed

    @field_serializer("pointer")
    def serialize_transition_constraints(value, _info):
        filtered = [
            v for v in value or [] if getattr(v, "type", None) != "custom_transition"
        ]
        read_ranges = []
        write_ranges = []
        for v in filtered:
            if isinstance(v, PointerRange):
                range_dict = {"min": v.min, "max": v.max}
                if v.role == PointerRangeRole.READ:
                    read_ranges.append(range_dict)
                elif v.role == PointerRangeRole.WRITE:
                    write_ranges.append(range_dict)
        return {"read": read_ranges, "write": write_ranges}
