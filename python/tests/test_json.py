"""to_json.json and from_json.json, plus the round trip between them."""

from __future__ import annotations

import json
import math
from datetime import datetime, timezone
from typing import Any

import pytest
from conftest import cases

from tree2 import UNDEFINED, Tree, TreeError, from_json, parse, to_json


def spelled(value: Any) -> str:
    """JSON text, so that `1` and `1.0` and `true` stay apart."""
    return json.dumps(value, ensure_ascii=False)


@pytest.mark.parametrize('case', cases('to_json'))
def test_to_json(case: dict[str, Any]) -> None:
    assert spelled(to_json(parse(case['input'], 'test'))) == spelled(case['json'])


@pytest.mark.parametrize('case', cases('from_json'))
def test_from_json(case: dict[str, Any]) -> None:
    assert str(from_json(case['json'])) == case['output']


@pytest.mark.parametrize('case', cases('from_json'))
def test_json_round_trip(case: dict[str, Any]) -> None:
    tree = parse(str(from_json(case['json'])), 'test')
    assert spelled(to_json(tree)) == spelled(case['json'])


def test_comment_node_is_undefined() -> None:
    assert to_json(parse('- \\x\n', 'test').kids[0]) is UNDEFINED


def test_nan_is_a_number() -> None:
    value = to_json(parse('NaN\n', 'test'))
    assert isinstance(value, float)
    assert math.isnan(value)


def test_unknown_type_is_reported() -> None:
    with pytest.raises(TreeError, match=r'Unknown json type \(nope\) at test#1:1/4'):
        to_json(parse('nope\n', 'test'))


def test_several_roots_are_reported() -> None:
    with pytest.raises(TreeError, match='Multiple json root'):
        to_json(parse('a\nb\n', 'test'))


def test_missing_value_is_reported() -> None:
    with pytest.raises(TreeError, match='Missing json value'):
        to_json(parse('* a\n', 'test'))


def test_object_keeps_key_order() -> None:
    assert list(to_json(parse('* \n\tb \\1\n\ta \\2\n', 'test'))) == ['b', 'a']


def test_from_json_of_bytes() -> None:
    tree = from_json(bytes(range(10)))
    assert str(tree) == '\\00 01 02 03 04 05 06 07\n\\08 09\n'


def test_from_json_of_datetime() -> None:
    tree = from_json(datetime(2020, 1, 2, 3, 4, 5, tzinfo=timezone.utc))
    assert str(tree) == '\\2020-01-02T03:04:05+00:00\n'


def test_from_json_drops_undefined() -> None:
    assert str(from_json({'a': 1, 'b': UNDEFINED})) == '* a 1\n'
    assert str(from_json([1, UNDEFINED])) == '/ 1\n'


def test_from_json_of_a_stranger() -> None:
    with pytest.raises(TypeError, match='Can not convert object to a tree'):
        from_json(object())


def test_from_json_reuses_the_span() -> None:
    span = parse('a\n', 'test').span
    assert from_json({'a': [1]}, span).kids[0].span is span


def test_to_json_of_a_built_tree() -> None:
    tree = Tree.struct('*', [Tree.struct('a', [Tree.struct('1')])])
    assert to_json(tree) == {'a': 1}
