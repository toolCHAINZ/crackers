import json

from crackers import StateEqualityConstraint
from pydantic import BaseModel


class MemoryEqualityConstraintWrapper(BaseModel):
    """
    Generates a constraint that sets a region of memory to a fixed value; essentially a memset

    space: the space name, should almost always be "ram"
    address: the start address of the buffer
    size: the number of works (usually bytes) to set
    value: the value to set them to; must be a 1-byte value
    """

    space: str
    address: int
    size: int
    value: int


class RegMapping(BaseModel):
    name: str
    value: int


class PointerMapping(BaseModel):
    reg: str
    value: str


class StateEqualityConstraintWrapper(BaseModel):
    """
    Allows for specifying several common types of state constraints.

    reg: specify equality between registers and concrete values (e.g. eax = 1)
         note that this does NOT support the instruction pointer (e.g. rip, pc)
    pointer: specify that a register point to a null-terminated string (e.g. edx = 'hello')
    memory: specifying that a range in memory is equal to a concrete value (e.g. ram[1000..1010] = 0x00)
    """

    reg: list[RegMapping]
    pointer: list[PointerMapping]
    memory: MemoryEqualityConstraintWrapper | None

    def fixup(self) -> dict:
        j = {}
        if self.reg is not None:
            j["register"] = {a.name: a.value for a in self.reg}
        if self.pointer is not None:
            j["pointer"] = {b.reg: b.value for b in self.pointer}
        if self.memory is not None:
            j["memory"] = self.memory.model_dump_json()
        print(j)
        return j


class PointerRangeWrapper(BaseModel):
    min: int
    max: int


class PointerRangeConstraintsWrapper(BaseModel):
    """
    Specifies allowable ranges in the 'ram' space for gadgets to read and write from
    """

    read: list[PointerRangeWrapper] | None
    write: list[PointerRangeWrapper] | None


class ConstraintConfigWrapper(BaseModel):
    """
    Allows for specifying several common types of constraints on chains. Any
    constraint not expressed in this structure should be implemented separately via
    a closure passed to the `SynthesisParams` structure.

    precondition: state constraints on the initial state of the chain
      (useful for encoding facts about the vulnerability and exploit)
    postcondition: state constraints on the final state of the chain
      (useful for enforcing semantics not represented in the reference program)
    pointer: transition constraints restricting the available memory accessible by gadgets
      (useful for enforcing that a gadget read only from controlled memory)
    """

    precondition: StateEqualityConstraintWrapper | None
    postcondition: StateEqualityConstraintWrapper | None
    pointer: PointerRangeConstraintsWrapper | None
