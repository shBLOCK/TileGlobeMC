import json
import os
from collections.abc import Iterable
from dataclasses import dataclass
from os import PathLike
from pathlib import Path
from typing import Any, Self, ClassVar, final, overload

from mashumaro import DataClassDictMixin
from mashumaro.types import SerializableType

__all__ = (
    "PROJECT_ROOT",
    "MOD_DATAGEN_ROOT",
    "ResLoc",
    "load_json",
)

PROJECT_ROOT = Path(__file__).parents[3]
MOD_DATAGEN_ROOT = PROJECT_ROOT / "neoforge/serverData/tileglobemc/"


@final
@dataclass(init=False, frozen=True, slots=True)
class ResLoc(SerializableType):
    MINECRAFT: ClassVar[str] = "minecraft"

    namespace: str
    path: str

    @overload
    def __init__(self, string: str, /):
        pass

    @overload
    def __init__(self, namespace: str, path: str, /):
        pass

    def __init__(self, *args):
        match args:
            case (namespace, path):
                pass
            case (string, ):
                match string.count(':'):
                    case 0:
                        namespace = ResLoc.MINECRAFT
                        path = string
                    case 1:
                        namespace, path = string.split(':')
                    case _:
                        raise ValueError(f"Invalid resource location string: \"{string}\"")
            case _:
                raise ValueError(args)

        object.__setattr__(self, "namespace", namespace)
        object.__setattr__(self, "path", path)

    def __str__(self) -> str:
        return f"{self.namespace}:{self.path}"

    def __fspath__(self) -> str:
        return f"{self.namespace}/{self.path}"

    @classmethod
    def _deserialize(cls, value: str) -> Self:
        return cls(value)

    @classmethod
    def list_resloc_files_in_dir(cls, root: PathLike) -> Iterable[tuple[ResLoc, Path]]:
        root = Path(root)
        for path in root.rglob("*"):
            if not path.is_file():
                continue
            parts = path.relative_to(root).with_suffix("").parts
            resloc = ResLoc(parts[0], "/".join(parts[1:]))
            yield resloc, path
    
    def is_minecraft(self):
        return self.namespace == ResLoc.MINECRAFT
    
    def as_ident(self) -> str:
        ident = self.path.replace("/", "_")
        if self.namespace != ResLoc.MINECRAFT:
            ident = f"{self.namespace}_{ident}"
        return ident

def load_json(path: PathLike) -> Any:
    return json.load(Path(path).with_suffix(".json").open())
