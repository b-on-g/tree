"""Serialization back to the tree format."""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from .tree import Tree

__all__ = ['serialize']


def serialize(tree: Tree) -> str:
    """Renders a node and its kids in the tree format.

    This is SPEC §4's recursive ``dump`` turned inside out: dumping is pure
    pre-order — a node emits its own text and then its kids, with nothing
    trailing — so an explicit stack does the job and deep trees can not
    exhaust the interpreter's.
    """

    out: list[str] = []
    # (node, prefix, lead) — `lead` is emitted before the node, `prefix`
    # indents every kid of it.
    stack = [(tree, '', '')]

    while stack:
        node, prefix, lead = stack.pop()

        if lead:
            out.append(lead)

        while True:
            if node.type:
                if not prefix:
                    prefix = '\t'

                out.append(node.type)

                if len(node.kids) == 1:
                    # a lone kid stays on this line: `a b c`
                    out.append(' ')
                    node = node.kids[0]
                    continue

                out.append('\n')

            elif node.value or prefix:
                out.append('\\' + node.value + '\n')

            break

        kid_prefix = prefix + '\t'
        for kid in reversed(node.kids):
            stack.append((kid, kid_prefix, prefix))

    return ''.join(out)
