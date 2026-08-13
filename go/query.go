package tree2

import "strconv"

// Step is one step of a path through a tree: a type to match, a child index,
// or a wildcard matching every child. The zero Step is the wildcard.
type Step struct {
	kind  stepKind
	typ   string
	index int
}

type stepKind uint8

const (
	stepAny stepKind = iota
	stepType
	stepIndex
)

// Any makes a step matching every child.
func Any() Step { return Step{kind: stepAny} }

// ByType makes a step matching every child of the given type.
func ByType(typ string) Step { return Step{kind: stepType, typ: typ} }

// ByIndex makes a step matching the child at the given index. An index outside
// the children matches nothing.
func ByIndex(index int) Step { return Step{kind: stepIndex, index: index} }

// String renders the step for diagnostics.
func (s Step) String() string {
	switch s.kind {
	case stepType:
		return quote(s.typ)
	case stepIndex:
		return strconv.Itoa(s.index)
	default:
		return "*"
	}
}

// Select walks the path and returns a list node holding everything it reached.
// An empty path yields a list holding the receiver itself.
func (t *Tree) Select(path ...Step) *Tree {

	next := []*Tree{t}

	for _, step := range path {

		if len(next) == 0 {
			break
		}

		prev := next
		next = nil

		for _, item := range prev {
			switch step.kind {

			case stepType:
				for _, kid := range item.kids {
					if kid.typ == step.typ {
						next = append(next, kid)
					}
				}

			case stepIndex:
				if kid := item.Kid(step.index); kid != nil {
					next = append(next, kid)
				}

			default:
				next = append(next, item.kids...)
			}
		}
	}

	return newTree("", "", next, t.span)
}

// Filter keeps the children the path reaches something from, and returns a
// clone of this node holding them.
func (t *Tree) Filter(path ...Step) *Tree {
	return t.filter(path, nil)
}

// FilterValue keeps the children the path reaches a node with that value from,
// and returns a clone of this node holding them.
func (t *Tree) FilterValue(value string, path ...Step) *Tree {
	return t.filter(path, &value)
}

func (t *Tree) filter(path []Step, value *string) *Tree {

	sub := make([]*Tree, 0, len(t.kids))

	for _, kid := range t.kids {
		found := kid.Select(path...)

		if value == nil {
			if len(found.kids) > 0 {
				sub = append(sub, kid)
			}
			continue
		}

		for _, child := range found.kids {
			if child.value == *value {
				sub = append(sub, kid)
				break
			}
		}
	}

	return t.cloneOwned(sub)
}
