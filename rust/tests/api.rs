//! The parts of the API the shared corpus does not reach: spans, the odd
//! corners of the parser, JSON numbers, and the promise never to panic.

use tree2::{from_json, parse, to_json, Json, Span, Step, Tree};

// ------------------------------------------------------------------ spans

#[test]
fn spans_render_as_coordinates() {
    assert_eq!(Span::begin("t", "").to_string(), "t#1:1/0");
    assert_eq!(Span::end("t", "abc").to_string(), "t#1:4/0");
    assert_eq!(Span::entire("t", "hello world").to_string(), "t#1:1/11");
    assert_eq!(Span::unknown().to_string(), "?#1:1/0");
}

#[test]
fn spans_measure_code_points() {
    // Four UTF-16 code units, three code points.
    assert_eq!(Span::entire("t", "a😀b").length(), 3);
    assert_eq!(Span::end("t", "a😀b").col(), 4);
}

#[test]
fn spans_move_and_narrow() {
    let span = Span::entire("t", "hello world");

    assert_eq!(span.after(2).to_string(), "t#1:12/2");
    assert_eq!(span.span(3, 4, 5).to_string(), "t#3:4/5");

    // The reference's default end is -1, one short of the whole span.
    assert_eq!(
        span.slice_from(0).expect("in range").to_string(),
        "t#1:1/10"
    );
    assert_eq!(span.slice(2, 5).expect("in range").to_string(), "t#1:3/3");
    assert_eq!(
        span.slice_from(-3).expect("in range").to_string(),
        "t#1:9/2"
    );
}

#[test]
fn spans_refuse_indices_they_do_not_hold() {
    let span = Span::entire("t", "hello world");

    let messages = [
        span.slice_from(20).expect_err("begin past the end"),
        span.slice(0, 99).expect_err("end past the end"),
        span.slice(5, 2).expect_err("end before begin"),
    ]
    .map(|error| error.to_string());

    assert_eq!(
        messages,
        [
            "Begin value '20' out of range (t#1:1/11)",
            "End value '99' out of range (t#1:1/11)",
            "End value '2' can't be less than begin value (t#1:1/11)",
        ],
    );
}

#[test]
fn errors_carry_their_position() {
    let tree = parse("a b\n", "t").expect("parses");

    assert_eq!(
        tree.kids()[0].error("Boom").to_string(),
        "Boom\na\n (t#1:1/1)"
    );
    assert_eq!(tree.kids()[0].error("Boom").span().to_string(), "t#1:1/1");
}

// --------------------------------------------------------------- factories

#[test]
fn multi_line_data_splits_into_one_child_per_line() {
    let node = Tree::data("a\nbb\nc", [], Span::begin("t", "x"));

    assert_eq!(node.value(), "");
    assert_eq!(
        node.kids()
            .iter()
            .map(|kid| format!("{}@{}", kid.value(), kid.span()))
            .collect::<Vec<_>>(),
        ["a@t#1:1/1", "bb@t#1:2/2", "c@t#1:4/1"],
    );
}

#[test]
fn derived_factories_reuse_the_receiver_position() {
    let tree = parse("a\n\tb\n", "t").expect("parses");
    let b = &tree.kids()[0].kids()[0];

    assert_eq!(b.new_data("x", []).span().to_string(), "t#2:2/1");
    assert_eq!(
        b.new_structure("x", [])
            .expect("valid type")
            .span()
            .to_string(),
        "t#2:2/1",
    );
    assert_eq!(b.new_list([]).span().to_string(), "t#2:2/1");
    assert_eq!(b.with_kids([]).span().to_string(), "t#2:2/1");
    assert_eq!(
        b.with_kids_at([], Span::unknown()).span().to_string(),
        "?#1:1/0",
    );
}

#[test]
fn nodes_compare_by_shape_not_by_position() {
    let one = parse("a b\n", "one").expect("parses");
    let other = parse("a\n\tb\n", "other").expect("parses");

    assert_eq!(one, other);
    assert_ne!(one, parse("a c\n", "one").expect("parses"));
}

// ----------------------------------------------------------------- parser

#[test]
fn a_blank_line_unwinds_the_nesting() {
    // Matches the reference: after a blank line the next line hangs off the
    // root again, tab or no tab.
    assert_eq!(
        parse("a\n\n\tb\n", "t").expect("parses").to_string(),
        "a\nb\n"
    );
    assert_eq!(
        parse("a\n\n\n\tb\n", "t").expect("parses").to_string(),
        "a\nb\n",
    );
}

