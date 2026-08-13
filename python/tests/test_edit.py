"""insert.json and update.json."""

from __future__ import annotations

from typing import Any

import pytest
from conftest import cases

from tree2 import Tree, TreeError, parse


@pytest.mark.parametrize('case', cases('insert'))
def test_insert(case: dict[str, Any]) -> None:
    root = parse(case['input'], 'test')
    value = None if case['insert'] is None else Tree.struct(case['insert'])
    assert str(root.insert(value, *case['path'])) == case['output']


@pytest.mark.parametrize('case', cases('update'))
def test_update(case: dict[str, Any]) -> None:
    root = parse(case['input'], 'test')
    values = [Tree.struct(type) for type in case['update']]
    assert str(root.update(values, *case['path'])[0]) == case['output']


def test_update_without_path_returns_the_values() -> None:
    root = parse('a b\n', 'test')
    values = (Tree.struct('x'), Tree.struct('y'))
    assert root.update(values) == values


def test_update_leaves_the_source_tree_alone() -> None:
    root = parse('a b c\n', 'test')
    before = str(root)

    root.update([Tree.struct('z')], 'a', 'b')

    assert str(root) == before


def test_update_shares_untouched_kids() -> None:
    root = parse('r\n\ta x\n\tb y\n', 'test')
    changed = root.update([Tree.struct('z')], None, 'a')[0]

    assert changed.kids[0].kids[1] is root.kids[0].kids[1]


def test_deleting_the_root_is_reported() -> None:
    """`insert(None)` with an empty path asks for the whole tree to be
    replaced by nothing; the reference hands back `undefined` for it."""
    with pytest.raises(TreeError, match='Can not delete the root node'):
        parse('a\n', 'test').insert(None)
