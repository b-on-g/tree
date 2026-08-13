package tree2

import "fmt"

// Handler rewrites one node into any number of nodes, splicing its output in
// place of the input.
type Handler func(input *Tree, belt Belt, ctx *HackContext) ([]*Tree, error)

// Belt maps a node type to the handler rewriting it. The handler stored under
// "" catches every type without one of its own; types without any handler at
// all are kept and their children rewritten.
type Belt map[string]Handler

// HackContext carries whatever the handlers of one pass need.
type HackContext struct {
	// Span, when set, replaces the span of every node rebuilt by the default
	// pass-through handler.
	Span *Span
	// Data is free-form payload for the handlers themselves.
	Data map[string]any
}

// Hack rewrites the children of this node through the belt and returns them.
func (t *Tree) Hack(belt Belt, ctx *HackContext) ([]*Tree, error) {

	out := make([]*Tree, 0, len(t.kids))

	for _, kid := range t.kids {
		next, err := kid.HackSelf(belt, ctx)
		if err != nil {
			return nil, err
		}
		out = append(out, next...)
	}

	return out, nil
}

// HackSelf rewrites this node through the belt: the handler for its type, else
// the one under "", else a pass-through that keeps the node and rewrites its
// children. Errors are wrapped with the node and its position.
func (t *Tree) HackSelf(belt Belt, ctx *HackContext) ([]*Tree, error) {

	handle, found := belt[t.typ]
	if !found || handle == nil {
		handle, found = belt[""]
	}

	if !found || handle == nil {
		kids, err := t.Hack(belt, ctx)
		if err != nil {
			return nil, t.hackError(err)
		}
		span := t.span
		if ctx != nil && ctx.Span != nil {
			span = *ctx.Span
		}
		return []*Tree{t.CloneAt(kids, span)}, nil
	}

	out, err := handle(t, belt, ctx)
	if err != nil {
		return nil, t.hackError(err)
	}

	return out, nil
}

func (t *Tree) hackError(err error) error {
	return fmt.Errorf("%w\n%s%s", err, t.Clone(nil), t.span)
}
