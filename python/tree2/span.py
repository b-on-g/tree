"""Positions inside a source resource."""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
from typing import ClassVar, TypeVar

from .errors import TreeError

__all__ = ['Span']

E = TypeVar('E', bound=Exception)


@dataclass(frozen=True, slots=True, repr=False)
class Span:
    """A region of a source resource.

    ``col`` and ``length`` are counted in code points, not in UTF-16 code
    units as the TypeScript reference does — a Python string is a sequence of
    code points, so this is both the natural and the free choice.
    """

    uri: str = '?'
    source: str = ''
    row: int = 1
    col: int = 1
    length: int = 0

    #: Span of the beginning of an unnamed resource.
    unknown: ClassVar[Span]

    @staticmethod
    def begin(uri: str, source: str = '') -> Span:
        """Zero-length span at the very beginning of the resource."""
        return Span(uri, source, 1, 1, 0)

    @staticmethod
    def end(uri: str, source: str) -> Span:
        """Zero-length span right past the last character of the resource."""
        return Span(uri, source, 1, len(source) + 1, 0)

    @staticmethod
    def entire(uri: str, source: str) -> Span:
        """Span covering the whole resource."""
        return Span(uri, source, 1, 1, len(source))

    def span(self, row: int, col: int, length: int) -> Span:
        """Another span over the same resource."""
        return Span(self.uri, self.source, row, col, length)

    def after(self, length: int = 0) -> Span:
        """Span of `length` starting right after this one, on the same row."""
        return Span(self.uri, self.source, self.row, self.col + self.length, length)

    def slice(self, begin: int, end: int = -1) -> Span:
        """Sub-span. Negative bounds count from the end of this span."""
        length = self.length

        if begin < 0:
            begin += length
        if end < 0:
            end += length

        if begin < 0 or begin > length:
            raise self.error(f"Begin value '{begin}' out of range", ValueError)
        if end < 0 or end > length:
            raise self.error(f"End value '{end}' out of range", ValueError)
        if end < begin:
            raise self.error(f"End value '{end}' can't be less than begin value", ValueError)

        return self.span(self.row, self.col + begin, end - begin)

    # mypy can not tie a type variable to a default; `cls` is E's only source
    def error(self, message: str, cls: Callable[[str], E] = TreeError) -> E:  # type: ignore[assignment]
        """Makes — but does not raise — an error pointing at this span."""
        return cls(f'{message} ({self})')

    def __str__(self) -> str:
        return f'{self.uri}#{self.row}:{self.col}/{self.length}'

    def __repr__(self) -> str:
        return f'Span({self})'


Span.unknown = Span.begin('?')
