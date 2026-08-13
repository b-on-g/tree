//! The mapping between trees and JSON.

use std::sync::Arc;

use crate::error::Error;
use crate::span::Span;
use crate::tree::Tree;

/// A JSON value.
///
/// Self-contained on purpose: the core of this crate has no dependencies. The
/// optional `serde` feature adds conversions to and from
/// `serde_json::Value`.
///
/// Objects keep their entries in order, and a repeated key overwrites the entry
/// it repeats without moving it — the way a JavaScript object behaves.
#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    /// `null`.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// A number. `NaN` is reachable through the tree format's `NaN` type.
    Number(f64),
    /// A string.
    String(String),
    /// An array.
    Array(Vec<Json>),
    /// An object, in entry order.
    Object(Vec<(String, Json)>),
}

impl Json {
    /// Makes an object out of its entries, collapsing repeated keys.
    pub fn object(entries: impl IntoIterator<Item = (String, Json)>) -> Self {
        let mut out: Vec<(String, Json)> = Vec::new();

        for (key, value) in entries {
            match out.iter_mut().find(|(known, _)| *known == key) {
                Some(entry) => entry.1 = value,
                None => out.push((key, value)),
            }
        }

        Json::Object(out)
    }

    /// The value under a key, for objects.
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(entries) => entries
                .iter()
                .find(|(known, _)| known == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }
}

/// Converts a tree to JSON, following the node types `*`, `/`, `true`,
/// `false`, `null`, `-` and numbers.
///
/// A node typed `-` is a comment: it is dropped by whatever contains it. At the
/// very root there is nothing to drop it, so it surfaces as [`Json::Null`].
///
/// ```
/// let tree = tree2::parse("* a \\1\n", "?")?;
/// assert_eq!(tree2::to_json(&tree)?.get("a"), Some(&tree2::Json::String("1".into())));
/// # Ok::<_, Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// Fails on an untyped node holding several typed children, on an object entry
/// with no value, and on a type that is neither a literal nor a number.
pub fn to_json(tree: &Tree) -> Result<Json, Error> {
    Ok(convert(tree)?.unwrap_or(Json::Null))
}

/// `None` stands for JavaScript's `undefined` — what a `-` node converts to.
fn convert(tree: &Tree) -> Result<Option<Json>, Error> {
    if tree.kind().is_empty() {
        if tree.kids().iter().all(|kid| kid.kind().is_empty()) {
            return Ok(Some(Json::String(tree.text())));
        }

        let [only] = tree.kids() else {
            return Err(Error::new(
                format!("Multiple json root at {}", tree.span()),
                tree.span().clone(),
            ));
        };

        return convert(only);
    }

    match tree.kind() {
        "-" => Ok(None),
        "true" => Ok(Some(Json::Bool(true))),
        "false" => Ok(Some(Json::Bool(false))),
        "null" => Ok(Some(Json::Null)),
        "*" => object(tree).map(Some),
        "/" => array(tree).map(Some),

        kind => match number(kind) {
            Some(number) => Ok(Some(Json::Number(number))),
            None if kind == "NaN" => Ok(Some(Json::Number(f64::NAN))),
            None => Err(Error::new(
                format!("Unknown json type ({kind}) at {}", tree.span()),
                tree.span().clone(),
            )),
        },
    }
}

/// Every child of a `*` node contributes one entry.
fn object(tree: &Tree) -> Result<Json, Error> {
    let mut entries = Vec::with_capacity(tree.kids().len());

    for kid in tree.kids() {
        if kid.kind() == "-" {
            continue;
        }

        let key = if kid.kind().is_empty() {
            // A data node spells its key out over every child but the last one,
            // which is the value.
            let last = kid.kids().len().saturating_sub(1);
            kid.with_kids(&kid.kids()[..last]).text()
        } else {
            kid.kind().to_owned()
        };

        let Some(value) = kid.kids().last() else {
            return Err(kid
                .span()
                .error(format_args!("Missing json value for key {key}")));
        };

        if let Some(value) = convert(value)? {
            entries.push((key, value));
        }
    }

    Ok(Json::object(entries))
}

/// Every child of a `/` node contributes one item.
fn array(tree: &Tree) -> Result<Json, Error> {
    let mut items = Vec::with_capacity(tree.kids().len());

    for kid in tree.kids() {
        if kid.kind() == "-" {
            continue;
        }
        if let Some(value) = convert(kid)? {
            items.push(value);
        }
    }

    Ok(Json::Array(items))
}

/// Converts JSON to a tree, at a given position.
///
/// ```
/// let json = tree2::Json::Array(vec![tree2::Json::Number(1.0), tree2::Json::Bool(true)]);
/// assert_eq!(tree2::from_json(&json, tree2::Span::unknown()).to_string(), "/\n\t1\n\ttrue\n");
/// ```
pub fn from_json(json: &Json, span: Span) -> Tree {
    match json {
        Json::Null => literal("null", span),
        Json::Bool(true) => literal("true", span),
        Json::Bool(false) => literal("false", span),
        Json::Number(number) => literal(&number_to_string(*number), span),
        Json::String(string) => Tree::data(string, [], span),

        Json::Array(items) => {
            let kids: Vec<Tree> = items
                .iter()
                .map(|item| from_json(item, span.clone()))
                .collect();
            Tree::raw(Arc::from("/"), Arc::from(""), kids.into(), span)
        }

        Json::Object(entries) => {
            let kids: Vec<Tree> = entries
                .iter()
                .map(|(key, value)| {
                    let value = from_json(value, span.clone());
                    if plain_key(key) {
                        Tree::raw(
                            Arc::from(key.as_str()),
                            Arc::from(""),
                            [value].into(),
                            span.clone(),
                        )
                    } else {
                        Tree::data(key, [value], span.clone())
                    }
                })
                .collect();
            Tree::raw(Arc::from("*"), Arc::from(""), kids.into(), span)
        }
    }
}

