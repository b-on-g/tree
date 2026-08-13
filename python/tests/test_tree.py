"""Factories, immutability, hack and node errors."""

from __future__ import annotations

from collections.abc import Sequence

import pytest

from tree2 import Belt, Context, Span, Tree, TreeError, parse


def test_defaults_make_an_empty_list() -> None:
    node = Tree()

    assert (node.type, node.value, node.kids) == ('', '', ())
    assert node.span is Span.unknown
    assert str(node) == ''


def test_kids_are_stored_as_a_tuple() -> None:
    node = Tree('a', '', [Tree.struct('b')], Span.unknown)

    assert isinstance(node.kids, tuple)


def test_is_immutable() -> None:
    node = Tree.struct('a')

    with pytest.raises(AttributeError):
        node.type = 'b'  # type: ignore[misc]


def test_is_hashable() -> None:
    assert {Tree.struct('a'), Tree.struct('a')} == {Tree.struct('a')}


def test_struct_rejects_a_type_it_can_not_spell() -> None:
    for wrong in ('a b', 'a\tb', 'a\nb', 'a\\b'):
        with pytest.raises(TreeError, match='Wrong type'):
            Tree.struct(wrong)


def test_struct_error_quotes_the_type_and_the_span() -> None:
    span = Span.entire('test', 'x')

    with pytest.raises(TreeError, match=r'Wrong type "a b" \(test#1:1/1\)'):
        Tree.struct('a b', (), span)


def test_data_keeps_a_single_line_as_a_value() -> None:
    node = Tree.data('hello')

    assert (node.value, node.kids) == ('hello', ())


def test_data_splits_a_multiline_value() -> None:
    node = Tree.data('a\nbb', [Tree.struct('x')])

    assert node.value == ''
    assert [kid.value for kid in node.kids[:2]] == ['a', 'bb']
    assert node.kids[2].type == 'x'


def test_data_spans_walk_the_lines() -> None:
    node = Tree.data('a\nbb\n', (), Span.entire('test', 'source'))

    assert [str(kid.span) for kid in node.kids] == ['test#1:1/1', 'test#1:2/2', 'test#1:4/0']


def test_derived_factories_reuse_the_span() -> None:
    origin = parse('a\n', 'test').kids[0]

    assert origin.make_struct('b').span is origin.span
    assert origin.make_data('x').span is origin.span
    assert origin.make_list().span is origin.span


def test_clone_keeps_type_and_value() -> None:
    node = Tree.struct('a', [Tree.struct('b')])
    other = node.clone([Tree.struct('c')])

    assert str(other) == 'a c\n'
    assert str(node) == 'a b\n'


def test_clone_can_move_the_span() -> None:
    span = Span.entire('other', 'x')

    assert Tree.struct('a').clone((), span).span is span


def test_hack_replaces_in_place() -> None:
    def to_777(input: Tree, belt: Belt, context: Context) -> Sequence[Tree]:
        return [input.make_struct('777', input.hack(belt, context))]

    result = parse('foo bar xxx\n', 'test').hack({'bar': to_777})

    assert [str(node) for node in result] == ['foo 777 xxx\n']


def test_hack_falls_back_to_the_empty_type() -> None:
    def drop(input: Tree, belt: Belt, context: Context) -> Sequence[Tree]:
        return []

    assert parse('a\nb\n', 'test').hack({'': drop}) == ()


def test_hack_keeps_unknown_types_and_recurses() -> None:
    def double(input: Tree, belt: Belt, context: Context) -> Sequence[Tree]:
        return [input, input]

    result = parse('a\n\tb\n', 'test').hack({'b': double})

    assert [str(node) for node in result] == ['a\n\tb\n\tb\n']


def test_hack_takes_the_span_from_the_context() -> None:
    span = Span.entire('other', 'x')
    result = parse('a\n', 'test').hack({}, {'span': span})

    assert result[0].span is span


def test_hack_explains_where_a_handler_failed() -> None:
    def fail(input: Tree, belt: Belt, context: Context) -> Sequence[Tree]:
        raise ValueError('Nope')

    with pytest.raises(ValueError) as caught:
        parse('a b\n', 'test').hack({'b': fail})

    assert str(caught.value) == 'Nope\nb\ntest#1:3/1\na\ntest#1:1/1'


def test_error_carries_the_node_and_the_span() -> None:
    node = parse('a b\n', 'test').kids[0]
    error = node.error('Boom')

    assert isinstance(error, TreeError)
    assert str(error) == 'Boom\na\n (test#1:1/1)'


def test_error_of_a_chosen_class() -> None:
    assert isinstance(Tree.struct('a').error('Boom', KeyError), KeyError)


def test_text_of_a_data_node_with_kids() -> None:
    node = Tree.data('a\nb')

    assert node.text() == 'a\nb'
