//! The **tree** format: parser, serializer and AST.
//!
//! `tree` is a whitespace-significant format for structural data with exactly
//! three lexical ingredients — the tab, the space and the backslash — and no
//! escaping rules at all. It is what [`$mol`](https://mol.hyoo.ru/) writes its
//! `view.tree` and `meta.tree` in, and it works just as well as a config or an
//! intermediate representation on its own.
//!
//! ```
//! let source = "\
//! server
//! \thost \\0.0.0.0
//! \tport \\8080
//! \tgreeting
//! \t\t\\Hello,
//! \t\t\\world!
//! ";
//!
//! let tree = tree2::parse(source, "server.tree")?;
//!
//! let host = tree.select(&["server".into(), "host".into()]);
//! assert_eq!(host.kids()[0].text(), "0.0.0.0");
//!
//! let greeting = tree.select(&["server".into(), "greeting".into()]);
//! assert_eq!(greeting.kids()[0].text(), "Hello,\nworld!");
//!
//! // Nodes carry where they came from.
//! assert_eq!(host.kids()[0].span().to_string(), "server.tree#2:2/4");
//!
//! // Serialization is a fixed point, and collapses single children onto one line.
//! assert_eq!(tree.to_string(), source);
//! # Ok::<_, tree2::SyntaxError>(())
//! ```
//!
//! This is a port of [`$mol_tree2`](https://github.com/hyoo-ru/mam_mol/tree/master/tree2),
//! following the shared specification of the [`tree2`](https://github.com/b-on-g/tree2)
//! repository. It measures columns in Unicode scalar values rather than in the
//! reference's UTF-16 code units, and it corrects the three reference bugs the
//! specification names.

#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]

/// The examples in the readme are doctests too.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct Readme;

mod error;
mod json;
mod parse;
mod serialize;
mod span;
mod tree;

pub use crate::error::{Error, SyntaxError};
pub use crate::json::{from_json, to_json, Json};
pub use crate::parse::parse;
pub use crate::span::Span;
pub use crate::tree::{Belt, Context, Handler, Step, Tree};
