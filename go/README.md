# tree2 for Go

A Go port of [`$mol_tree2`](https://github.com/hyoo-ru/mam_mol/tree/master/tree2) —
parser, serializer and AST for the **tree** format. The contract it implements
is [`SPEC.md`](../SPEC.md) in the repository root; the shared conformance
corpus is [`fixtures/`](../fixtures), which this package's tests read directly.

```sh
go get github.com/b-on-g/tree2/go
```

```go
import tree2 "github.com/b-on-g/tree2/go"
```

## Parsing and serializing

```go
source := "server\n\thost \\0.0.0.0\n\tport \\8080\n"

tree, err := tree2.Parse(source, "config.tree")
if err != nil {
	log.Fatal(err) // *tree2.SyntaxError, with reason, line and span
}

host := tree.Select(tree2.ByType("server"), tree2.ByType("host")).Kid(0)
fmt.Println(host.Text())        // 0.0.0.0
fmt.Println(host.Span())        // config.tree#2:2/4

fmt.Print(tree)                 // serializes back, byte for byte
```

Every line, including the last, must end with a newline — a document that
does not is a syntax error. A `*SyntaxError` renders as four lines:

```
Wrong nodes separator
config.tree#1:5/1
    !
foo  bar
```

## Building

Trees are immutable. `List`, `Data` and `Struct` build nodes from scratch; the
same names on a node build one at that node's position, and `Clone` keeps its
type and value.

```go
span := tree2.SpanUnknown

port, err := tree2.Struct("port", []*tree2.Tree{
	tree2.Data("8080", nil, span),
}, span)
if err != nil {
	log.Fatal(err) // the type carried a space, tab, newline or backslash
}

fmt.Print(tree2.List([]*tree2.Tree{port}, span)) // port \8080
```

`Data` splits a multi-line value into one data child per line, which is how the
format carries text:

```go
fmt.Print(tree2.Data("Hello,\nworld!", nil, span))
// \Hello,
// \world!
```

## Walking and editing

A path is a sequence of steps: `ByType` matches children of a type, `ByIndex`
the child at an index, `Any` every child. The zero `Step` is `Any`.

```go
tree.Select(tree2.ByType("server"), tree2.Any())   // list of every setting
tree.Filter(tree2.ByType("host"))                  // kids that have a host
tree.FilterValue("8080", tree2.ByType("port"), tree2.Any())
```

`Update` replaces whatever sits at a path and returns the new siblings at the
receiver's level; `Insert` is the single-node form, and inserting `nil`
deletes. Both leave the original tree untouched and share every subtree they
did not have to rebuild.

```go
updated, err := tree.Insert(
	tree2.Data("9090", nil, span),
	tree2.ByType("server"), tree2.ByType("port"), tree2.ByIndex(0),
)

deleted, err := tree.Insert(nil, tree2.ByType("server"), tree2.ByType("host"))
```

A missing path is created when there is something to put there — a type step
makes a struct node, an index or wildcard step an empty list. Updating a
missing path to nothing leaves the tree alone.

## JSON

```go
value, err := tree.ToJSON()                       // nil, bool, float64, string, []any, map[string]any
back, err := tree2.FromJSON(value, tree2.SpanUnknown)
```

`FromJSON` also takes Go integers, `[]byte` (uppercase hex, eight bytes to a
line), `time.Time` and anything else that `encoding/json` can marshal, which
covers structs and named types.

Map keys are visited in sorted order, since a Go map has none of its own. An
object that is single-key or already alphabetical therefore round-trips
unchanged; any other comes back sorted.

## Rewriting

`Hack` runs a rewriting pass: a `Belt` maps a node type to a handler whose
output is spliced in place of the input. Types without a handler keep their
node and have their children rewritten.

```go
belt := tree2.Belt{
	// unwrap every group, leaving its children behind
	"group": func(input *tree2.Tree, belt tree2.Belt, ctx *tree2.HackContext) ([]*tree2.Tree, error) {
		return input.Hack(belt, ctx)
	},
}

nodes, err := tree.Hack(belt, nil)
```

Errors coming out of a handler are wrapped with the node they happened on and
its position, so a failed pass reports where in the source it gave up.

## Notes on this port

* **Columns are Unicode code points.** The reference counts UTF-16 code units,
  which is a JavaScript artifact; see SPEC.md §2.3. The two agree outside the
  astral planes. Internally the parser scans bytes and only counts
  non-continuation ones, so the cost is a counter, not a decoder.
* **A path step is a `Step` value, not an `any`.** `ByType`, `ByIndex` and
  `Any` are total: no step can be malformed, so no query can fail at run time
  over its path.
* **`Update` and `Insert` return an error**, because a type step over a missing
  path has to build a struct node, and a type carrying a space cannot be
  serialized. Nothing else in them fails.
* **An index step outside the children matches nothing** — the reference reads
  past the start of its array instead and hands back a hole (SPEC.md, Known
  reference bugs). Where `Update` has to splice such a step in anyway, it
  splices at the nearest valid position: the front for a negative index, the
  back for one past the end.
* **`Kids` returns a copy.** `Kid` and `KidCount` walk the children without
  allocating; the factories copy the slices handed to them. Nothing a caller
  holds ever aliases a tree.
* **`Too few tabs` does not care whether the source ends with a newline.** The
  reference raises it only when the offending line is not the unterminated last
  one, and then either drops that line silently or crashes with
  `RangeError: Invalid array length` (SPEC.md, Known reference bugs). Here
  `"\t\tfoo\n\tbar"` and `"\t\tfoo\n\tbar\n"` report the same error at the same
  position.
* **Parse never panics.** Malformed input, invalid UTF-8 included, comes back
  as a `*SyntaxError`.

## Tests

```sh
go test ./...            # unit tests plus the whole shared corpus
go test -fuzz FuzzParse  # parse, serialize and re-parse, looking for a fixed point
```
