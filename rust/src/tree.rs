//! The tree node and everything you can do with one.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use crate::error::Error;
use crate::span::Span;

/// A node of the tree.
///
/// A node is either a **struct** node, which carries a `kind` (the format's
/// `type`) and no value, a **data** node, which carries a value and no kind, or
/// a **list** — an anonymous container with neither. The parse root is always a
/// list.
///
/// Nodes are immutable: every operation returns a new node, sharing the
/// children it did not touch. Cloning one is a single atomic increment.
///
/// ```
/// let tree = tree2::parse("server\n\tport \\8080\n", "conf")?;
/// let port = tree.select(&["server".into(), "port".into()]);
/// assert_eq!(port.kids()[0].text(), "8080");
/// # Ok::<_, tree2::SyntaxError>(())
/// ```
#[derive(Clone)]
pub struct Tree(Arc<Node>);

struct Node {
    kind: Arc<str>,
    value: Arc<str>,
    kids: Box<[Tree]>,
    span: Span,
}

impl Drop for Node {
    /// Dismantles the subtree iteratively. Letting the children drop
    /// themselves would recurse once per level, and a document nested a few
    /// thousand levels deep would take the stack with it.
    fn drop(&mut self) {
        let mut pending: Vec<Tree> = std::mem::take(&mut self.kids).into_vec();

        while let Some(tree) = pending.pop() {
            // `None` when someone else still holds the node.
            if let Some(mut node) = Arc::into_inner(tree.0) {
                pending.extend(std::mem::take(&mut node.kids));
                // Dropping `node` here re-enters this method, but it has no
                // children left to walk.
            }
        }
    }
}

/// One step of a path through the tree, as taken by [`Tree::select`],
/// [`Tree::filter`], [`Tree::update`] and [`Tree::insert`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step<'a> {
    /// Every child of that kind.
    Kind(&'a str),
    /// The child at that index. Anything outside `0 .. kids.len()` matches
    /// nothing.
    Index(isize),
    /// Every child.
    All,
}

impl<'a> From<&'a str> for Step<'a> {
    fn from(kind: &'a str) -> Self {
        Step::Kind(kind)
    }
}

impl<'a> From<&'a String> for Step<'a> {
    fn from(kind: &'a String) -> Self {
        Step::Kind(kind)
    }
}

impl From<isize> for Step<'_> {
    fn from(index: isize) -> Self {
        Step::Index(index)
    }
}

impl From<i32> for Step<'_> {
    fn from(index: i32) -> Self {
        Step::Index(index as isize)
    }
}

impl From<usize> for Step<'_> {
    fn from(index: usize) -> Self {
        Step::Index(index as isize)
    }
}

impl Tree {
    /// The raw constructor. Private, because it validates nothing.
    pub(crate) fn raw(kind: Arc<str>, value: Arc<str>, kids: Box<[Tree]>, span: Span) -> Self {
        Self(Arc::new(Node {
            kind,
            value,
            kids,
            span,
        }))
    }

    /// Makes a list node — an anonymous container.
    pub fn list(kids: impl Into<Box<[Tree]>>, span: Span) -> Self {
        Self::raw(empty(), empty(), kids.into(), span)
    }

    /// Makes a data node.
    ///
    /// A multi-line value is **split**: the node itself ends up with an empty
    /// value and one data child per line, followed by `kids`.
    ///
    /// ```
    /// let tree = tree2::Tree::data("a\nb", [], tree2::Span::unknown());
    /// assert_eq!(tree.value(), "");
    /// assert_eq!(tree.to_string(), "\\a\n\\b\n");
    /// ```
    pub fn data(value: impl AsRef<str>, kids: impl Into<Box<[Tree]>>, span: Span) -> Self {
        let value = value.as_ref();
        let kids = kids.into();

        if !value.contains('\n') {
            return Self::raw(empty(), Arc::from(value), kids, span);
        }

        let mut line_span = span.span(span.row(), span.col(), 0);
        let mut split = Vec::with_capacity(kids.len() + 1);

        for line in value.split('\n') {
            line_span = line_span.after(line.chars().count());
            split.push(Self::raw(
                empty(),
                Arc::from(line),
                Box::default(),
                line_span.clone(),
            ));
        }

        split.extend(kids.into_vec());

        Self::raw(empty(), empty(), split.into(), span)
    }

