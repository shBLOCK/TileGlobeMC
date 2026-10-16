from dataclasses import dataclass
from pathlib import Path
from typing import Literal, ClassVar, Iterable, Self

from mashumaro.config import BaseConfig
from mashumaro.mixins.json import DataClassJSONMixin
from mashumaro.types import Discriminator

from .misc import ResLoc, MOD_DATAGEN_ROOT, load_json


__all__ = (
    "Property",
    "BooleanProperty",
    "IntegerProperty",
    "EnumProperty",
    "BlockDef",
)


@dataclass(frozen=True, kw_only=True)
class Property(DataClassJSONMixin):
    name: str
    id_group_size: int

    class Config(BaseConfig):
        discriminator = Discriminator(field="type", include_subtypes=True)


@dataclass(frozen=True, kw_only=True)
class BooleanProperty(Property):
    type: Literal["boolean"] = "boolean"


@dataclass(frozen=True, kw_only=True)
class IntegerProperty(Property):
    type: Literal["integer"] = "integer"
    min: int
    max: int


@dataclass(frozen=True, kw_only=True)
class EnumProperty(Property):
    type: Literal["enum"] = "enum"
    values: tuple[str, ...]


@dataclass(frozen=True, kw_only=True)
class BlockDef(DataClassJSONMixin):
    BLOCK_DEF_ROOT: ClassVar[Path] = MOD_DATAGEN_ROOT / "block_def"

    resloc: ResLoc
    default_state: int
    id_base: int
    total_states: int
    blockstate_properties: tuple[Property, ...]

    @classmethod
    def load(cls, resloc: ResLoc) -> Self:
        return cls.from_dict(load_json(cls.BLOCK_DEF_ROOT / resloc))

    @classmethod
    def all_reslocs(cls) -> Iterable[ResLoc]:
        return ResLoc.list_resloc_files_in_dir(cls.BLOCK_DEF_ROOT)

    @classmethod
    def load_all(cls) -> Iterable[Self]:
        for resloc, _ in cls.all_reslocs():
            yield cls.load(resloc)


if __name__ == '__main__':
    print(BlockDef.load(ResLoc("vault")))
    # for resloc, _ in BlockDef.all_reslocs() {
    # 
    # }
