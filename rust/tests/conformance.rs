//! The shared conformance corpus, read straight out of `../fixtures`.
//!
//! Every port of the tree format runs the same cases, so the corpus is never
//! vendored — it is loaded from the repository at test time.

use std::path::PathBuf;

use serde_json::Value;

use tree2::{from_json, parse, to_json, Belt, Context, Json, Span, Step, Tree};

// ------------------------------------------------------------------ corpus

fn load(name: &str) -> Vec<Value> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures")
        .join(format!("{name}.json"));

    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));

    serde_json::from_str(&text).unwrap_or_else(|error| panic!("cannot parse {name}.json: {error}"))
}

fn text(case: &Value, key: &str) -> String {
    case[key]
        .as_str()
        .unwrap_or_else(|| panic!("case has no string `{key}`: {case}"))
        .to_owned()
}

fn name(case: &Value) -> String {
    text(case, "name")
}

fn count(kind: &str, cases: usize) {
    println!("{kind}: {cases} cases");
}

// --------------------------------------------------------------- helpers

/// Path steps, spelled in the corpus as strings, numbers and nulls.
fn steps(value: &Value) -> Vec<Step<'_>> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("path is not an array: {value}"))
        .iter()
        .map(|step| match step {
            Value::String(kind) => Step::Kind(kind),
            Value::Number(index) => Step::Index(
                index
                    .as_i64()
                    .unwrap_or_else(|| panic!("index is not an integer: {index}"))
                    as isize,
            ),
            Value::Null => Step::All,
            other => panic!("unknown path step: {other}"),
        })
        .collect()
}

/// The corpus carries both what the reference produces (UTF-16 columns) and
/// what a port must produce (code points).
fn check_span(actual: &Span, expected: &Value, source: &str, at: &str) {
    assert_eq!(actual.uri(), expected["uri"], "{at}: uri");
    assert_eq!(actual.row() as u64, expected["row"], "{at}: row");
    assert_eq!(actual.col() as u64, expected["col_cp"], "{at}: col");
    assert_eq!(
        actual.length() as u64,
        expected["length_cp"],
        "{at}: length"
    );
    assert_eq!(actual.source(), source, "{at}: source");
}

fn check_tree(actual: &Tree, expected: &Value, source: &str, at: &str) {
    assert_eq!(actual.kind(), expected["type"], "{at}: type");
    assert_eq!(actual.value(), expected["value"], "{at}: value");

    if let Some(span) = expected.get("span") {
        check_span(actual.span(), span, source, at);
    }

    let kids = expected["kids"]
        .as_array()
        .unwrap_or_else(|| panic!("{at}: kids is not an array"));

    assert_eq!(actual.kids().len(), kids.len(), "{at}: kids count");

    for (index, (actual, expected)) in actual.kids().iter().zip(kids).enumerate() {
        check_tree(actual, expected, source, &format!("{at}/{index}"));
    }
}

/// The corpus spells JSON with `serde_json`; the crate spells it itself.
fn json_of(value: &Value) -> Json {
    match value {
        Value::Null => Json::Null,
        Value::Bool(bool) => Json::Bool(*bool),
        Value::Number(number) => Json::Number(number.as_f64().expect("finite number")),
        Value::String(string) => Json::String(string.clone()),
        Value::Array(items) => Json::Array(items.iter().map(json_of).collect()),
        Value::Object(entries) => Json::Object(
            entries
                .iter()
                .map(|(key, value)| (key.clone(), json_of(value)))
                .collect(),
        ),
    }
}

// ----------------------------------------------------------------- parse

#[test]
fn parses() {
    let cases = load("parse");

    for case in &cases {
        let (name, source) = (name(case), text(case, "input"));
        let uri = text(case, "uri");

        let tree = parse(source.clone(), uri).unwrap_or_else(|error| panic!("{name}: {error}"));

        check_tree(&tree, &case["tree"], &source, &name);
    }

    count("parse", cases.len());
}

#[test]
fn parses_spans() {
    let cases = load("spans");

    for case in &cases {
        let (name, source) = (name(case), text(case, "input"));
        let uri = text(case, "uri");

        let tree = parse(source.clone(), uri).unwrap_or_else(|error| panic!("{name}: {error}"));

        check_tree(&tree, &case["tree"], &source, &name);
    }

    count("spans", cases.len());
}

#[test]
fn reports_syntax_errors() {
    let cases = load("parse_errors");

    for case in &cases {
        let (name, source) = (name(case), text(case, "input"));

        let Err(error) = parse(source.clone(), text(case, "uri")) else {
            panic!("{name}: parsed without an error");
        };

        assert_eq!(error.reason(), text(case, "reason"), "{name}: reason");
        assert_eq!(error.line(), text(case, "line"), "{name}: line");
        check_span(error.span(), &case["span"], &source, &name);

        // The rendered message embeds the reference's UTF-16 column, so it only
        // has to match where the two agree — which is everywhere but the astral
        // planes, and every error case is plain ASCII.
        if source.is_ascii() {
            assert_eq!(error.to_string(), text(case, "message"), "{name}: message");
        }
    }

    count("parse_errors", cases.len());
}