#[test]
fn a_missing_final_newline_does_not_mask_an_indent_error() {
    // The reference lets a dedented last line off when the document does not end
    // in a newline, and then either drops the line or crashes.
    let one = parse("\t\tfoo\n\tbar", "t").expect_err("too few tabs");
    assert_eq!(one.reason(), "Too few tabs");
    assert_eq!(one.span().to_string(), "t#2:1/1");

    let deeper = parse("\t\ta\nb", "t").expect_err("too few tabs");
    assert_eq!(deeper.reason(), "Too few tabs");
    assert_eq!(deeper.span().to_string(), "t#2:1/0");

    // Terminating the source changes nothing about an indent error. The
    // invariant is that narrow: `\tfoo` is an unterminated last line and fails
    // with `Unexpected EOF, LF required`, while `\tfoo\n` parses.
    for source in ["\ta\nb", "\t\ta\n\tb", "\t\ta\nb"] {
        let unterminated = parse(source, "t").expect_err("too few tabs");
        let terminated = parse(format!("{source}\n"), "t").expect_err("too few tabs");

        assert_eq!(unterminated.reason(), "Too few tabs", "{source:?}");
        assert_eq!(
            unterminated.to_string(),
            terminated.to_string(),
            "{source:?}"
        );
    }

    assert!(parse("\tfoo\n", "t").is_ok());
    assert_eq!(
        parse("\tfoo", "t").expect_err("no newline").reason(),
        "Unexpected EOF, LF required",
    );
}

#[test]
fn a_carriage_return_is_ordinary_content() {
    let tree = parse("a \\x\r\n", "t").expect("parses");
    assert_eq!(tree.kids()[0].kids()[0].value(), "x\r");
}

#[test]
fn deep_nesting_does_not_overflow_the_stack() {
    // Indentation makes a deep document quadratic in size, so this is as far as
    // a test wants to go — parsing, serializing and dropping are all iterative,
    // and none of them cares about the depth.
    let depth = 4_000;

    let mut source = String::new();
    for level in 0..depth {
        source.push_str(&"\t".repeat(level));
        source.push_str("a\n");
    }

    let tree = parse(source, "t").expect("parses");

    // Every level holds the next one, so serialization collapses it all onto
    // one line.
    let flat = tree.to_string();
    assert_eq!(flat.matches('a').count(), depth);
    assert_eq!(flat.lines().count(), 1);
}

#[test]
fn dropping_a_deep_tree_does_not_overflow_the_stack() {
    let mut tree = Tree::structure("a", [], Span::unknown()).expect("valid type");

    for _ in 0..200_000 {
        tree = Tree::structure("a", [tree], Span::unknown()).expect("valid type");
    }

    drop(tree);
}

#[test]
fn nothing_panics_on_junk() {
    // Delimiters, whitespace, astral characters and the ends of lines, in every
    // arrangement of up to four of them.
    let alphabet = ['\t', ' ', '\\', '\n', 'x', '😀'];
    let mut source = String::new();

    for a in alphabet {
        for b in alphabet {
            for c in alphabet {
                for d in alphabet {
                    source.clear();
                    source.extend([a, b, c, d]);

                    let Ok(tree) = parse(source.clone(), "junk") else {
                        continue;
                    };

                    // Whatever came out has to be a fixed point of the format.
                    let once = tree.to_string();
                    let twice = parse(once.clone(), "junk").expect("reparses").to_string();
                    assert_eq!(once, twice, "not a fixed point: {source:?}");

                    let _ = tree.text();
                    let _ = to_json(&tree);
                    let _ = tree.select(&[Step::All, Step::Index(-1), Step::Kind("x")]);
                    let _ = tree.update(&[], &[Step::Index(-3), Step::All]);
                }
            }
        }
    }
}

// ------------------------------------------------------------------ edits

#[test]
fn an_out_of_range_index_appends() {
    let tree = parse("a\n\tx\n", "t").expect("parses");
    let value = Tree::structure("v", [], Span::unknown()).expect("valid type");

    let edited = tree
        .insert(Some(&value), &[Step::Kind("a"), Step::Index(7)])
        .expect("inserts");

    assert_eq!(edited.to_string(), "a\n\tx\n\tv\n");
}

#[test]
fn a_negative_index_matches_nothing() {
    let tree = parse("a\n\tx\n\ty\n", "t").expect("parses");

    assert_eq!(tree.select(&[Step::Kind("a"), Step::Index(-1)]).kids(), []);
}

#[test]
fn there_is_nothing_to_insert_at_an_empty_path() {
    let tree = parse("a\n", "t").expect("parses");

    assert_eq!(
        tree.insert(None, &[])
            .expect_err("nothing comes out")
            .to_string(),
        "Nothing to insert (t#1:1/2)",
    );
}

#[test]
fn a_kind_no_node_may_carry_is_refused() {
    let tree = parse("a\n", "t").expect("parses");
    let value = Tree::structure("v", [], Span::unknown()).expect("valid type");

    let error = tree
        .insert(Some(&value), &[Step::Kind("no way")])
        .expect_err("the kind cannot be created");

    assert_eq!(error.to_string(), "Wrong type \"no way\" (t#1:1/2)");
}

