"""reference_bugs.json — the places where the port is right and the reference
implementation is not, plus the neighbouring cases where it is right and the
port must not over-correct."""

from __future__ import annotations

from typing import Any

import pytest
from conftest import cases

from tree2 import Tree, TreeSyntaxError, parse


def run(case: dict[str, Any]) -> Tree:
    root = parse(case['input'], 'test')
    path = case['path']

    if case['op'] == 'select':
        return root.select(*path)

    if case['op'] == 'update':
        return root.update([Tree.struct(type) for type in case['update']], *path)[0]

    if case['op'] == 'insert':
        value = None if case['insert'] is None else Tree.struct(case['insert'])
        return root.insert(value, *path)

    raise AssertionError(f'unhandled operation {case["op"]}')


@pytest.mark.parametrize('case', cases('reference_bugs'))
def test_reference_bug(case: dict[str, Any]) -> None:
    if case['op'] == 'parse':
        want = case['error']

        with pytest.raises(TreeSyntaxError) as caught:
            parse(case['input'], 'test')

        error = caught.value
        assert error.reason == want['reason']
        assert (error.span.row, error.span.col, error.span.length) == (
            want['row'],
            want['col'],
            want['length'],
        )
        return

    assert str(run(case)) == case['output']


def test_negative_index_never_matches() -> None:
    root = parse('a\n\tx\n\ty\n', 'test')
    assert root.select('a', -1).kids == ()
    assert root.select('a', -3).kids == ()


def test_empty_update_still_replaces_what_exists() -> None:
    root = parse('a b\n', 'test')
    assert str(root.update([], 'a', 'b')[0]) == 'a\n'


def test_a_dedented_last_line_can_not_vanish() -> None:
    """The reference drops it when it is unterminated, so a file that lost its
    last line feed quietly loses data too."""
    with pytest.raises(TreeSyntaxError, match='Too few tabs'):
        parse('\t\tfoo\n\tbar', 'test')

    with pytest.raises(TreeSyntaxError, match='Too few tabs'):
        parse('\t\ta\nb', 'test')