// ------------------------------------------------------------- serialize

#[test]
fn serializes() {
    let cases = load("serialize");

    for case in &cases {
        let (name, source) = (name(case), text(case, "input"));
        let expected = text(case, "output");

        let once = parse(source, "test")
            .unwrap_or_else(|error| panic!("{name}: {error}"))
            .to_string();
        assert_eq!(once, expected, "{name}");

        let twice = parse(once.clone(), "test")
            .unwrap_or_else(|error| panic!("{name}: {error}"))
            .to_string();
        assert_eq!(twice, once, "{name}: not a fixed point");
    }

    count("serialize", cases.len());
}

/// Trees the parser could never produce, built through the factories instead.
#[test]
fn serializes_built_trees() {
    let cases = load("serialize_built");
    let here = Span::unknown;

    for case in &cases {
        let name = name(case);

        let built = match name.as_str() {
            "multiline data splits into kids" => Tree::data("a\nb\nc", [], here()),
            "multiline data with extra kids" => Tree::data(
                "a\nb",
                [Tree::structure("x", [], here()).expect("valid type")],
                here(),
            ),
            "struct with no kids" => Tree::structure("foo", [], here()).expect("valid type"),
            "struct with one kid collapses inline" => Tree::structure(
                "a",
                [Tree::structure("b", [], here()).expect("valid type")],
                here(),
            )
            .expect("valid type"),
            "struct with two kids goes multiline" => Tree::structure(
                "a",
                [
                    Tree::structure("b", [], here()).expect("valid type"),
                    Tree::structure("c", [], here()).expect("valid type"),
                ],
                here(),
            )
            .expect("valid type"),
            "list of structs" => Tree::list(
                [
                    Tree::structure("a", [], here()).expect("valid type"),
                    Tree::structure("b", [], here()).expect("valid type"),
                ],
                here(),
            ),
            "empty list" => Tree::list([], here()),
            "data at root" => Tree::list([Tree::data("x", [], here())], here()),
            "empty data at root" => Tree::list([Tree::data("", [], here())], here()),
            "nested data under struct" => Tree::structure(
                "a",
                [Tree::data("x", [], here()), Tree::data("y", [], here())],
                here(),
            )
            .expect("valid type"),
            other => panic!("no builder for case {other}"),
        };

        check_tree(&built, &case["tree"], "", &name);
        assert_eq!(built.to_string(), text(case, "output"), "{name}");
    }

    count("serialize_built", cases.len());
}

#[test]
fn rejects_wrong_types() {
    for kind in ["a b", "a\tb", "a\nb", "a\\b"] {
        let error = Tree::structure(kind, [], Span::begin("test", ""))
            .expect_err("a type with a delimiter in it");

        assert_eq!(
            error.to_string(),
            format!(
                "Wrong type {} (test#1:1/0)",
                serde_json::to_string(kind).expect("quotable")
            ),
        );
    }
}

// --------------------------------------------------------------- queries

#[test]
fn takes_text() {
    let cases = load("text");

    for case in &cases {
        let (name, source) = (name(case), text(case, "input"));
        let tree = parse(source, "test").unwrap_or_else(|error| panic!("{name}: {error}"));

        assert_eq!(tree.text(), text(case, "root_text"), "{name}: root");
        assert_eq!(
            tree.kids()[0].text(),
            text(case, "text"),
            "{name}: first kid"
        );
    }

    count("text", cases.len());
}

#[test]
fn selects() {
    let cases = load("select");

    for case in &cases {
        let (name, source) = (name(case), text(case, "input"));
        let tree = parse(source, "test").unwrap_or_else(|error| panic!("{name}: {error}"));

        let found = tree.select(&steps(&case["path"]));

        assert_eq!(found.to_string(), text(case, "output"), "{name}");
    }

    count("select", cases.len());
}

#[test]
fn filters() {
    let cases = load("filter");

    for case in &cases {
        let (name, source) = (name(case), text(case, "input"));
        let tree = parse(source, "test").unwrap_or_else(|error| panic!("{name}: {error}"));

        let value = case["has_value"]
            .as_bool()
            .expect("has_value")
            .then(|| text(case, "value"));

        let kept = tree.kids()[0].filter(&steps(&case["path"]), value.as_deref());

        assert_eq!(kept.to_string(), text(case, "output"), "{name}");
    }

    count("filter", cases.len());
}

// ----------------------------------------------------------------- edits

#[test]
fn inserts() {
    let cases = load("insert");

    for case in &cases {
        let (name, source) = (name(case), text(case, "input"));
        let tree = parse(source, "test").unwrap_or_else(|error| panic!("{name}: {error}"));

        let value = case["insert"]
            .as_str()
            .map(|kind| Tree::structure(kind, [], Span::unknown()).expect("valid type"));

        let edited = tree
            .insert(value.as_ref(), &steps(&case["path"]))
            .unwrap_or_else(|error| panic!("{name}: {error}"));

        assert_eq!(edited.to_string(), text(case, "output"), "{name}");
    }

    count("insert", cases.len());
}