// ------------------------------------------------------------------- json

#[test]
fn numbers_are_read_the_way_javascript_reads_them() {
    let read = |source: &str| {
        let tree = parse(format!("{source}\n"), "t").expect("parses");
        to_json(&tree).expect("converts")
    };

    assert_eq!(read("1"), Json::Number(1.0));
    assert_eq!(read("-1.5"), Json::Number(-1.5));
    assert_eq!(read(".5"), Json::Number(0.5));
    assert_eq!(read("1e3"), Json::Number(1000.0));
    assert_eq!(read("0x10"), Json::Number(16.0));
    assert_eq!(read("0b101"), Json::Number(5.0));
    assert_eq!(read("0o17"), Json::Number(15.0));
    assert_eq!(read("Infinity"), Json::Number(f64::INFINITY));
    assert_eq!(read("-Infinity"), Json::Number(f64::NEG_INFINITY));

    let Json::Number(nan) = read("NaN") else {
        panic!("NaN is a number");
    };
    assert!(nan.is_nan());
}

#[test]
fn a_type_that_is_not_a_number_is_not_json() {
    let tree = parse("zzz\n", "t").expect("parses");

    assert_eq!(
        to_json(&tree).expect_err("unknown type").to_string(),
        "Unknown json type (zzz) at t#1:1/3",
    );

    // Rust reads these; JavaScript does not, and neither does the format.
    for junk in ["inf", "nan", "1_0"] {
        let tree = parse(format!("{junk}\n"), "t").expect("parses");
        assert!(to_json(&tree).is_err(), "{junk} is not a number");
    }
}

#[test]
fn several_typed_roots_are_not_json() {
    let tree = parse("a\nb\n", "t").expect("parses");

    assert_eq!(
        to_json(&tree).expect_err("two roots").to_string(),
        "Multiple json root at t#1:1/4",
    );
}

#[test]
fn an_object_entry_without_a_value_is_not_json() {
    let tree = parse("*\n\ta\n", "t").expect("parses");

    assert_eq!(
        to_json(&tree).expect_err("no value").to_string(),
        "Missing json value for key a (t#2:2/1)",
    );
}

#[test]
fn a_comment_node_drops_the_entry_that_holds_it() {
    let tree = parse("-\n", "t").expect("parses");
    assert_eq!(to_json(&tree).expect("converts"), Json::Null);

    let tree = parse("/\n\t- \\a\n\t\\b\n", "t").expect("parses");
    assert_eq!(
        to_json(&tree).expect("converts"),
        Json::Array(vec![Json::String("b".into())]),
    );
}

#[test]
fn numbers_are_written_the_way_javascript_writes_them() {
    let write = |number: f64| from_json(&Json::Number(number), Span::unknown()).to_string();

    assert_eq!(write(1.0), "1\n");
    assert_eq!(write(-1.5), "-1.5\n");
    assert_eq!(write(-0.0), "0\n");
    assert_eq!(write(1e21), "1e+21\n");
    assert_eq!(write(1.5e-7), "1.5e-7\n");
    assert_eq!(write(f64::INFINITY), "Infinity\n");
    assert_eq!(write(f64::NAN), "NaN\n");
}

#[test]
fn nested_json_round_trips() {
    let json = Json::object([
        (
            "a".to_owned(),
            Json::object([(
                "b".to_owned(),
                Json::Array(vec![Json::Number(1.0), Json::String("x".into())]),
            )]),
        ),
        ("".to_owned(), Json::Number(2.0)),
    ]);

    let tree = from_json(&json, Span::unknown());
    assert_eq!(
        tree.to_string(),
        "*\n\ta * b /\n\t\t1\n\t\t\\x\n\t\\\n\t\t2\n"
    );

    let back = parse(tree.to_string(), "t").expect("parses");
    assert_eq!(to_json(&back).expect("converts"), json);
}

#[test]
fn repeated_object_keys_collapse_where_the_first_one_stood() {
    let json = Json::object([
        ("a".to_owned(), Json::Number(1.0)),
        ("b".to_owned(), Json::Number(2.0)),
        ("a".to_owned(), Json::Number(3.0)),
    ]);

    assert_eq!(
        json,
        Json::Object(vec![
            ("a".to_owned(), Json::Number(3.0)),
            ("b".to_owned(), Json::Number(2.0)),
        ]),
    );
}

// ------------------------------------------------------------------ serde

#[cfg(feature = "serde")]
#[test]
fn serde_values_convert_both_ways() {
    let value: serde_json::Value =
        serde_json::from_str(r#"{"a":[1,"x",true,null],"b":{"c":1.5}}"#).expect("parses");

    let json = Json::from(value.clone());
    let tree = from_json(&json, Span::unknown());
    let back = to_json(&parse(tree.to_string(), "t").expect("parses")).expect("converts");

    assert_eq!(serde_json::Value::from(back), value);
}