    /// Makes a struct node — a node carrying a kind, the format's `type`.
    ///
    /// # Errors
    ///
    /// A kind containing a space, a tab, a newline or a backslash could not be
    /// serialized back, and is rejected as `Wrong type "..."`.
    pub fn structure(
        kind: impl AsRef<str>,
        kids: impl Into<Box<[Tree]>>,
        span: Span,
    ) -> Result<Self, Error> {
        let kind = kind.as_ref();

        if kind.contains([' ', '\n', '\t', '\\']) {
            return Err(span.error(format_args!("Wrong type {}", quote(kind))));
        }

        Ok(Self::raw(Arc::from(kind), empty(), kids.into(), span))
    }

    /// Makes a list node at this node's position.
    pub fn new_list(&self, kids: impl Into<Box<[Tree]>>) -> Self {
        Self::list(kids, self.span().clone())
    }

    /// Makes a data node at this node's position.
    pub fn new_data(&self, value: impl AsRef<str>, kids: impl Into<Box<[Tree]>>) -> Self {
        Self::data(value, kids, self.span().clone())
    }

    /// Makes a struct node at this node's position.
    ///
    /// # Errors
    ///
    /// As [`Tree::structure`].
    pub fn new_structure(
        &self,
        kind: impl AsRef<str>,
        kids: impl Into<Box<[Tree]>>,
    ) -> Result<Self, Error> {
        Self::structure(kind, kids, self.span().clone())
    }

    /// Makes a copy of this node with different children.
    pub fn with_kids(&self, kids: impl Into<Box<[Tree]>>) -> Self {
        Self::raw(
            self.0.kind.clone(),
            self.0.value.clone(),
            kids.into(),
            self.0.span.clone(),
        )
    }

    /// Makes a copy of this node with different children and a different span.
    pub fn with_kids_at(&self, kids: impl Into<Box<[Tree]>>, span: Span) -> Self {
        Self::raw(self.0.kind.clone(), self.0.value.clone(), kids.into(), span)
    }

    /// The kind of a struct node — the format calls it the `type`. Empty for
    /// data nodes and lists.
    pub fn kind(&self) -> &str {
        &self.0.kind
    }

    /// The value of a data node. Empty for struct nodes and lists.
    pub fn value(&self) -> &str {
        &self.0.value
    }

    /// The children, in order.
    pub fn kids(&self) -> &[Tree] {
        &self.0.kids
    }

    /// Where this node came from.
    pub fn span(&self) -> &Span {
        &self.0.span
    }

    /// The multi-line text: this node's value followed by the values of its
    /// data children, joined by newlines. Struct children are skipped.
    ///
    /// ```
    /// let tree = tree2::parse("greeting\n\t\\Hello,\n\t\\world!\n", "?")?;
    /// assert_eq!(tree.kids()[0].text(), "Hello,\nworld!");
    /// # Ok::<_, tree2::SyntaxError>(())
    /// ```
    pub fn text(&self) -> String {
        let mut text = String::from(self.value());
        let mut first = true;

        for kid in self.kids() {
            if !kid.kind().is_empty() {
                continue;
            }
            if !first {
                text.push('\n');
            }
            text.push_str(kid.value());
            first = false;
        }

        text
    }

