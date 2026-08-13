"""Errors raised by :mod:`tree2`."""

from __future__ import annotations

import re
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from .span import Span

__all__ = ['TreeError', 'TreeSyntaxError']

_NON_SPACE = re.compile(r'\S')


class TreeError(Exception):
    """Base class for every error this package raises on its own."""


class TreeSyntaxError(TreeError, SyntaxError):
    """Malformed source, with the offending line and its coordinates.

    ``str()`` renders four lines: the reason, the span, a row of ``!`` markers
    under the offending region, and the line itself.
    """

    def __init__(self, reason: str, line: str, span: Span) -> None:
        self.reason = reason
        self.line = line
        self.span = span
        self.message = _render(reason, line, span)
        super().__init__(self.message)

    def __str__(self) -> str:
        return self.message

    def __repr__(self) -> str:
        return f'{type(self).__name__}({self.reason!r}, {self.line!r}, {self.span})'


def _render(reason: str, line: str, span: Span) -> str:
    """Builds the four-line message. Whitespace before the span is kept, so
    the markers stay aligned when the line is indented with tabs."""
    indent = _NON_SPACE.sub(' ', line[: span.col - 1])
    markers = '!' * span.length
    return f'{reason}\n{span}\n{indent}{markers}\n{line}'
