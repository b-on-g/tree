# tree2 — specification

Language-neutral specification of the **tree** format and of the `tree2` API,
derived from the reference TypeScript implementation
([`$mol_tree2`](https://github.com/hyoo-ru/mam_mol/tree/master/tree2)).

Every port MUST satisfy this document and MUST pass the shared conformance
corpus in [`fixtures/`](./fixtures). Where a port deviates, the deviation MUST
be listed under [Permitted deviations](#permitted-deviations).

---

## 1. The format

`tree` is a whitespace-significant format for structural data. It has exactly
three lexical ingredients — the tab, the space, and the backslash — and no
escaping rules at all.

```
foo
	bar
	baz \some raw text
	\another raw text
```

### 1.1 Lines

A document is a sequence of lines separated by `\n` (U+000A). **Every line,
including the last, MUST be terminated by `\n`.** A document that ends without
a newline is a syntax error (`Unexpected EOF, LF required`).

`\r` is not special: it is ordinary content. Ports MUST NOT strip it.

### 1.2 Indentation

A line begins with zero or more tabs. Nothing else counts as indentation — a
leading space is an error.

The indentation of the **first line that produces a node** becomes the document's
base indent; it is subtracted from every line. This lets a document be embedded
in indented source without reindenting it. Lines before that first node-producing
line (blank lines) do not fix the base indent, but each one overwrites the
running candidate, so only the last one before the first node matters.

Relative to the base indent:

* indent smaller than 0 → `Too few tabs`
* indent greater than the current nesting depth → `Too many tabs`

### 1.3 Nodes

There are exactly two kinds of node.

A **struct node** carries a `type` and no `value`. Its type is a run of
characters containing no space, tab, backslash or newline. Several struct nodes
may sit on one line separated by a single space, and each is the parent of the
next:

```
a b c        ≡    a
                  	b
                  		c
```

A **data node** carries a `value` and no `type`. It starts at a backslash and
runs to the end of the line; the backslash is not part of the value. Everything
after it is verbatim — spaces, tabs, backslashes:

```
\  two leading spaces and a trailing tab	
```

A data node is always the last node on its line, since it consumes the rest of
it. A line may hold struct nodes, then optionally one data node.

Multi-line text is expressed as consecutive data nodes under a shared parent:

```
text
	\first line
	\second line
```

### 1.4 Separators

Exactly one space separates two nodes on a line. Two spaces, or a tab after the
indentation, is `Wrong nodes separator`. A trailing space at the end of a line
is likewise an error, because it is a separator with nothing after it.

---

## 2. Data model

### 2.1 Node

```
Node {
    type:  string        // non-empty for struct nodes, empty otherwise
    value: string        // non-empty for data nodes, empty otherwise
    kids:  Node[]        // ordered children
    span:  Span          // position in the source
}
```

`type` and `value` are never both non-empty. A node with both empty is a
**list** — an anonymous container. The parse root is always a list.

**Nodes are immutable.** Every operation returns a new node. Ports SHOULD make
this structural: children are shared, not copied deeply. In Rust this means
`Rc<Node>`/`Arc<Node>` children; in Go and Python, sharing the child slice /
tuple where nothing changed.

### 2.2 Span

```
Span {
    uri:    string       // name of the source resource
    source: string       // full source text
    row:    int          // 1-based line number
    col:    int          // 1-based column
    length: int          // length of the marked region
}
```

Constructors:

| | row | col | length |
|---|---|---|---|
| `begin(uri)` | 1 | 1 | 0 |
| `end(uri, source)` | 1 | `len(source) + 1` | 0 |
| `entire(uri, source)` | 1 | 1 | `len(source)` |

Operations:

* `span(row, col, length)` — new span over the same resource.
* `after(length = 0)` — `span(row, col + this.length, length)`.
* `slice(begin, end = -1)` — negative indices count from `this.length`;
  yields `span(row, col + begin, end - begin)`. Out-of-range or `end < begin`
  raises a range error.
* `error(message)` — an error carrying `"{message} ({span})"`.

A span renders as `` `{uri}#{row}:{col}/{length}` ``, e.g. `test#1:5/1`.

### 2.3 Columns

**The reference measures `col` and `length` in UTF-16 code units**, because a JS
string index is a UTF-16 index. `\a😀b` therefore reports `length` 4.

**Ports measure them in Unicode scalar values (code points) instead**, so the
same input reports 3. This is a deliberate, spec'd divergence: UTF-16 offsets
are a JavaScript artifact, and they are neither natural nor cheap in Rust, Go
or Python — while byte offsets would make `row:col` useless to a human reading
an error next to their editor's ruler.

The corpus carries both. Each span in `fixtures/` has:

* `col`, `length` — UTF-16, what the reference produces;
* `col_cp`, `length_cp` — code points, **what a port MUST assert against**.

The two coincide for every input outside the astral planes, which is every
input the fixtures contain except the two explicitly named `astral plane`.

A port MAY additionally expose a byte or UTF-16 offset alongside, but `col`
and `length` are code points.

---

## 3. Parsing

`parse(source, uri = '?') -> Node`

Returns a list node whose span is `Span.entire(uri, source)`.

The reference is a single-pass character scanner with no lookahead and no
regular expressions. Ports SHOULD keep it that way — the delimiters are all
ASCII, so a byte-wise scan is safe even in UTF-8 languages as long as the
column counter only advances on non-continuation bytes.

```
pos       := 0
row       := 0
min_indent := 0
root      := list([], entire(uri, source))
stack     := [ root ]

while pos < len(source):

    indent     := 0
    line_start := pos
    row        := row + 1

    while source[pos] == '\t':
        indent := indent + 1
        pos    := pos + 1

    if root.kids is empty:
        min_indent := indent
    indent := indent - min_indent

    if indent < 0 or indent >= len(stack):
        sp := span(row, 1, pos - line_start)
        skip pos forward to the next '\n' (or end)
        if indent < 0:
            fail SyntaxError("Too few tabs", source[line_start .. pos], sp)
        else:
            fail SyntaxError("Too many tabs", source[line_start .. pos], sp)

    truncate stack to indent + 1 entries
    parent := stack[indent]

    // struct nodes
    while pos < len(source) and source[pos] != '\\' and source[pos] != '\n':

        error_start := pos
        while source[pos] == ' ' or source[pos] == '\t':
            pos := pos + 1

        if pos > error_start:
            line_end := index of '\n' at or after pos, else len(source)
            sp := span(row, error_start - line_start + 1, pos - error_start)
            fail SyntaxError("Wrong nodes separator", source[line_start .. line_end], sp)

        type_start := pos
        while pos < len(source) and source[pos] not in { '\\', ' ', '\t', '\n' }:
            pos := pos + 1

        if pos > type_start:
            node   := Node(source[type_start .. pos], "", [],
                           span(row, type_start - line_start + 1, pos - type_start))
            append node to parent.kids
            parent := node

        if source[pos] == ' ':
            pos := pos + 1

    // data node
    if pos < len(source) and source[pos] == '\\':
        data_start := pos
        while pos < len(source) and source[pos] != '\n':
            pos := pos + 1
        node   := Node("", source[data_start + 1 .. pos], [],
                       span(row, data_start - line_start + 2, pos - data_start - 1))
        append node to parent.kids
        parent := node

    if pos == len(source):
        sp := span(row, pos - line_start + 1, 1)
        fail SyntaxError("Unexpected EOF, LF required", source[line_start .. len(source)], sp)

    push parent onto stack
    pos := pos + 1

return root
```

Notes that are easy to get wrong:

* The `indent >= len(stack)` bound is checked **before** the stack is truncated,
  against the length left by the previous line.
* `Too few tabs` is raised unconditionally. The reference suppresses it when the
  offending line is also the unterminated last line of the document, with bad
  consequences — see [Known reference bugs](#known-reference-bugs). A missing
  final newline is an error in its own right, but it MUST NOT mask an
  indentation error that the same source reports once the newline is there.
  Note this is narrower than "appending a newline never changes the error":
  appending one is exactly what *fixes* `Unexpected EOF, LF required`.
* The separator check runs at the top of every iteration of the struct loop, so
  the *second* of two consecutive spaces is what trips it. This also means a
  leading space (line not starting with a tab) is reported at column 1.
* Exactly one trailing space is consumed after each type. A line ending in a
  space therefore re-enters the loop, finds a separator with no node after it,
  and fails.
* `min_indent` is re-assigned on every line until the root has children, so it
  ends up holding the indentation of the first node-producing line.

### 3.1 Syntax errors

A syntax error carries `reason`, the offending `line`, and a `span`. Its
rendered message is four lines:

```
{reason}
{span}
{indent}{markers}
{line}
```

where `indent` is `line[0 .. span.col-1]` with every non-whitespace character
replaced by a space, and `markers` is `!` repeated `span.length` times. For
`foo  bar\n`:

```
Wrong nodes separator
test#1:5/1
    !
foo  bar
```

Ports MUST reproduce `reason`, `line` and `span` exactly (see
`fixtures/parse_errors.json`). Reproducing the rendered `message` byte-for-byte
is RECOMMENDED but not required, since it embeds the UTF-16 column.

---

## 4. Serialization

`serialize(node) -> string`

```
dump(node, prefix = ""):

    if node.type is non-empty:
        if prefix is empty:
            prefix := "\t"
        emit node.type
        if len(node.kids) == 1:
            emit " "
            dump(node.kids[0], prefix)     // same prefix — stays on this line
            return
        emit "\n"

    else if node.value is non-empty or prefix is non-empty:
        emit "\\" + node.value + "\n"

    for kid in node.kids:
        emit prefix
        dump(kid, prefix + "\t")
```

Consequences:

* A struct with exactly one child collapses onto one line, recursively — this
  is what turns the nested form back into `a b c d`.
* A list node emits nothing of itself, only its children. An empty list
  serializes to the empty string.
* A data node with an empty value at the very root emits nothing, because
  `value` and `prefix` are both empty. One nested anywhere emits `\` + newline.

Serialization is a fixed point: `parse(serialize(parse(s)))` and
`parse(s)` serialize identically for every parseable `s`. The corpus asserts
this for all of `fixtures/serialize.json`.

---

## 5. API

Names below are the canonical concepts; each port uses its own idiomatic
spelling (`from_string` / `FromString` / `parse`, etc.) — see
[Naming](#8-naming-and-packaging).

### 5.1 Factories

| | behaviour |
|---|---|
| `list(kids, span)` | node with empty type and value |
| `data(value, kids, span)` | data node — see splitting below |
| `struct(type, kids, span)` | struct node; validates `type` |

`struct` rejects a type matching `[ \n\t\\]` with `Wrong type {json-quoted type}`
carried on the span.

`data` **splits a multi-line value**: if `value` contains `\n`, the result is a
node with an *empty* value whose leading children are one data node per line,
followed by the `kids` that were passed in. Each line's span is derived by
walking `after(len(line))` from a zero-length span at the parent's position.
A single-line value is stored as-is.

Each factory also has a derived instance form that reuses the receiver's span
(`node.struct(type, kids)` etc.), plus `clone(kids, span = self.span)` which
keeps type and value.

### 5.2 Queries

**`text()`** — `self.value` concatenated with the values of every *data* child,
joined by `\n`. Struct children are skipped entirely (not recursed into).

**`select(...path)`** — walks the tree and returns a list node holding the
matches. Each path step is one of:

* a string — every child whose `type` equals it;
* an integer `i` — the child at index `i` when `0 <= i < len(kids)`, otherwise
  no match (see [Known reference bugs](#known-reference-bugs) for negative `i`);
* null/none — every child.

An empty path yields a list containing the receiver.

**`filter(path, value = none)`** — keeps the children for which
`child.select(...path)` is non-empty; when `value` is given, keeps those where
some matched node's `value` equals it. Returns a clone of the receiver with the
surviving children.

### 5.3 Edits

**`update(values, ...path)` -> Node[]** — replaces whatever sits at `path` with
`values`, returning the new siblings at the receiver's own level. With an empty
path it returns `values` unchanged. Otherwise, by the type of the first step:

* string — children of that type are recursed into; if none matched **and**
  `values` is non-empty, a new struct of that type is created and the remaining
  path applied inside it (see [Known reference bugs](#known-reference-bugs) for
  the empty case);
* integer `i` — the child at `i` (or a fresh empty list if absent) is recursed
  into and spliced back in its place;
* null/none — every child is recursed into; if there are no children, one fresh
  empty list stands in.

**`insert(value | null, ...path)` -> Node** — `update` with a single value, or
with an empty list when `null` is passed, taking the first result. Passing
`null` therefore deletes.

### 5.4 Transformation

**`hack(belt, context)`** — a rewriting pass. `belt` maps a type to a handler
`(input, belt, context) -> Node[]`. `hack` maps every child through
`hack_self`, concatenating the results.

`hack_self` picks `belt[self.type]`, falling back to `belt[""]`, falling back to
the identity `[ self.clone(self.hack(belt, context), context.span) ]`. Errors
raised inside a handler are re-raised with `"\n{self without kids}{self.span}"`
appended to the message.

Ports MAY name this differently or expose it as a visitor; it is the one part
of the API where an idiomatic reshaping is expected. Ports MUST keep the
semantics: unknown types recurse, known types are replaced by the handler's
output *in place* (splicing, not nesting).

**`error(message)`** — an error over `self.span` whose text is
`"{message}\n{self without kids}"`.

---

## 6. JSON mapping

### 6.1 tree → JSON

Driven by the node's type:

| type | result |
|---|---|
| empty | `text()` if every child is a data node; else the single child converted, and an error if there is not exactly one |
| `-` | undefined — the entry is dropped by its container |
| `true` / `false` / `null` | the corresponding literal |
| `*` | object |
| `/` | array |
| anything else | parsed as a number; `NaN` is accepted as the literal type `NaN`; otherwise `Unknown json type` |

For `*`, each child contributes one entry: the key is the child's `type`, or —
when the child is a data node — the text of all its children except the last;
the value is the conversion of its **last** child. Children typed `-` are
skipped, as are entries whose value is undefined.

For `/`, each child is converted in order; `-` children and undefined values are
skipped.

### 6.2 JSON → tree

| input | result |
|---|---|
| boolean, number, null | struct node typed with its literal spelling |
| string | `data(value)` — multi-line strings split per §5.1 |
| array | struct `/` whose children are the converted items |
| object | struct `*` whose children are one node per key |
| byte buffer | data node: uppercase hex bytes, space-separated, 8 per line |
| date | data node holding the ISO-8601 string |

For an object key matching `^[^\n\t\\ ]+$`, the entry is a struct node typed
with the key and holding the converted value as its single child. Otherwise it
is `data(key)` with the converted value as its child. Keys whose value is
undefined are skipped.

Ports in languages without `undefined` MAY omit the `-` / undefined handling
from `from_json`, but MUST keep it in `to_json`, where it is reachable from
input trees. Buffer and date handling is OPTIONAL and SHOULD follow the
language's own conventions.

### 6.3 Round-trip

`to_json(parse(serialize(from_json(x)))) == x` for every JSON value `x` built
from the mappings above. `fixtures/from_json.json` and `fixtures/to_json.json`
pin both directions.

---

## 7. Conformance corpus

`fixtures/*.json` is generated from the reference implementation by
`node tools/build-ref.js && node tools/gen-fixtures.js`. **Never hand-edit it** —
add a case to `tools/gen-fixtures.js` and regenerate, so all ports move together.

| file | asserts |
|---|---|
| `parse.json` | `input` parses to `tree`, spans included |
| `parse_errors.json` | `input` fails with `reason`, `line`, `span` |
| `serialize.json` | `parse(input)` serializes to `output`, and that is a fixed point |
| `serialize_built.json` | trees built through the factories serialize to `output` |
| `text.json` | `text()` of the root and of its first child |
| `select.json` | `select(...path)` serializes to `output` |
| `filter.json` | `filter(path, value)` on the root's first child |
| `insert.json` | `insert(struct(insert), ...path)`; `insert: null` means delete |
| `update.json` | `update([struct(t) for t in update], ...path)[0]` |
| `to_json.json` | `to_json(parse(input))` equals `json` |
| `from_json.json` | `from_json(json)` serializes to `output` |
| `spans.json` | span-focused parse cases |
| `reference_bugs.json` | hand-written; port behaviour where the reference is wrong |

In `select` / `insert` / `update` paths, a JSON `null` is the null path step and
a JSON number is an index step.

A port's test suite MUST read these files from `../fixtures` at test time rather
than vendoring a copy.

---

## 8. Naming and packaging

The reference is written in `$mol`'s flat global namespace. **Ports must not
carry that over.** Each language gets one module/crate/package named `tree2`
exposing a `Tree`/`Node` type and free functions, spelled the way that language
spells things.

| | crate/module | parse | serialize |
|---|---|---|---|
| Rust | `tree2` | `tree2::parse` | `Display` for `Tree` |
| Go | `github.com/b-on-g/tree2/go` → package `tree2` | `tree2.Parse` | `String()` |
| Python | `tree2` | `tree2.parse` | `str(tree)` |

Errors follow the host language: `Result<_, SyntaxError>` in Rust, an `error`
return in Go, an exception in Python. A port MUST NOT panic/abort on malformed
input.

---

## Known reference bugs

Three places where a port cannot simply follow the reference.
`fixtures/reference_bugs.json` pins the required port behaviour, and also pins
the neighbouring cases where the reference is right, so that a port does not
over-correct.

### A missing final newline can swallow a line, or crash

`Too few tabs` is raised only `if( str.length > pos )` — that is, only when the
offending line is not the last one in an unterminated document. The intent was
presumably to be lenient about a trailing fragment. The effect is that the check
falls through to `stack.length = indent + 1` with a negative `indent`:

* `indent == -1` sets the stack length to 0, which then makes the later
  `stack.length > 0` guard on the EOF check false, so **no error is raised at
  all** and the line is silently dropped;
* `indent <= -2` sets it to -1, which is `RangeError: Invalid array length`.

Deleting the trailing newline from a file therefore turns a clean syntax error
into either lost content or an unrelated crash:

| input | reference |
|---|---|
| `\t\tfoo\n\tbar\n` | `Too few tabs` |
| `\t\tfoo\n\tbar` | returns `foo\n` — `bar` is gone, no error |
| `\t\ta\nb\n` | `Too few tabs` |
| `\t\ta\nb` | `RangeError: Invalid array length` |

**Ports:** raise `Too few tabs` unconditionally, so that a missing final newline
cannot mask an indentation error. This is the one bug of the three that loses
data.

### An out-of-range index step in `select` yields a hole

The bound check is `i < len(kids)` with no lower bound. A negative `i` therefore
passes it, JavaScript hands back `undefined` rather than failing, and the result
is a list node containing a hole that crashes the moment anything reads it — far
from where the index came from, with a message naming neither the path nor the
node:

```js
const kids = tree.select( 'app', null ).kids
tree.select( 'app', kids.length - 1 )   // kids is empty → index -1
// TypeError: Cannot read properties of undefined (reading 'type')
```

Nobody writes `-1` by hand; it arrives from `length - 1` on an empty list, or
from `findIndex`/`indexOf` missing. This one is mostly a robustness defect —
but a port has to define the behaviour regardless, because a negative index is
not even representable against a `Vec`/slice without a decision.

**Ports:** treat any `i` outside `0 <= i < len(kids)` as no match.

### Deleting an absent path creates it

`insert(value, ...path)` is `update(maybe(value), ...path)`, and `maybe(null)`
is `[]` — so **deleting is updating to an empty list**. But `update` guards its
create-missing branch with `if( !replaced && value )`, and `[]` is truthy in
JavaScript. Removing a setting that is not there therefore adds it:

```js
const config = $mol_tree2_from_string( 'config\n\tport \\8080\n', 't' )
config.insert( null, 'config', 'theme', 'dark' ).toString()
// 'config\n\tport \\8080\n\ttheme\n'   — expected 'config port \\8080\n'
```

The node is created only when at least one more step follows it, because the
final step is *replaced* by the value rather than created, and an empty value
list replaces it with nothing. So a one-step tail is a no-op and the bug starts
at two:

| path missing from `a b` | reference | expected |
|---|---|---|
| `a, z` | `a b` ✅ | `a b` |
| `a, z, q` | `a` + `b` + `z` ❌ | `a b` |
| `a, z, q, w` | `a` + `b` + `z q` ❌ | `a b` |

The reference's own tests miss this because they only ever delete paths that
already exist.

**Ports:** take the create-missing branch only when `values` is non-empty. Do
not go further than that — a non-empty `update` MUST still create the missing
path, last step included:

```js
$mol_tree2_from_string( 'a b\n', 't' ).update( [ struct('x') ], 'a', 'z', 'q' )[0]
// 'a\n\tb\n\tz x\n'   — `z` created, `q` replaced by `x`
```

---

## Permitted deviations

1. **Columns are code points, not UTF-16 code units** (§2.3). Applies to all
   ports.
2. `from_json` MAY omit `undefined`, buffer and date handling where the language
   has no such concept (§6.2).
3. `hack` MAY be reshaped into an idiomatic visitor, keeping its semantics
   (§5.4).
4. Deprecated reference members (`Tree.fromString`) are not ported.

Anything else is a bug in the port.
