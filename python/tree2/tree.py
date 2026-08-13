"""The node type: an immutable AST with human readable serialization."""

from __future__ import annotations

import json
import re
from collections.abc import Callable, Iterable, Mapping, Sequence
from dataclasses import dataclass
from types import MappingProxyType
from typing import Any, TypeVar

from .errors import TreeError, TreeSyntaxError
from .serializer import serialize
from .span import Span

__all__ = ['Belt', 'Context', 'Hacker', 'PathStep', 'Tree']

E = TypeVar('E', bound=Exception)

#: One step of a path: a type to match, a kid index, or `None` for every kid.
PathStep = str | int | None

#: Arbitrary state threaded through a `hack` pass. A `span` entry, when
#: present, replaces the span of every node the default handler rebuilds.
Context = Mapping[str, Any]

#: Rewrites one node into any number of nodes, spliced in its place.
Hacker = Callable[['Tree', 'Belt', Context], Sequence['Tree']]

#: Maps a node type to its rewriter. The `''` entry catches everything else.
Belt = Mapping[str, Hacker]

EMPTY_CONTEXT: Context = MappingProxyType({})

_WRONG_TYPE = re.compile(r'[ \n\t\\]')


@dataclass(frozen=True, slots=True, repr=False)
class Tree:
    """A node of the tree.

    A node carries either a `type` (struct node) or a `value` (data node), or
    neither (list node — an anonymous container). Nodes are immutable: every
    operation returns a new node and shares the untouched kids.

    Prefer the factories — :meth:`list`, :meth:`data`, :meth:`struct` — over
    the constructor: `data` splits multi-line values and `struct` validates
    the type.
    """

    type: str = ''
    value: str = ''
    kids: tuple[Tree, ...] = ()
    span: Span = Span.unknown

    def __post_init__(self) -> None:
        # any iterable of kids is welcome, but only a tuple is stored
        object.__setattr__(self, 'kids', tuple(self.kids))

    # ------------------------------------------------------------ factories

    @staticmethod
    def list(kids: Iterable[Tree] = (), span: Span | None = None) -> Tree:
        """Makes a collection node — no type, no value."""
        return Tree('', '', tuple(kids), Span.unknown if span is None else span)

    @staticmethod
    def data(value: str, kids: Iterable[Tree] = (), span: Span | None = None) -> Tree:
        """Makes a data node for any string.

        A multi-line value becomes one data kid per line, in front of `kids`,
        and the node's own value is emptied — that is how the format spells
        multi-line text.
        """
        span = Span.unknown if span is None else span
        chunks = value.split('\n')

        if len(chunks) > 1:
            return Tree('', '', (*_split(chunks, span), *kids), span)

        return Tree('', value, tuple(kids), span)

    @staticmethod
    def struct(type: str, kids: Iterable[Tree] = (), span: Span | None = None) -> Tree:
        """Makes a structural node, rejecting a type the format can not spell."""
        span = Span.unknown if span is None else span

        if _WRONG_TYPE.search(type):
            raise span.error(f'Wrong type {json.dumps(type, ensure_ascii=False)}')

        return Tree(type, '', tuple(kids), span)

    def make_list(self, kids: Iterable[Tree] = ()) -> Tree:
        """Makes a list node at this node's position."""
        return Tree.list(kids, self.span)

    def make_data(self, value: str, kids: Iterable[Tree] = ()) -> Tree:
        """Makes a data node at this node's position."""
        return Tree.data(value, kids, self.span)

    def make_struct(self, type: str, kids: Iterable[Tree] = ()) -> Tree:
        """Makes a struct node at this node's position."""
        return Tree.struct(type, kids, self.span)

    def clone(self, kids: Iterable[Tree], span: Span | None = None) -> Tree:
        """Copies this node, keeping type and value, with other kids."""
        return Tree(self.type, self.value, tuple(kids), self.span if span is None else span)

    # -------------------------------------------------------------- queries

    def text(self) -> str:
        """Multi-line text content: own value plus the values of the data
        kids, joined by line feeds. Struct kids are skipped entirely."""
        return self.value + '\n'.join(kid.value for kid in self.kids if not kid.type)

    def select(self, *path: PathStep) -> Tree:
        """Collects the nodes at `path` into a list node.

        A string step matches kids by type, an integer step picks the kid at
        that index, `None` takes every kid. An empty path yields a list
        holding this node.
        """
        found = [self]

        for step in path:

            if not found:
                break

            previous, found = found, []

            for item in previous:
                if isinstance(step, str):
                    found.extend(kid for kid in item.kids if kid.type == step)
                elif isinstance(step, int):
                    if 0 <= step < len(item.kids):
                        found.append(item.kids[step])
                else:
                    found.extend(item.kids)

        return self.make_list(found)

    def filter(self, path: Sequence[PathStep], value: str | None = None) -> Tree:
        """Keeps the kids that have something at `path` — and, when `value` is
        given, those where some matched node carries exactly that value."""
        kept = []

        for item in self.kids:
            found = item.select(*path)
            if value is None:
                if found.kids:
                    kept.append(item)
            elif any(kid.value == value for kid in found.kids):
                kept.append(item)

        return self.clone(kept)

    # ---------------------------------------------------------------- edits

    def insert(self, value: Tree | None, *path: PathStep) -> Tree:
        """Replaces whatever sits at `path` with `value`. `None` deletes it."""
        result = self.update(() if value is None else (value,), *path)

        if not result:
            # deleting with an empty path — there is no node left to return
            raise self.error('Can not delete the root node')

        return result[0]

    def update(self, values: Sequence[Tree], *path: PathStep) -> tuple[Tree, ...]:
        """Replaces whatever sits at `path` with `values`, returning the new
        siblings at this node's own level.

        A missing string step is created on the way, unless `values` is empty
        — deleting a path that is not there leaves the tree alone.
        """
        if not path:
            return tuple(values)

        step, rest = path[0], path[1:]

        if isinstance(step, str):

            replaced = False
            kids = []

            for item in self.kids:
                if item.type != step:
                    kids.append(item)
                else:
                    replaced = True
                    kids.extend(item.update(values, *rest))

            if not replaced and values:
                kids.extend(self.make_struct(step).update(values, *rest))

            return (self.clone(kids),)

        if isinstance(step, int):

            if 0 <= step < len(self.kids):
                spliced = self.kids[step].update(values, *rest)
                return (self.clone((*self.kids[:step], *spliced, *self.kids[step + 1 :])),)

            # no such kid — a fresh list stands in for it, at the near end
            spliced = self.make_list().update(values, *rest)
            at = len(self.kids) if step > 0 else 0
            return (self.clone((*self.kids[:at], *spliced, *self.kids[at:])),)

        return (
            self.clone(
                kid
                for item in self.kids or (self.make_list(),)
                for kid in item.update(values, *rest)
            ),
        )

    # ------------------------------------------------------- transformation

    def hack(self, belt: Belt, context: Context = EMPTY_CONTEXT) -> tuple[Tree, ...]:
        """Rewrites every kid through `belt`, splicing the results together."""
        return tuple(kid for item in self.kids for kid in item.hack_self(belt, context))

    def hack_self(self, belt: Belt, context: Context = EMPTY_CONTEXT) -> tuple[Tree, ...]:
        """Rewrites this node through `belt`.

        The handler for this node's type wins, then the `''` handler, then the
        identity: keep the node and hack its kids.
        """
        handle = belt.get(self.type) or belt.get('')

        try:
            if handle is None:
                return (self.clone(self.hack(belt, context), context.get('span', self.span)),)
            return tuple(handle(self, belt, context))
        except Exception as error:
            # every node on the way out adds itself, so the message ends up
            # holding the path from the failure to the root
            _explain(error, f'\n{self.clone(())}{self.span}')
            raise

    # --------------------------------------------------------------- errors

    # mypy can not tie a type variable to a default; `cls` is E's only source
    def error(self, message: str, cls: Callable[[str], E] = TreeError) -> E:  # type: ignore[assignment]
        """Makes — but does not raise — an error pointing at this node."""
        return self.span.error(f'{message}\n{self.clone(())}', cls)

    # ---------------------------------------------------------------- dunder

    def __str__(self) -> str:
        return serialize(self)

    def __repr__(self) -> str:
        return f'Tree(type={self.type!r}, value={self.value!r}, kids={len(self.kids)}, {self.span})'


def _split(chunks: Sequence[str], span: Span) -> list[Tree]:
    """One data node per line, each span picking up where the previous ended."""
    kid_span = span.span(span.row, span.col, 0)
    kids = []

    for chunk in chunks:
        kid_span = kid_span.after(len(chunk))
        kids.append(Tree('', chunk, (), kid_span))

    return kids


def _explain(error: BaseException, suffix: str) -> None:
    """Appends the offending node and its position to an error's message."""
    if isinstance(error, TreeSyntaxError):
        error.message += suffix
        return

    args = error.args

    if args and isinstance(args[0], str):
        error.args = (args[0] + suffix, *args[1:])
        if isinstance(error, SyntaxError):
            error.msg = error.args[0]
    else:
        error.args = (*args, suffix)
