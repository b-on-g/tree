"""The parser: source text in, tree out."""

from __future__ import annotations

from .errors import TreeSyntaxError
from .span import Span
from .tree import Tree

__all__ = ['parse']

_SEPARATORS = ' \t'
_TYPE_STOPPERS = '\\ \t\n'


class _Frame:
    """A node under construction. Kids are appended while scanning, then the
    whole thing is frozen into immutable :class:`Tree` nodes in one pass."""

    __slots__ = ('built', 'kids', 'span', 'type', 'value')

    def __init__(self, type: str, value: str, span: Span) -> None:
        self.type = type
        self.value = value
        self.span = span
        self.kids: list[_Frame] = []
        self.built: Tree


def parse(src: str, uri: str = '?') -> Tree:
    """Parses the tree format. Returns the root list node.

    Raises :class:`~tree2.TreeSyntaxError` on malformed input.
    """

    span = Span.entire(uri, src)
    end = len(src)

    root = _Frame('', '', span)
    stack = [root]

    pos = 0
    row = 0
    min_indent = 0

    while pos < end:

        indent = 0
        line_start = pos
        row += 1

        # read indent
        while pos < end and src[pos] == '\t':
            indent += 1
            pos += 1

        # the first line that produces a node fixes the base indent
        if not root.kids:
            min_indent = indent

        indent -= min_indent

        # invalid tab count
        if indent < 0 or indent >= len(stack):

            wrong = span.span(row, 1, pos - line_start)

            # skip the offending line
            while pos < end and src[pos] != '\n':
                pos += 1

            # unconditionally, last line or not: appending a line feed to a
            # source must not change which error it reports
            reason = 'Too few tabs' if indent < 0 else 'Too many tabs'
            raise TreeSyntaxError(reason, src[line_start:pos], wrong)

        del stack[indent + 1 :]
        parent = stack[indent]

        # struct nodes
        while pos < end and src[pos] != '\\' and src[pos] != '\n':

            # a type can contain neither space nor tab
            error_start = pos
            while pos < end and src[pos] in _SEPARATORS:
                pos += 1

            if pos > error_start:
                line_end = src.find('\n', pos)
                if line_end < 0:
                    line_end = end
                raise TreeSyntaxError(
                    'Wrong nodes separator',
                    src[line_start:line_end],
                    span.span(row, error_start - line_start + 1, pos - error_start),
                )

            type_start = pos
            while pos < end and src[pos] not in _TYPE_STOPPERS:
                pos += 1

            if pos > type_start:
                node = _Frame(
                    src[type_start:pos],
                    '',
                    span.span(row, type_start - line_start + 1, pos - type_start),
                )
                parent.kids.append(node)
                parent = node

            # eat exactly one separating space
            if pos < end and src[pos] == ' ':
                pos += 1

        # data node — runs to the end of the line
        if pos < end and src[pos] == '\\':
            data_start = pos
            while pos < end and src[pos] != '\n':
                pos += 1
            node = _Frame(
                '',
                src[data_start + 1 : pos],
                span.span(row, data_start - line_start + 2, pos - data_start - 1),
            )
            parent.kids.append(node)
            parent = node

        # only a line feed may follow
        if pos == end:
            raise TreeSyntaxError(
                'Unexpected EOF, LF required',
                src[line_start:],
                span.span(row, pos - line_start + 1, 1),
            )

        stack.append(parent)
        pos += 1

    return _freeze(root)


def _freeze(root: _Frame) -> Tree:
    """Turns the mutable frames into immutable nodes, deepest first, without
    recursing — a document nested a few thousand levels deep is still valid."""

    order = [root]
    cursor = 0
    while cursor < len(order):
        order.extend(order[cursor].kids)
        cursor += 1

    for frame in reversed(order):
        frame.built = Tree(
            frame.type,
            frame.value,
            tuple(kid.built for kid in frame.kids),
            frame.span,
        )

    return root.built