    /// Collects every node the path leads to into a list node.
    ///
    /// An empty path yields a list holding this node.
    ///
    /// ```
    /// use tree2::Step;
    /// let tree = tree2::parse("a\n\tb x\n\tb y\n", "?")?;
    /// let found = tree.select(&[Step::Kind("a"), Step::Kind("b")]);
    /// assert_eq!(found.to_string(), "b x\nb y\n");
    /// # Ok::<_, tree2::SyntaxError>(())
    /// ```
    pub fn select(&self, path: &[Step<'_>]) -> Self {
        let mut found = vec![self.clone()];

        for step in path {
            if found.is_empty() {
                break;
            }

            let prev = std::mem::take(&mut found);

            for item in &prev {
                match *step {
                    Step::Kind(kind) => {
                        found.extend(item.kids().iter().filter(|kid| kid.kind() == kind).cloned())
                    }
                    Step::Index(index) => {
                        if let Some(kid) = index
                            .try_into()
                            .ok()
                            .and_then(|i: usize| item.kids().get(i))
                        {
                            found.push(kid.clone());
                        }
                    }
                    Step::All => found.extend(item.kids().iter().cloned()),
                }
            }
        }

        self.new_list(found)
    }

    /// Keeps the children the path leads somewhere from.
    ///
    /// With a `value`, keeps those where some node the path leads to carries it.
    pub fn filter(&self, path: &[Step<'_>], value: Option<&str>) -> Self {
        let kids: Vec<Tree> = self
            .kids()
            .iter()
            .filter(|kid| {
                let found = kid.select(path);
                match value {
                    None => !found.kids().is_empty(),
                    Some(value) => found.kids().iter().any(|node| node.value() == value),
                }
            })
            .cloned()
            .collect();

        self.with_kids(kids)
    }

    /// Replaces whatever sits at `path` with `values`, returning the new
    /// siblings at this node's own level.
    ///
    /// An empty path returns `values` unchanged. A path that leads nowhere is
    /// created — but only when `values` is non-empty, so updating to nothing
    /// never materialises the path it was asked to clear.
    ///
    /// ```
    /// use tree2::{Span, Step, Tree};
    /// let tree = tree2::parse("a b c d\n", "?")?;
    /// let x = Tree::structure("x", [], Span::unknown())?;
    /// let updated = tree.update(&[x], &[Step::Kind("a"), Step::Kind("b"), Step::Kind("c")])?;
    /// assert_eq!(updated[0].to_string(), "a b x\n");
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Fails when a missing [`Step::Kind`] has to be created but names a kind
    /// no struct node may carry.
    pub fn update(&self, values: &[Tree], path: &[Step<'_>]) -> Result<Vec<Tree>, Error> {
        let Some((step, rest)) = path.split_first() else {
            return Ok(values.to_vec());
        };

        let kids = match *step {
            Step::Kind(kind) => {
                let mut replaced = false;
                let mut kids = Vec::with_capacity(self.kids().len());

                for kid in self.kids() {
                    if kid.kind() != kind {
                        kids.push(kid.clone());
                        continue;
                    }
                    replaced = true;
                    kids.extend(kid.update(values, rest)?);
                }

                if !replaced && !values.is_empty() {
                    kids.extend(self.new_structure(kind, [])?.update(values, rest)?);
                }

                kids
            }

            Step::Index(index) => {
                // A negative index addresses nothing, as one past the end does.
                let index = usize::try_from(index).unwrap_or(usize::MAX);
                let taken = self.kids().get(index);

                let updated = match taken {
                    Some(kid) => kid.update(values, rest)?,
                    None => self.new_list([]).update(values, rest)?,
                };

                // A missing index has no place of its own: the replacement goes
                // after everything else.
                let split = index.min(self.kids().len());
                let mut kids = Vec::with_capacity(self.kids().len() + updated.len());
                kids.extend_from_slice(&self.kids()[..split]);
                kids.extend(updated);
                if taken.is_some() {
                    kids.extend_from_slice(&self.kids()[split + 1..]);
                }

                kids
            }

            Step::All => {
                let mut kids = Vec::with_capacity(self.kids().len());

                if self.kids().is_empty() {
                    kids.extend(self.new_list([]).update(values, rest)?);
                } else {
                    for kid in self.kids() {
                        kids.extend(kid.update(values, rest)?);
                    }
                }

                kids
            }
        };

        Ok(vec![self.with_kids(kids)])
    }

    /// Puts a single node at `path`, or removes what is there when given
    /// `None`.
    ///
    /// ```
    /// use tree2::{Span, Step, Tree};
    /// let tree = tree2::parse("a b c d\n", "?")?;
    /// assert_eq!(tree.insert(None, &[Step::Kind("a"), Step::Kind("b"), Step::Kind("c")])?.to_string(), "a b\n");
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// As [`Tree::update`]. Also fails when there is nothing to return at all,
    /// which only an empty path with no value can ask for.
    pub fn insert(&self, value: Option<&Tree>, path: &[Step<'_>]) -> Result<Tree, Error> {
        let values: Vec<Tree> = value.into_iter().cloned().collect();

        self.update(&values, path)?
            .into_iter()
            .next()
            .ok_or_else(|| self.span().error("Nothing to insert"))
    }

    /// Rewrites the children of this node through `belt`, splicing each
    /// handler's output in place.
    ///
    /// # Errors
    ///
    /// Whatever a handler raises, annotated with the node it was handling.
    pub fn hack<T>(&self, belt: &Belt<T>, context: &mut Context<T>) -> Result<Vec<Tree>, Error> {
        let mut out = Vec::with_capacity(self.kids().len());

        for kid in self.kids() {
            out.extend(kid.hack_self(belt, context)?);
        }

        Ok(out)
    }

    /// Rewrites this node itself.
    ///
    /// The handler is `belt[kind]`, or `belt[""]`, or — when the belt knows
    /// neither — the identity: keep the node, rewrite its children.
    ///
    /// # Errors
    ///
    /// Whatever the handler raises, annotated with this node and its span.
    pub fn hack_self<T>(
        &self,
        belt: &Belt<T>,
        context: &mut Context<T>,
    ) -> Result<Vec<Tree>, Error> {
        let handle = belt.handler(self.kind()).or_else(|| belt.handler(""));

        let result = match handle {
            Some(handle) => handle(self, belt, context),
            // The identity: keep the node, rewrite the children. It is annotated
            // like any other handler, so an error picks up the whole chain of
            // nodes it came out through.
            None => self.hack(belt, context).map(|kids| {
                let span = context.span.clone().unwrap_or_else(|| self.span().clone());
                vec![self.with_kids_at(kids, span)]
            }),
        };

        result.map_err(|error| {
            error.annotated(format_args!("\n{}{}", self.with_kids([]), self.span()))
        })
    }

    /// Makes an error over this node's position, quoting the node itself.
    pub fn error(&self, message: impl fmt::Display) -> Error {
        self.span()
            .error(format_args!("{message}\n{}", self.with_kids([])))
    }
}

impl PartialEq for Tree {
    /// Structural equality: kind, value and children. Spans are positions, not
    /// identity, and are not compared.
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
            || (self.kind() == other.kind()
                && self.value() == other.value()
                && self.kids() == other.kids())
    }
}

impl Eq for Tree {}

impl fmt::Debug for Tree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tree")
            .field("kind", &self.kind())
            .field("value", &self.value())
            .field("span", &self.span())
            .field("kids", &self.kids())
            .finish()
    }
}

/// The context a rewriting pass carries along.
///
/// `span`, when set, overrides the span of every node the identity handler
/// rebuilds.
#[derive(Clone, Debug, Default)]
pub struct Context<T> {
    /// Position to stamp on rebuilt nodes.
    pub span: Option<Span>,
    /// Whatever the pass needs to carry.
    pub data: T,
}

impl<T> Context<T> {
    /// Makes a context around some data, with no span override.
    pub fn new(data: T) -> Self {
        Self { span: None, data }
    }
}

/// One rewriting rule: takes a node, gives back the nodes that replace it.
pub type Handler<T> = Box<dyn Fn(&Tree, &Belt<T>, &mut Context<T>) -> Result<Vec<Tree>, Error>>;

/// A set of rewriting rules, keyed by the kind of node they handle.
///
/// The rule under `""` handles every kind the belt has no rule for; without it
/// unknown kinds are kept and their children rewritten.
///
/// ```
/// use tree2::{Belt, Context, Tree};
///
/// let belt: Belt<()> = Belt::new().with("b", |node, _belt, _cx| {
///     Ok(vec![node.new_data("replaced", [])])
/// });
///
/// let tree = tree2::parse("a b\n", "?")?;
/// let hacked = tree.hack(&belt, &mut Context::new(()))?;
/// assert_eq!(hacked[0].to_string(), "a \\replaced\n");
/// # Ok::<_, Box<dyn std::error::Error>>(())
/// ```
pub struct Belt<T> {
    handlers: HashMap<String, Handler<T>>,
}

impl<T> Belt<T> {
    /// Makes an empty belt.
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Adds a rule for one kind of node.
    #[must_use]
    pub fn with(
        mut self,
        kind: impl Into<String>,
        handle: impl Fn(&Tree, &Belt<T>, &mut Context<T>) -> Result<Vec<Tree>, Error> + 'static,
    ) -> Self {
        self.handlers.insert(kind.into(), Box::new(handle));
        self
    }

    /// The rule for a kind of node, if there is one.
    pub fn handler(&self, kind: &str) -> Option<&Handler<T>> {
        self.handlers.get(kind)
    }
}

impl<T> Default for Belt<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> fmt::Debug for Belt<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Belt")
            .field("kinds", &self.handlers.keys().collect::<Vec<_>>())
            .finish()
    }
}

/// The one shared empty string, for the half of a node that is always empty.
fn empty() -> Arc<str> {
    Arc::from("")
}

/// Renders a string the way `JSON.stringify` does, for the `Wrong type`
/// message.
fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');

    for char in text.chars() {
        match char {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            char if (char as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", char as u32)),
            char => out.push(char),
        }
    }

    out.push('"');
    out
}
