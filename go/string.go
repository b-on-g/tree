package tree2

import "strings"

// String serializes the node back to the tree format.
//
// A struct node with exactly one child collapses onto one line, recursively,
// which is what turns nested nodes back into `a b c d`. A list emits nothing of
// itself, only its children, so an empty list serializes to an empty string.
//
// Serialization is a fixed point of parsing: Parse(x.String()).String() equals
// x.String() for every x that Parse produced.
func (t *Tree) String() string {
	if t == nil {
		return ""
	}
	var out strings.Builder
	dump(&out, t, "")
	return out.String()
}

func dump(out *strings.Builder, t *Tree, prefix string) {

	if t.typ != "" {

		if prefix == "" {
			prefix = "\t"
		}

		out.WriteString(t.typ)

		if len(t.kids) == 1 {
			out.WriteByte(' ')
			dump(out, t.kids[0], prefix) // same prefix — stays on this line
			return
		}

		out.WriteByte('\n')

	} else if t.value != "" || prefix != "" {

		out.WriteByte('\\')
		out.WriteString(t.value)
		out.WriteByte('\n')

	}

	for _, kid := range t.kids {
		out.WriteString(prefix)
		dump(out, kid, prefix+"\t")
	}
}
