package tree2

import "strings"

// Parse reads the tree format. It returns a list node spanning the whole
// source, whose children are the top-level nodes of the document.
//
// Every line, including the last, must end with a newline. On malformed input
// Parse returns a *SyntaxError and never panics.
//
// The scan is byte-wise: every delimiter of the format is ASCII, so only the
// column counter has to care about UTF-8, which it does by skipping
// continuation bytes.
func Parse(src, uri string) (*Tree, error) {
	span := SpanEntire(uri, src)

	// Nodes are appended to in place while the document is being read. That is
	// the one place a tree is mutable, and nothing outside this function ever
	// sees a node before it is finished.
	root := newTree("", "", nil, span)
	stack := []*Tree{root}

	pos, row, minIndent := 0, 0, 0

	for pos < len(src) {

		indent := 0
		lineStart := pos
		// col is the 1-based column of pos inside the current line.
		col := 1
		row++

		for pos < len(src) && src[pos] == '\t' {
			indent++
			pos++
			col++
		}

		// The indent of the first line that produces a node becomes the base
		// indent of the whole document.
		if len(root.kids) == 0 {
			minIndent = indent
		}
		indent -= minIndent

		if indent < 0 || indent >= len(stack) {

			sp := span.Span(row, 1, pos-lineStart)

			// Report the offending line as a whole.
			for pos < len(src) && src[pos] != '\n' {
				pos++
			}

			reason := "Too many tabs"
			if indent < 0 {
				// Raised even when the offending line is the unterminated last
				// one, so that appending a newline to a source never changes
				// which error it reports. The reference suppresses it there
				// and then either drops the line or crashes; see SPEC.md,
				// Known reference bugs.
				reason = "Too few tabs"
			}

			return nil, &SyntaxError{Reason: reason, Line: src[lineStart:pos], Span: sp}
		}

		stack = stack[:indent+1]
		parent := stack[indent]

		// struct nodes
		for pos < len(src) && src[pos] != '\\' && src[pos] != '\n' {

			// A single space separates two nodes; anything else is an error.
			sepStart, sepCol := pos, col
			for pos < len(src) && (src[pos] == ' ' || src[pos] == '\t') {
				pos++
				col++
			}

			if pos > sepStart {
				lineEnd := strings.IndexByte(src[pos:], '\n')
				if lineEnd < 0 {
					lineEnd = len(src)
				} else {
					lineEnd += pos
				}
				return nil, &SyntaxError{
					Reason: "Wrong nodes separator",
					Line:   src[lineStart:lineEnd],
					Span:   span.Span(row, sepCol, pos-sepStart),
				}
			}

			typeStart, typeCol := pos, col
			for pos < len(src) && src[pos] != '\\' && src[pos] != ' ' && src[pos] != '\t' && src[pos] != '\n' {
				if src[pos]&0xC0 != 0x80 {
					col++
				}
				pos++
			}

			if pos > typeStart {
				kid := newTree(
					src[typeStart:pos],
					"",
					nil,
					span.Span(row, typeCol, col-typeCol),
				)
				parent.kids = append(parent.kids, kid)
				parent = kid
			}

			if pos < len(src) && src[pos] == ' ' {
				pos++
				col++
			}
		}

		// data node — runs to the end of the line
		if pos < len(src) && src[pos] == '\\' {
			dataStart, dataCol := pos, col
			for pos < len(src) && src[pos] != '\n' {
				if src[pos]&0xC0 != 0x80 {
					col++
				}
				pos++
			}
			kid := newTree(
				"",
				src[dataStart+1:pos],
				nil,
				span.Span(row, dataCol+1, col-dataCol-1),
			)
			parent.kids = append(parent.kids, kid)
			parent = kid
		}

		if pos == len(src) {
			return nil, &SyntaxError{
				Reason: "Unexpected EOF, LF required",
				Line:   src[lineStart:],
				Span:   span.Span(row, col, 1),
			}
		}

		stack = append(stack, parent)
		pos++
	}

	return root, nil
}
