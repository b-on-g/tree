"""Mapping between trees and JSON values."""

from __future__ import annotations

import math
import re
from collections.abc import Iterable, Mapping
from datetime import date, datetime
from typing import Any, Final

from .errors import TreeError
from .span import Span
from .tree import Tree

__all__ = ['UNDEFINED', 'from_json', 'to_json']


class _Undefined:
    """JavaScript's `undefined`, which JSON has no room for. A `-` node maps
    to it, and a container drops every entry that holds it."""

    __slots__ = ()

    def __repr__(self) -> str:
        return 'UNDEFINED'

    def __bool__(self) -> bool:
        return False


#: What a `-` node — a comment — converts to.
UNDEFINED: Final = _Undefined()

_PLAIN_KEY = re.compile(r'^[^\n\t\\ ]+$')
_INTEGER = re.compile(r'^[+-]?[0-9]+$')
_DECIMAL = re.compile(r'^[+-]?([0-9]+\.?[0-9]*|\.[0-9]+)([eE][+-]?[0-9]+)?$')
_RADIX = {'0x': 16, '0X': 16, '0o': 8, '0O': 8, '0b': 2, '0B': 2}

_HEX_PER_LINE = 8


def to_json(tree: Tree) -> Any:
    """Converts a tree in the json.tree dialect to a JSON value.

    Returns :data:`UNDEFINED` for a `-` node, which containers drop.
    """

    if not tree.type:

        if all(not kid.type for kid in tree.kids):
            return tree.text()

        if len(tree.kids) != 1:
            raise TreeError(f'Multiple json root at {tree.span}')

        return to_json(tree.kids[0])

    if tree.type == '-':
        return UNDEFINED
    if tree.type == 'true':
        return True
    if tree.type == 'false':
        return False
    if tree.type == 'null':
        return None

    if tree.type == '*':

        obj: dict[str, Any] = {}

        for kid in tree.kids:

            if kid.type == '-':
                continue

            if not kid.kids:
                raise kid.error('Missing json value')

            # a key the format can not spell as a type is a data node, its
            # own kids being the key's lines and the value the last one
            key = kid.type or kid.clone(kid.kids[:-1]).text()
            value = to_json(kid.kids[-1])

            if value is not UNDEFINED:
                obj[key] = value

        return obj

    if tree.type == '/':

        items = []

        for kid in tree.kids:

            if kid.type == '-':
                continue

            value = to_json(kid)

            if value is not UNDEFINED:
                items.append(value)

        return items

    number = _to_number(tree.type)

    if number is None:
        raise TreeError(f'Unknown json type ({tree.type}) at {tree.span}')

    return number


def from_json(value: Any, span: Span | None = None) -> Tree:
    """Converts a JSON value to a tree in the json.tree dialect.

    Beside the JSON types, `bytes` becomes hex dump lines and `date` /
    `datetime` an ISO-8601 string.
    """

    span = Span.unknown if span is None else span

    if value is None:
        return Tree('null', '', (), span)

    if isinstance(value, bool):
        return Tree('true' if value else 'false', '', (), span)

    if isinstance(value, str):
        return Tree.data(value, (), span)

    if isinstance(value, int | float):
        return Tree(_from_number(value), '', (), span)

    if isinstance(value, bytes | bytearray | memoryview):
        return Tree.data(_hex_dump(bytes(value)), (), span)

    if isinstance(value, datetime | date):
        return Tree.data(value.isoformat(), (), span)

    if isinstance(value, Mapping):

        kids = []

        for key, item in value.items():

            if item is UNDEFINED:
                continue

            name = key if isinstance(key, str) else str(key)
            sub = from_json(item, span)

            if _PLAIN_KEY.match(name):
                kids.append(Tree(name, '', (sub,), span))
            else:
                kids.append(Tree.data(name, (sub,), span))

        return Tree('*', '', tuple(kids), span)

    if isinstance(value, Iterable):
        items = [from_json(item, span) for item in value if item is not UNDEFINED]
        return Tree('/', '', tuple(items), span)

    raise TypeError(f'Can not convert {type(value).__name__} to a tree')


def _to_number(literal: str) -> int | float | None:
    """Reads a numeric type the way JavaScript's `Number` would, so that the
    dialect stays the same on both sides. `None` means: not a number."""

    if literal == 'NaN':
        return math.nan

    if _INTEGER.match(literal):
        return int(literal)

    if _DECIMAL.match(literal):
        return float(literal)

    if literal in ('Infinity', '+Infinity'):
        return math.inf
    if literal == '-Infinity':
        return -math.inf

    radix = _RADIX.get(literal[:2])

    if radix is not None:
        try:
            return int(literal[2:], radix)
        except ValueError:
            return None

    return None


def _from_number(value: int | float) -> str:
    """Spells a number the way JavaScript's `String` would."""

    if isinstance(value, int):
        return str(value)

    if math.isnan(value):
        return 'NaN'

    if math.isinf(value):
        return 'Infinity' if value > 0 else '-Infinity'

    if value.is_integer() and abs(value) < 1e21:
        return str(int(value))

    return repr(value)


def _hex_dump(data: bytes) -> str:
    """Uppercase hex bytes, space separated, eight per line."""
    codes = [f'{byte:02X}' for byte in data]
    lines = [
        ' '.join(codes[at : at + _HEX_PER_LINE]) for at in range(0, len(codes), _HEX_PER_LINE)
    ]
    return '\n'.join(lines)
