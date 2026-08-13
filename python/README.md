# tree2

Parser, serializer and AST for the **tree** format — a whitespace-significant
structural format with three lexical ingredients (tab, space, backslash) and no
escaping rules at all.

```
server
	host \0.0.0.0
	port \8080
	greeting
		\Hello,
		\world!
```

This is the Python port of [`$mol_tree2`](https://github.com/hyoo-ru/mam_mol/tree/master/tree2).
The format and the API are pinned by [`SPEC.md`](../SPEC.md); the test suite
runs the shared conformance corpus in [`fixtures/`](../fixtures) straight from
the repository.

No runtime dependencies. Python 3.10 and up.

## Install

```sh
pip install -e '.[dev]'   # from this directory
```

## Use

```python
from tree2 import Tree, parse, from_json, to_json

config = parse(open('server.tree').read(), 'server.tree')

config.select('server', 'port', None).text()   # '8080'
config.select('server', None)                  # list node of every setting

str(config.insert(Tree.data('8081'), 'server', 'port', 0))
# 'server\n\thost \\0.0.0.0\n\tport \\8081\n\t...'
```

Everything is immutable: an edit returns a new tree and shares the kids it did
not touch.

```python
>>> tree = parse('foo bar \\some text\n', 'test')
>>> tree.kids[0].type
'foo'
>>> tree.kids[0].kids[0].kids[0].value
'some text'
>>> str(tree.kids[0].span)
'test#1:1/3'
```

### Building

```python
Tree.struct('user', [Tree.data('Jin')])   # user \Jin
Tree.data('two\nlines')                   # a node whose kids are the lines
Tree.list([Tree.struct('a'), Tree.struct('b')])
```

`node.make_struct(...)`, `node.make_data(...)`, `node.make_list(...)` are the
same factories with `node`'s own span, and `node.clone(kids)` keeps the type
and the value.

### Querying and editing

| | |
|---|---|
| `node.text()` | own value plus the values of the data kids, joined by `\n` |
| `node.select(*path)` | list node of everything at `path` |
| `node.filter(path, value=None)` | kids that have something at `path` |
| `node.insert(value, *path)` | new tree with `value` at `path`; `None` deletes |
| `node.update(values, *path)` | same, with any number of nodes |
| `node.hack(belt, context)` | rewriting pass, one handler per type |
| `node.error(message)` | an error pointing at this node |

A path step is a `str` (kids of that type), an `int` (the kid at that index) or
`None` (every kid).

### JSON

```python
>>> str(from_json({'port': 8080, 'hosts': ['a', 'b']}))
'*\n\tport 8080\n\thosts /\n\t\t\\a\n\t\t\\b\n'
>>> to_json(parse('* port 8080\n', 'test'))
{'port': 8080}
```

`bytes` becomes a hex dump and `datetime` an ISO-8601 string. A `-` node is a
comment: it converts to `tree2.UNDEFINED`, and objects and arrays drop it.

### Errors

`parse` raises `TreeSyntaxError`, which is both a `SyntaxError` and a
`TreeError`, and carries `reason`, `line` and `span`:

```
>>> parse('foo  bar\n', 'test')
tree2.errors.TreeSyntaxError: Wrong nodes separator
test#1:5/1
    !
foo  bar
```

Everything else this package raises on its own is a `TreeError`, except
out-of-range span slicing, which is a `ValueError`.

## Positions

A `Span` is `uri`, `source`, `row`, `col` and `length`, rendered as
`uri#row:col/length`. **Columns are counted in code points**, where the
TypeScript reference counts UTF-16 code units — the two differ only for astral
characters, and code points are what a Python string is made of. This is a
spec'd divergence, see SPEC.md §2.3.

## Development

```sh
ruff check .
mypy tree2
pytest
```
