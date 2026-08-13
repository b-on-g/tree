# tree2

Parser, serializer and AST for the **tree** format, in Rust.

`tree` is a whitespace-significant format for structural data with exactly three
lexical ingredients — the tab, the space and the backslash — and no escaping
rules at all. Indentation is the nesting, a backslash starts raw text that runs
to the end of the line, and that is the whole grammar.

```text
server
	host \0.0.0.0
	port \8080
	greeting
		\Hello,
		\world!
```

This is a port of [`$mol_tree2`](https://github.com/hyoo-ru/mam_mol/tree/master/tree2),
following the [shared specification](../SPEC.md) and passing the
[shared conformance corpus](../fixtures) that every port of the format runs.

## Install

```toml
[dependencies]
tree2 = "0.1"
```

The crate has no dependencies. The optional `serde` feature adds conversions
between `Json` and `serde_json::Value`.

## Parsing and reading

```rust
use tree2::{parse, Step};

let source = "\
config
\thost \\localhost
\tport \\8080
";

let tree = parse(source, "app.tree")?;

// A path of steps: a kind, an index, or every child.
let port = tree.select(&[Step::Kind("config"), Step::Kind("port")]);
assert_eq!(port.kids()[0].text(), "8080");

// Every node remembers where it came from.
assert_eq!(port.kids()[0].span().to_string(), "app.tree#3:2/4");

// Serialization is a fixed point, and collapses a single child onto one line.
assert_eq!(tree.to_string(), source);
# Ok::<_, tree2::SyntaxError>(())
```

A `&str` converts into a `Step`, so a path of kinds can be written
`&["config".into(), "port".into()]`.

## Syntax errors

An error knows what went wrong, which line it happened on, and where:

```rust
let error = tree2::parse("foo  bar\n", "test").expect_err("two spaces");

assert_eq!(error.reason(), "Wrong nodes separator");
assert_eq!(error.span().to_string(), "test#1:5/1");
assert_eq!(
    error.to_string(),
    "Wrong nodes separator\ntest#1:5/1\n    !\nfoo  bar",
);
```

## Editing

Nodes are immutable, so every edit returns a new tree that shares everything it
did not touch:

```rust
use tree2::{parse, Span, Step, Tree};

let tree = parse("config\n\tport \\8080\n", "app.tree")?;

// Replace what is at a path — creating the path if it is missing.
let value = Tree::data("9090", [], Span::unknown());
let path = [Step::Kind("config"), Step::Kind("port"), Step::Index(0)];
let edited = tree.insert(Some(&value), &path)?;
assert_eq!(edited.to_string(), "config port \\9090\n");

// `None` deletes.
let cleared = tree.insert(None, &[Step::Kind("config"), Step::Kind("port")])?;
assert_eq!(cleared.to_string(), "config\n");
# Ok::<_, Box<dyn std::error::Error>>(())
```

## JSON

```rust
use tree2::{from_json, parse, to_json, Json, Span};

let tree = parse("* \n\tname \\Alice\n\tage 33\n", "user.tree")?;
let json = to_json(&tree)?;

assert_eq!(json.get("name"), Some(&Json::String("Alice".into())));
assert_eq!(json.get("age"), Some(&Json::Number(33.0)));

assert_eq!(from_json(&json, Span::unknown()).to_string(), "*\n\tname \\Alice\n\tage 33\n");
# Ok::<_, Box<dyn std::error::Error>>(())
```

`Json` is defined by this crate, so the core stays dependency-free. Objects keep
their entries in order.

## Rewriting

`hack` runs a set of handlers over a tree, splicing each handler's output in
place of the node it matched. Unknown kinds are kept, and their children
rewritten:

```rust
use tree2::{parse, Belt, Context};

let belt = Belt::<()>::new().with("greet", |node, _belt, _context| {
    Ok(vec![node.new_data(format!("Hello, {}!", node.text()), [])])
});

let tree = parse("page\n\tgreet \\world\n", "page.tree")?;
let hacked = tree.hack(&belt, &mut Context::new(()))?;

assert_eq!(hacked[0].to_string(), "page \\Hello, world!\n");
# Ok::<_, Box<dyn std::error::Error>>(())
```

## What this port decides

* **Columns are code points**, not the reference's UTF-16 code units. A span
  reads the way a human counts characters, in every language the format is
  ported to.
* **The three reference bugs are fixed**, as the specification requires: a
  dedented last line fails with `Too few tabs` whether or not the document ends
  in a newline, deleting an absent path leaves it absent, and a negative index
  step matches nothing rather than producing a hole.
* **Nothing panics on malformed input.** Every fallible operation returns a
  `Result`; there is no `unwrap` in the crate, and no `unsafe`.
* **Parsing, serializing and dropping a tree are iterative**, so a document
  nested thousands of levels deep is handled without touching the stack depth.
* `Tree::kind` is the format's `type`, since `type` is a Rust keyword.

## Tests

The suite reads `../fixtures/*.json` straight from the repository — the corpus
is shared with the other ports and never vendored.

```sh
cargo test
cargo test --all-features   # includes the serde bridge
```

## Licence

MIT, following upstream `$mol`.
