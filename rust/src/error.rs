//! Errors raised by the parser and by the tree API.

use std::fmt;

use crate::span::Span;

/// An error carrying the source position it was raised at.
///
/// The rendered text is whatever the raiser passed; [`Span::error`] is the
/// usual way to build one and appends ` ({span})` to the message.
#[derive(Clone, Debug)]
pub struct Error {
    message: String,
    span: Span,
}

impl Error {
    /// Makes an error whose message is used verbatim.
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }

    /// The rendered message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Where the error was raised.
    pub fn span(&self) -> &Span {
        &self.span
    }

    /// Appends context to the message, keeping the span.
    ///
    /// This is how [`crate::Tree::hack_self`] annotates an error with the node
    /// that was being rewritten when it happened.
    #[must_use]
    pub fn annotated(mut self, tail: impl fmt::Display) -> Self {
        use fmt::Write as _;
        // Writing into a String cannot fail.
        let _ = write!(self.message, "{tail}");
        self
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

/// A syntax error: what went wrong, the offending line, and where it is.
///
/// Rendered over four lines — the reason, the span, a row of `!` markers under
/// the offending region, and the line itself:
///
/// ```text
/// Wrong nodes separator
/// test#1:5/1
///     !
/// foo  bar
/// ```
#[derive(Clone, Debug)]
pub struct SyntaxError {
    reason: String,
    line: String,
    span: Span,
}

impl SyntaxError {
    /// Makes a syntax error.
    pub fn new(reason: impl Into<String>, line: impl Into<String>, span: Span) -> Self {
        Self {
            reason: reason.into(),
            line: line.into(),
            span,
        }
    }

    /// What went wrong, e.g. `Wrong nodes separator`.
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// The offending line, without its terminator.
    pub fn line(&self) -> &str {
        &self.line
    }

    /// Where in the source the error is.
    pub fn span(&self) -> &Span {
        &self.span
    }
}

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use fmt::Write as _;

        writeln!(f, "{}", self.reason)?;
        writeln!(f, "{}", self.span)?;

        // Everything before the marked region, with non-whitespace blanked out,
        // so the markers line up under the offending text.
        for char in self.line.chars().take(self.span.col().saturating_sub(1)) {
            f.write_char(if char.is_whitespace() { char } else { ' ' })?;
        }
        for _ in 0..self.span.length() {
            f.write_char('!')?;
        }

        write!(f, "\n{}", self.line)
    }
}

impl std::error::Error for SyntaxError {}