/// A struct node holding a literal. The spelling never needs validating.
fn literal(kind: &str, span: Span) -> Tree {
    Tree::raw(Arc::from(kind), Arc::from(""), Box::default(), span)
}

/// Whether a key can be a node type, or has to be spelled out as data.
fn plain_key(key: &str) -> bool {
    !key.is_empty() && !key.contains(['\n', '\t', '\\', ' '])
}

/// Renders a number the way JavaScript's `String(number)` does, since that is
/// what the tree format's numeric types are spelled with.
fn number_to_string(number: f64) -> String {
    if number.is_nan() {
        return "NaN".to_owned();
    }
    if number.is_infinite() {
        return if number > 0.0 {
            "Infinity"
        } else {
            "-Infinity"
        }
        .to_owned();
    }
    if number == 0.0 {
        return "0".to_owned();
    }

    let magnitude = number.abs();

    if (1e-6..1e21).contains(&magnitude) {
        return format!("{number}");
    }

    // Outside that range JavaScript switches to exponential notation, and
    // writes a positive exponent with its sign.
    let exponential = format!("{number:e}");
    match exponential.split_once('e') {
        Some((mantissa, exponent)) if !exponent.starts_with('-') => {
            format!("{mantissa}e+{exponent}")
        }
        _ => exponential,
    }
}

/// Reads a number the way JavaScript's `Number(string)` does: an optional sign
/// and `Infinity` or a decimal, or one of the `0x` / `0o` / `0b` radix forms.
/// `None` is JavaScript's `NaN` — the string is not a number at all.
fn number(text: &str) -> Option<f64> {
    if text.is_empty() {
        return Some(0.0);
    }

    if let Some(digits) = strip_prefix_either(text, "0x", "0X") {
        return radix(digits, 16);
    }
    if let Some(digits) = strip_prefix_either(text, "0o", "0O") {
        return radix(digits, 8);
    }
    if let Some(digits) = strip_prefix_either(text, "0b", "0B") {
        return radix(digits, 2);
    }

    let body = text.strip_prefix(['+', '-']).unwrap_or(text);
    let sign = if text.starts_with('-') { -1.0 } else { 1.0 };

    if body == "Infinity" {
        return Some(sign * f64::INFINITY);
    }
    if !decimal(body) {
        return None;
    }

    // Rust reads every spelling that got this far, and a few more — hence the
    // check above, which keeps `inf` and `nan` out.
    text.parse().ok()
}

fn strip_prefix_either<'a>(text: &'a str, one: &str, other: &str) -> Option<&'a str> {
    text.strip_prefix(one).or_else(|| text.strip_prefix(other))
}

fn radix(digits: &str, radix: u32) -> Option<f64> {
    if digits.is_empty() {
        return None;
    }

    let mut value = 0.0f64;

    for digit in digits.chars() {
        value = value * f64::from(radix) + f64::from(digit.to_digit(radix)?);
    }

    Some(value)
}

/// Whether the text spells a decimal literal: digits with an optional fraction
/// and an optional exponent, or a bare fraction.
fn decimal(text: &str) -> bool {
    let (mantissa, exponent) = match text.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => (mantissa, Some(exponent)),
        None => (text, None),
    };

    let (whole, fraction) = match mantissa.split_once('.') {
        Some((whole, fraction)) => (whole, fraction),
        None => (mantissa, ""),
    };

    if whole.is_empty() && fraction.is_empty() {
        return false;
    }
    if !whole.bytes().all(|byte| byte.is_ascii_digit()) {
        return false;
    }
    if !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
        return false;
    }

    match exponent {
        None => true,
        Some(exponent) => {
            let digits = exponent.strip_prefix(['+', '-']).unwrap_or(exponent);
            !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
        }
    }
}

#[cfg(feature = "serde")]
mod serde_bridge {
    use super::Json;

    impl From<serde_json::Value> for Json {
        fn from(value: serde_json::Value) -> Self {
            match value {
                serde_json::Value::Null => Json::Null,
                serde_json::Value::Bool(bool) => Json::Bool(bool),
                serde_json::Value::Number(number) => {
                    Json::Number(number.as_f64().unwrap_or(f64::NAN))
                }
                serde_json::Value::String(string) => Json::String(string),
                serde_json::Value::Array(items) => {
                    Json::Array(items.into_iter().map(Json::from).collect())
                }
                serde_json::Value::Object(entries) => Json::object(
                    entries
                        .into_iter()
                        .map(|(key, value)| (key, Json::from(value))),
                ),
            }
        }
    }

    impl From<Json> for serde_json::Value {
        fn from(json: Json) -> Self {
            match json {
                Json::Null => serde_json::Value::Null,
                Json::Bool(bool) => serde_json::Value::Bool(bool),
                // A whole number goes back as an integer, the way JavaScript
                // writes one — `serde_json` tells `1` and `1.0` apart, and this
                // crate, like JavaScript, does not.
                Json::Number(number) if number.fract() == 0.0 && number.abs() < 9e18 => {
                    serde_json::Value::Number((number as i64).into())
                }
                Json::Number(number) => serde_json::Number::from_f64(number)
                    .map_or(serde_json::Value::Null, serde_json::Value::Number),
                Json::String(string) => serde_json::Value::String(string),
                Json::Array(items) => {
                    serde_json::Value::Array(items.into_iter().map(Into::into).collect())
                }
                Json::Object(entries) => serde_json::Value::Object(
                    entries
                        .into_iter()
                        .map(|(key, value)| (key, value.into()))
                        .collect(),
                ),
            }
        }
    }
}
