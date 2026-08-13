"""serialize.json and serialize_built.json."""

from __future__ import annotations

from collections.abc import Callable
from typing import Any

import pytest
from conftest import actual_shape, cases, expected_shape

from tree2 import Tree, parse, serialize


@pytest.mark.parametrize('case', cases('serialize'))
def test_serialize(case: dict[str, Any]) -> None:
    assert str(parse(case['input'], 'test')) == case['output']


@pytest.mark.parametrize('case', cases('serialize'))
def test_serialize_is_a_fixed_point(case: dict[str, Any]) -> None:
    once = str(parse(case['input'], 'test'))
    assert str(parse(once, 'test')) == once


# The corpus pins the shape and the output of trees built through the
# factories, but not the recipe — these mirror tools/gen-fixtures.js.
BUILDERS: dict[str, Callable[[], Tree]] = {
    'multiline data splits into kids': lambda: Tree.data('a\nb\nc'),
    'multiline data with extra kids': lambda: Tree.data('a\nb', [Tree.struct('x')]),
    'struct with no kids': lambda: Tree.struct('foo'),
    'struct with one kid collapses inline': lambda: Tree.struct('a', [Tree.struct('b')]),
    'struct with two kids goes multiline': lambda: Tree.struct(
        'a', [Tree.struct('b'), Tree.struct('c')]
    ),
    'list of structs': lambda: Tree.list([Tree.struct('a'), Tree.struct('b')]),
    'empty list': lambda: Tree.list([]),
    'data at root': lambda: Tree.list([Tree.data('x')]),
    'empty data at root': lambda: Tree.list([Tree.data('')]),
    'nested data under struct': lambda: Tree.struct('a', [Tree.data('x'), Tree.data('y')]),
}


@pytest.mark.parametrize('case', cases('serialize_built'))
def test_serialize_built(case: dict[str, Any]) -> None:
    tree = BUILDERS[case['name']]()
    assert actual_shape(tree) == expected_shape(case['tree'])
    assert str(tree) == case['output']


def test_serialize_function_matches_str() -> None:
    tree = parse('a b\n\tc\n', 'test')
    assert serialize(tree) == str(tree)


def test_deep_tree_survives_the_default_recursion_limit() -> None:
    """Nesting deeper than the interpreter's recursion limit still parses and
    serializes: both walks carry their own stack. Two kids per level keep the
    lone-kid collapse from folding it all onto one line."""
    depth = 5000
    lines = ['a\n']

    for level in range(1, depth):
        pad = '\t' * level
        lines.append(f'{pad}\\x\n')
        lines.append(f'{pad}a\n')

    source = ''.join(lines)
    tree = parse(source, 'test')

    assert str(tree) == source
