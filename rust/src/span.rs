//! Positions inside a source resource.

use std::fmt;
use std::sync::Arc;

use crate::error::Error;

/// A region of a source resource: a row, a column and a length.
///
/// Columns and lengths are measured in **Unicode scalar values** (code points),
/// not in UTF-16 code units as the JavaScript reference does. See the
/// specification, §2.3.
///
/// A span carries the whole source text, shared between every span of the same
/// resource, so cloning one is two atomic increments.
///
/// ```
/// let span = tree2::Span::entire("test", "hello\n");
/// assert_eq!(span.to_string(), "test#1:1/6");
/// ```
#[derive(Clone, PartialEq, Eq)]
pub struct Span {
    uri: Arc<str>,
    source: Arc<str>,
    row: usize,
    col: usize,
    length: usize,
}

impl Span {
    /// Makes a span over an explicit region.
    ///
    /// `row` and `col` are 1-based; `length` is a count of code points.
    pub fn new(
        uri: impl Into<Arc<str>>,
        source: impl Into<Arc<str>>,
        row: usize,
        col: usize,
        length: usize,
    ) -> Self {
        Self {
            uri: uri.into(),
            source: source.into(),
            row,
            col,
            length,
        }
    }

    /// Span for the beginning of a resource.
    pub fn begin(uri: impl Into<Arc<str>>, source: impl Into<Arc<str>>) -> Self {
        Self::new(uri, source, 1, 1, 0)
    }

    /// Span for the end of a resource.
    pub fn end(uri: impl Into<Arc<str>>, source: impl Into<Arc<str>>) -> Self {
        let source = source.into();
        let col = source.chars().count() + 1;
        Self::new(uri, source, 1, col, 0)
    }

    /// Span covering an entire resource.
    pub fn entire(uri: impl Into<Arc<str>>, source: impl Into<Arc<str>>) -> Self {
        let source = source.into();
        let length = source.chars().count();
        Self::new(uri, source, 1, 1, length)
    }

    /// Span for the beginning of an unknown resource, named `?`.
    pub fn unknown() -> Self {
        Self::begin("?", "")
    }

    /// Name of the source resource.
    pub fn uri(&self) -> &str {
        &self.uri
    }

    /// Full text of the source resource.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// 1-based line number.
    pub fn row(&self) -> usize {
        self.row
    }

    /// 1-based column, in code points.
    pub fn col(&self) -> usize {
        self.col
    }

    /// Length of the marked region, in code points.
    pub fn length(&self) -> usize {
        self.length
    }

    /// Makes another span over the same resource.
    pub fn span(&self, row: usize, col: usize, length: usize) -> Self {
        Self {
            uri: self.uri.clone(),
            source: self.source.clone(),
            row,
            col,
            length,
        }
    }

    /// Makes a span starting right after this one, on the same row.
    pub fn after(&self, length: usize) -> Self {
        self.span(self.row, self.col.saturating_add(self.length), length)
    }

    /// Makes a span over a part of this one.
    ///
    /// Negative indices count from the end of this span. Note that the
    /// reference's default `end` is `-1`, which drops the last code point —
    /// see [`Span::slice_from`].
    ///
    /// # Errors
    ///
    /// Fails when an index falls outside this span, or when `end` precedes
    /// `begin`.
    pub fn slice(&self, begin: isize, end: isize) -> Result<Self, Error> {
        let len = self.length as isize;

        let begin = if begin < 0 {
            begin.saturating_add(len)
        } else {
            begin
        };
        let end = if end < 0 {
            end.saturating_add(len)
        } else {
            end
        };

        if begin < 0 || begin > len {
            return Err(self.error(format_args!("Begin value '{begin}' out of range")));
        }
        if end < 0 || end > len {
            return Err(self.error(format_args!("End value '{end}' out of range")));
        }
        if end < begin {
            return Err(self.error(format_args!(
                "End value '{end}' can't be less than begin value"
            )));
        }

        Ok(self.span(
            self.row,
            self.col.saturating_add(begin as usize),
            (end - begin) as usize,
        ))
    }

    /// [`Span::slice`] with the reference's default end of `-1`.
    ///
    /// # Errors
    ///
    /// As [`Span::slice`].
    pub fn slice_from(&self, begin: isize) -> Result<Self, Error> {
        self.slice(begin, -1)
    }

    /// Makes an error over this span, rendered as `{message} ({span})`.
    pub fn error(&self, message: impl fmt::Display) -> Error {
        Error::new(format!("{message} ({self})"), self.clone())
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}#{}:{}/{}", self.uri, self.row, self.col, self.length)
    }
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Span({self})")
    }
}
