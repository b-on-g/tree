"""The **tree** format — parser, serializer and AST.

`tree` is a whitespace-significant format for structural data with three
lexical ingredients (tab, space, backslash) and no escaping rules at all::

    server
        host \\0.0.0.0
        port \\8080

    >>> from tree2 import parse
    >>> config = parse('server\\n\\thost \\\\0.0.0.0\\n\\tport \\\\8080\\n', 'config')
    >>> config.select('server', 'port', None).text()
    '8080'

This is a port of `$mol_tree2`; see SPEC.md in the repository for the format
and the API contract.
"""

from __future__ import annotations

from .errors import TreeError, TreeSyntaxError
from .json_map import UNDEFINED, from_json, to_json
from .parser import parse
from .serializer import serialize
from .span import Span
from .tree import Belt, Context, Hacker, PathStep, Tree

__all__ = [
    'UNDEFINED',
    'Belt',
    'Context',
    'Hacker',
    'PathStep',
    'Span',
    'Tree',
    'TreeError',
    'TreeSyntaxError',
    'from_json',
    'parse',
    'serialize',
    'to_json',
]

__version__ = '0.1.0'
