"""parse_errors.json — reason, offending line, coordinates, message."""

from __future__ import annotations

from typing import Any

import pytest
from conftest import actual_span, cases, expected_span

from tree2 import TreeError, TreeSyntaxError, parse


@pytest.mark.parametrize('case', cases('parse_errors'))
def test_parse_error(case: dict[str, Any]) -> None:
    with pytest.raises(TreeSyntaxError) as caught:
        parse(case['input'], case['uri'])

    error = caught.value

    assert error.reason == case['reason']
    assert error.line == case['line']
    assert actual_span(error.span) == expected_span(case['span'])


@pytest.mark.parametrize('case', cases('parse_errors'))
def test_parse_error_message(case: dict[str, Any]) -> None:
    """The rendered message embeds the column, so it can only match the
    reference where UTF-16 columns and code point ones agree. They do for
    every case here — none of them reaches outside the basic plane."""
    assert (case['span']['col'], case['span']['length']) == (
        case['span']['col_cp'],
        case['span']['length_cp'],
    )

    with pytest.raises(TreeSyntaxError) as caught:
        parse(case['input'], case['uri'])

    assert str(caught.value) == case['message']


def test_syntax_error_is_a_syntax_error() -> None:
    with pytest.raises(SyntaxError):
        parse('foo  bar\n')
    with pytest.raises(TreeError):
        parse('foo  bar\n')


def test_message_layout() -> None:
    with pytest.raises(TreeSyntaxError) as caught:
        parse('foo  bar\n', 'test')

    assert str(caught.value).split('\n') == [
        'Wrong nodes separator',
        'test#1:5/1',
        '    !',
        'foo  bar',
    ]


def test_markers_keep_tabs_of_the_line() -> None:
    with pytest.raises(TreeSyntaxError) as caught:
        parse('a\n\tb\tc\n', 'test')

    assert str(caught.value).split('\n')[2] == '\t !'


def test_repr_mentions_the_reason() -> None:
    with pytest.raises(TreeSyntaxError) as caught:
        parse('foo  bar\n', 'test')

    assert repr(caught.value) == "TreeSyntaxError('Wrong nodes separator', 'foo  bar', test#1:5/1)"


@pytest.mark.parametrize('source', ['\t\tfoo\n\tbar', '\t\ta\nb', 'a\n\t\tb'])
def test_a_missing_final_line_feed_does_not_mask_an_indent_error(source: str) -> None:
    """A source that reports an indentation error must report the same one
    without its final line feed — the reference drops the line instead, or
    throws a `RangeError`. Narrower than "a line feed never changes the
    error": adding one is exactly what fixes `Unexpected EOF, LF required`."""
    with pytest.raises(TreeSyntaxError) as bare:
        parse(source, 'test')

    with pytest.raises(TreeSyntaxError) as terminated:
        parse(source + '\n', 'test')

    assert bare.value.reason == terminated.value.reason
    assert bare.value.line == terminated.value.line
    # the spans differ in `source` by the line feed, so compare coordinates
    assert str(bare.value.span) == str(terminated.value.span)
