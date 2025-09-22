from enum import Enum
from typing import Literal, Callable, Annotated, Union

import z3
from pydantic import BaseModel, Field, field_serializer

from crackers.crackers import StateEqualityConstraint
from crackers.jingle_types import State, ModeledBlock


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
    This should be used if the other constraint variants are unable to
    encode the desired constraint. State constraints can be used
    as either a precondition constraint (applied to the initial state before ROP)
    or a postcondition constraint (applied to the final state of the ROP).
    State constraints are applied to either the first or final state of a given gadget.
    They take in a symbolic State and an optional second argument representing
    the address of the gadget in memory. They must return a Z3 boolean expression,
    with False indicating the constraint is not satisfied.

    Attributes:
        type (Literal["custom_state"]): Discriminator for this constraint type.
        code (Callable[[State, int], z3.BoolRef]): Function that generates a z3 constraint for the state.
    """

    type: Literal["custom_state"] = "custom_state"
    code: Callable[[State, int], z3.BoolRef]


class CustomTransitionConstraint(BaseModel):
    """
    Custom constraint on the transition, defined by a callable.
    This should be used if the other constraint variants are unable to
    encode the desired constraint. Transition constraints are applied
    to every gadget in the ROP chain. They take in a symbolic ModeledBlock
    (which contains both the starting and ending states as well as metadata about the gadget)
    and return a Z3 boolean expression, with False indicating the constraint is not satisfied.

    Attributes:
        type (Literal["custom_transition"]): Discriminator for this constraint type.
        code (Callable[[ModeledBlock, int], z3.BoolRef]): Function that generates a z3 constraint for the transition.
    """

    type: Literal["custom_transition"] = "custom_transition"
    code: Callable[[ModeledBlock, int], z3.BoolRef]


StateConstraint = Annotated[Union[MemoryValuation, RegisterValuation, RegisterStringValuation, CustomStateConstraint], Field(discriminator='type')]
TransitionConstraint = Annotated[Union[PointerRange, CustomTransitionConstraint], Field(discriminator='type')]

class ConstraintConfig(BaseModel):
    """
    Configuration for constraints applied to the synthesis process.

    Attributes:
        precondition (list[StateConstraint] | None): Constraints on the initial state.
        postcondition (list[StateConstraint] | None): Constraints on the final state.
        transition (list[TransitionConstraint] | None): Constraints on the transitions between states.
    """

    precondition: list[StateConstraint] | None = None
    postcondition: list[StateConstraint] | None = None
    transition: list[TransitionConstraint] | None = None

    @field_serializer('precondition', 'postcondition')
    def serialize_preconditions(value, _info):
        filtered = [v for v in value or [] if getattr(v, 'type', None) != 'custom_state']
        memory_vals = [v for v in filtered if isinstance(v, MemoryValuation)]
        memory_dict = None
        if memory_vals:
            if len(memory_vals) > 1:
                import warnings
                warnings.warn("Multiple memory constraints found; only the first will be serialized.")
            mem = memory_vals[0]
            memory_dict = {
                'space': mem.space,
                'address': mem.address,
                'size': mem.size,
                'value': mem.value
            }
        transformed: StateEqualityConstraint = {
            'register': {v.name: v.value for v in filtered if isinstance(v, RegisterValuation)},
            'memory': memory_dict,
            'pointer': {v.reg: v.value for v in filtered if isinstance(v, RegisterStringValuation)},
        }
        return transformed

    @field_serializer('transition')
    def skip_custom_transition_constraints(value, _info):
        return [v for v in value or [] if getattr(v, 'type', None) != 'custom_transition']
