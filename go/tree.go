// Package tree2 implements the `tree` format — a whitespace-significant
// structural format with three lexical ingredients (tab, space, backslash) and
// no escaping rules at all.
//
//	server
//		host \0.0.0.0
//		port \8080
//		greeting
//			\Hello,
//			\world!
//
// It is a port of $mol_tree2; see SPEC.md in the repository root for the
// language-neutral contract both follow.
//
// Trees are immutable: every operation returns a new node and shares the
// untouched subtrees with the old one.
package tree2

import (
	"bytes"
	"encoding/json"
	"strings"
)

// Tree is a node of an abstract syntax tree.
//
// A node is either a struct node (non-empty type, empty value), a data node
// (non-empty value, empty type) or a list — an anonymous container with both
// empty. Nodes are immutable; build them with List, Data, Struct and Clone.
//
// The zero Tree is a valid empty list with an unset span, but prefer List for
// clarity.
type Tree struct {
	typ   string
	value string
	kids  []*Tree
	span  Span
}

// newTree builds a node taking ownership of kids. Callers must never hand the
// same slice to anyone else afterwards.
func newTree(typ, value string, kids []*Tree, span Span) *Tree {
	return &Tree{typ: typ, value: value, kids: kids, span: span}
}

func cloneKids(kids []*Tree) []*Tree {
	if len(kids) == 0 {
		return nil
	}
	out := make([]*Tree, len(kids))
	copy(out, kids)
	return out
}

// List makes a list node — a container with neither type nor value.
func List(kids []*Tree, span Span) *Tree {
	return newTree("", "", cloneKids(kids), span)
}

// Data makes a data node holding value.
//
// A multi-line value is split: the resulting node carries an empty value and
// one data child per line, followed by kids. Each line gets its own span,
// walked from the node's own position.
func Data(value string, kids []*Tree, span Span) *Tree {
	if !strings.Contains(value, "\n") {
		return newTree("", value, cloneKids(kids), span)
	}

	chunks := strings.Split(value, "\n")
	sub := make([]*Tree, 0, len(chunks)+len(kids))

	kidSpan := span.Span(span.Row, span.Col, 0)
	for _, chunk := range chunks {
		kidSpan = kidSpan.After(runeLen(chunk))
		sub = append(sub, newTree("", chunk, nil, kidSpan))
	}
	sub = append(sub, kids...)

	return newTree("", "", sub, span)
}

// Struct makes a struct node of the given type. The type must not contain a
// space, tab, newline or backslash, because such a node could not be
// serialized back.
func Struct(typ string, kids []*Tree, span Span) (*Tree, error) {
	if strings.ContainsAny(typ, " \n\t\\") {
		return nil, span.Error("Wrong type " + quote(typ))
	}
	return newTree(typ, "", cloneKids(kids), span), nil
}

// List makes a list node at this node's position.
func (t *Tree) List(kids []*Tree) *Tree {
	return List(kids, t.span)
}

// Data makes a data node at this node's position.
func (t *Tree) Data(value string, kids []*Tree) *Tree {
	return Data(value, kids, t.span)
}

// Struct makes a struct node at this node's position.
func (t *Tree) Struct(typ string, kids []*Tree) (*Tree, error) {
	return Struct(typ, kids, t.span)
}

// Clone makes a copy of this node with different children, keeping its type,
// value and span.
func (t *Tree) Clone(kids []*Tree) *Tree {
	return t.CloneAt(kids, t.span)
}

// CloneAt makes a copy of this node with different children and a different
// span, keeping its type and value.
func (t *Tree) CloneAt(kids []*Tree, span Span) *Tree {
	return newTree(t.typ, t.value, cloneKids(kids), span)
}

// cloneOwned is Clone for a freshly built slice nobody else holds.
func (t *Tree) cloneOwned(kids []*Tree) *Tree {
	return newTree(t.typ, t.value, kids, t.span)
}

// Type returns the type of a struct node, or "" for data nodes and lists.
func (t *Tree) Type() string { return t.typ }

// Value returns the content of a data node, or "" for struct nodes and lists.
func (t *Tree) Value() string { return t.value }

// Span returns the position of this node in its source.
func (t *Tree) Span() Span { return t.span }

// KidCount returns the number of children.
func (t *Tree) KidCount() int { return len(t.kids) }

// Kid returns the child at index i, or nil when there is none. Use it together
// with KidCount to walk a tree without copying anything.
func (t *Tree) Kid(i int) *Tree {
	if i < 0 || i >= len(t.kids) {
		return nil
	}
	return t.kids[i]
}

// Kids returns the children as a fresh slice, so that the caller cannot
// disturb the tree. Prefer Kid and KidCount in hot paths.
func (t *Tree) Kids() []*Tree { return cloneKids(t.kids) }

// Text returns the multi-line content: this node's value followed by the
// values of its data children, joined by newlines. Struct children are skipped
// and never descended into.
func (t *Tree) Text() string {
	var out strings.Builder
	out.WriteString(t.value)

	first := true
	for _, kid := range t.kids {
		if kid.typ != "" {
			continue
		}
		if !first {
			out.WriteByte('\n')
		}
		first = false
		out.WriteString(kid.value)
	}

	return out.String()
}

// Error makes an error pointing at this node, quoting the node itself without
// its children.
func (t *Tree) Error(message string) error {
	return t.span.Error(message + "\n" + t.Clone(nil).String())
}

// quote renders a string the way JSON.stringify does, since that is what the
// reference puts into "Wrong type" messages.
func quote(str string) string {
	var buf bytes.Buffer
	enc := json.NewEncoder(&buf)
	enc.SetEscapeHTML(false)
	if err := enc.Encode(str); err != nil {
		return `"` + str + `"`
	}
	return strings.TrimSuffix(buf.String(), "\n")
}
