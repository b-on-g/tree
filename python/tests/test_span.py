"""Span arithmetic."""

from __future__ import annotations

import pytest

from tree2 import Span, TreeError


def test_constructors() -> None:
    assert str(Span.begin('test')) == 'test#1:1/0'
    assert str(Span.end('test', 'hello')) == 'test#1:6/0'
    assert str(Span.entire('test', 'hello')) == 'test#1:1/5'
    assert str(Span.unknown) == '?#1:1/0'


def test_constructors_count_code_points() -> None:
    assert str(Span.entire('test', 'a😀b')) == 'test#1:1/3'
    assert str(Span.end('test', 'a😀b')) == 'test#1:4/0'


def test_span_keeps_the_resource() -> None:
    origin = Span.entire('test', 'hello')
    moved = origin.span(3, 2, 1)

    assert (moved.uri, moved.source) == ('test', 'hello')
    assert str(moved) == 'test#3:2/1'


def test_after() -> None:
    assert str(Span.entire('test', 'hello').after(2)) == 'test#1:6/2'
    assert str(Span.entire('test', 'hello').after()) == 'test#1:6/0'


def test_slice() -> None:
    entire = Span.entire('test', 'hello')

    assert str(entire.slice(1, 3)) == 'test#1:2/2'
    assert str(entire.slice(0, 0)) == 'test#1:1/0'
    assert str(entire.slice(0, 5)) == 'test#1:1/5'

    # both bounds count from the end when negative — and `end` defaults to -1,
    # so the default slice stops one short of the end, as in the reference
    assert str(entire.slice(1)) == 'test#1:2/3'
    assert str(entire.slice(-2)) == 'test#1:4/1'


@pytest.mark.parametrize(
    ('begin', 'end', 'reason'),
    [
        (9, -1, "Begin value '9' out of range"),
        (-9, -1, "Begin value '-4' out of range"),
        (0, 9, "End value '9' out of range"),
        (3, 1, "End value '1' can't be less than begin value"),
    ],
)
def test_slice_out_of_range(begin: int, end: int, reason: str) -> None:
    with pytest.raises(ValueError, match=f'{reason} \\(test#1:1/5\\)'):
        Span.entire('test', 'hello').slice(begin, end)


def test_error() -> None:
    error = Span.entire('test', 'hello').error('Something')

    assert isinstance(error, TreeError)
    assert str(error) == 'Something (test#1:1/5)'


def test_error_of_a_chosen_class() -> None:
    error = Span.begin('test').error('Nope', KeyError)

    assert isinstance(error, KeyError)


def test_is_immutable_and_hashable() -> None:
    span = Span.begin('test')

    with pytest.raises(AttributeError):
        span.row = 2  # type: ignore[misc]

    assert {span, Span.begin('test')} == {span}


def test_repr() -> None:
    assert repr(Span.begin('test')) == 'Span(test#1:1/0)'
