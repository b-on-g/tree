//! Rendering a tree back into the tree format.

use std::fmt::{self, Write as _};

use crate::tree::Tree;

/// One unit of work for the serializer. The dump is iterative so that a deeply
/// nested tree cannot overflow the stack.
enum Job<'a> {
    /// Render this node, indenting its children by `prefix` tabs.
    Dump(&'a Tree, usize),
    /// Emit that many tabs.
    Tabs(usize),
}

impl fmt::Display for Tree {
    /// Serializes to the tree format.
    ///
    /// A struct node with exactly one child collapses onto one line, which is
    /// what turns nesting back into `a b c d`.
    ///
    /// ```
    /// let tree = tree2::parse("a\n\tb\n\t\tc\n", "?")?;
    /// assert_eq!(tree.to_string(), "a b c\n");
    /// # Ok::<_, tree2::SyntaxError>(())
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut jobs = vec![Job::Dump(self, 0)];

        while let Some(job) = jobs.pop() {
            let (node, mut prefix) = match job {
                Job::Tabs(count) => {
                    for _ in 0..count {
                        f.write_char('\t')?;
                    }
                    continue;
                }
                Job::Dump(node, prefix) => (node, prefix),
            };

            if !node.kind().is_empty() {
                if prefix == 0 {
                    prefix = 1;
                }

                f.write_str(node.kind())?;

                if let [only] = node.kids() {
                    // Stays on this line, at the same indent.
                    f.write_char(' ')?;
                    jobs.push(Job::Dump(only, prefix));
                    continue;
                }

                f.write_char('\n')?;
            } else if !node.value().is_empty() || prefix > 0 {
                f.write_char('\\')?;
                f.write_str(node.value())?;
                f.write_char('\n')?;
            }

            for kid in node.kids().iter().rev() {
                jobs.push(Job::Dump(kid, prefix + 1));
                jobs.push(Job::Tabs(prefix));
            }
        }

        Ok(())
    }
}