#[test]
fn updates() {
    let cases = load("update");

    for case in &cases {
        let (name, source) = (name(case), text(case, "input"));
        let tree = parse(source, "test").unwrap_or_else(|error| panic!("{name}: {error}"));

        let values = structs(&case["update"]);

        let edited = tree
            .update(&values, &steps(&case["path"]))
            .unwrap_or_else(|error| panic!("{name}: {error}"));

        assert_eq!(edited[0].to_string(), text(case, "output"), "{name}");
    }

    count("update", cases.len());
}

fn structs(value: &Value) -> Vec<Tree> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("not an array: {value}"))
        .iter()
        .map(|kind| {
            Tree::structure(kind.as_str().expect("type"), [], Span::unknown()).expect("valid type")
        })
        .collect()
}

// ------------------------------------------------------------------ json

#[test]
fn converts_to_json() {
    let cases = load("to_json");

    for case in &cases {
        let (name, source) = (name(case), text(case, "input"));
        let tree = parse(source, "test").unwrap_or_else(|error| panic!("{name}: {error}"));

        let json = to_json(&tree).unwrap_or_else(|error| panic!("{name}: {error}"));

        assert_eq!(json, json_of(&case["json"]), "{name}");
    }

    count("to_json", cases.len());
}

#[test]
fn converts_from_json() {
    let cases = load("from_json");

    for case in &cases {
        let name = name(case);
        let json = json_of(&case["json"]);

        let tree = from_json(&json, Span::unknown());

        assert_eq!(tree.to_string(), text(case, "output"), "{name}");

        // Every rendering must parse back to the JSON it came from.
        let back =
            parse(tree.to_string(), "test").unwrap_or_else(|error| panic!("{name}: {error}"));
        let round = to_json(&back).unwrap_or_else(|error| panic!("{name}: {error}"));

        assert_eq!(round, json, "{name}: round trip");
    }

    count("from_json", cases.len());
}

// ------------------------------------------------------- reference bugs

/// The cases where the reference implementation is wrong and this port is
/// deliberately not — plus the ones marked `agrees`, which pin how far the
/// corrections go.
#[test]
fn corrects_reference_bugs() {
    let cases = load("reference_bugs");

    for case in &cases {
        let (name, source) = (name(case), text(case, "input"));

        // A parse case fails on purpose; the rest operate on a parsed tree.
        if text(case, "op") == "parse" {
            let Err(error) = parse(source, "test") else {
                panic!("{name}: parsed without an error");
            };
            let expected = &case["error"];

            assert_eq!(error.reason(), expected["reason"], "{name}: reason");
            assert_eq!(error.span().row() as u64, expected["row"], "{name}: row");
            assert_eq!(error.span().col() as u64, expected["col"], "{name}: col");
            assert_eq!(
                error.span().length() as u64,
                expected["length"],
                "{name}: length"
            );

            continue;
        }

        let tree = parse(source, "test").unwrap_or_else(|error| panic!("{name}: {error}"));
        let path = steps(&case["path"]);

        let output = match text(case, "op").as_str() {
            "select" => tree.select(&path).to_string(),
            "update" => tree
                .update(&structs(&case["update"]), &path)
                .unwrap_or_else(|error| panic!("{name}: {error}"))[0]
                .to_string(),
            "insert" => {
                let value = case["insert"]
                    .as_str()
                    .map(|kind| Tree::structure(kind, [], Span::unknown()).expect("valid type"));
                tree.insert(value.as_ref(), &path)
                    .unwrap_or_else(|error| panic!("{name}: {error}"))
                    .to_string()
            }
            other => panic!("unknown op {other}"),
        };

        assert_eq!(output, text(case, "output"), "{name}");
    }

    count("reference_bugs", cases.len());
}

// ------------------------------------------------------------------ hack

#[test]
fn hacks() {
    let tree = parse("a\n\tb \\x\n\tc \\y\n", "test").expect("parses");

    // `b` is replaced in place by two nodes; `c` is left to the identity
    // handler, which keeps it and rewrites its children.
    let belt = Belt::<Vec<String>>::new().with("b", |node, _belt, context| {
        context.data.push(node.text());
        Ok(vec![
            node.new_structure("one", [])?,
            node.new_structure("two", [])?,
        ])
    });

    let mut context = Context::new(Vec::new());
    let hacked = tree.hack(&belt, &mut context).expect("hacked");

    assert_eq!(hacked.len(), 1);
    assert_eq!(hacked[0].to_string(), "a\n\tone\n\ttwo\n\tc \\y\n");
    assert_eq!(context.data, ["x"]);
}

#[test]
fn hack_errors_carry_the_node() {
    let tree = parse("a\n\tb \\x\n", "test").expect("parses");

    let belt = Belt::<()>::new().with("b", |node, _belt, _context| Err(node.error("Nope")));

    let error = tree
        .hack(&belt, &mut Context::new(()))
        .expect_err("the handler fails");

    // The failing node, then every node the error passed on its way out.
    assert_eq!(
        error.to_string(),
        "Nope\nb\n (test#2:2/1)\nb\ntest#2:2/1\na\ntest#1:1/1",
    );
}
