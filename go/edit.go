package tree2

// Update replaces whatever sits at the path with values and returns the new
// siblings at this node's own level — a single node in every case but the
// empty path, which returns values unchanged.
//
// A missing path is created when values is non-empty: a type step makes a
// struct node of that type, an index or wildcard step makes an empty list.
// Updating a missing path to nothing leaves the tree alone.
//
// The error is non-nil only when a type step would have to create a struct
// node whose type cannot be serialized; see Struct.
func (t *Tree) Update(values []*Tree, path ...Step) ([]*Tree, error) {

	if len(path) == 0 {
		return cloneKids(values), nil
	}

	step, rest := path[0], path[1:]

	switch step.kind {

	case stepType:

		replaced := false
		sub := make([]*Tree, 0, len(t.kids))

		for _, kid := range t.kids {
			if kid.typ != step.typ {
				sub = append(sub, kid)
				continue
			}
			replaced = true
			next, err := kid.Update(values, rest...)
			if err != nil {
				return nil, err
			}
			sub = append(sub, next...)
		}

		if !replaced && len(values) > 0 {
			fresh, err := t.Struct(step.typ, nil)
			if err != nil {
				return nil, err
			}
			next, err := fresh.Update(values, rest...)
			if err != nil {
				return nil, err
			}
			sub = append(sub, next...)
		}

		return []*Tree{t.cloneOwned(sub)}, nil

	case stepIndex:

		// An index outside the children addresses nothing, so a fresh empty
		// list stands in and is spliced at the nearest valid position.
		at, tail := step.index, step.index+1
		target := t.Kid(at)
		if target == nil {
			target = t.List(nil)
			at = min(max(at, 0), len(t.kids))
			tail = at
		}

		next, err := target.Update(values, rest...)
		if err != nil {
			return nil, err
		}

		sub := make([]*Tree, 0, len(t.kids)+len(next))
		sub = append(sub, t.kids[:at]...)
		sub = append(sub, next...)
		sub = append(sub, t.kids[tail:]...)

		return []*Tree{t.cloneOwned(sub)}, nil

	default:

		kids := t.kids
		if len(kids) == 0 {
			kids = []*Tree{t.List(nil)}
		}

		sub := make([]*Tree, 0, len(kids))
		for _, kid := range kids {
			next, err := kid.Update(values, rest...)
			if err != nil {
				return nil, err
			}
			sub = append(sub, next...)
		}

		return []*Tree{t.cloneOwned(sub)}, nil
	}
}

// Insert puts a single node at the path, or removes whatever is there when
// value is nil, and returns the new tree.
//
// It returns nil when the update leaves nothing at this level at all, which
// only happens for Insert(nil) with an empty path.
func (t *Tree) Insert(value *Tree, path ...Step) (*Tree, error) {

	var values []*Tree
	if value != nil {
		values = []*Tree{value}
	}

	next, err := t.Update(values, path...)
	if err != nil {
		return nil, err
	}
	if len(next) == 0 {
		return nil, nil
	}

	return next[0], nil
}
