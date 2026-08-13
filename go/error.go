package tree2

import (
	"strings"
	"unicode"
)

// SyntaxError reports malformed source. Parse returns it and nothing else.
type SyntaxError struct {
	// Reason is the short description, e.g. "Wrong nodes separator".
	Reason string
	// Line is the offending source line, without its terminator.
	Line string
	// Span marks the offending region inside that line.
	Span Span
}

// Error renders the four-line report: the reason, the span, a marker line and
// the source line itself.
//
//	Wrong nodes separator
//	test#1:5/1
//	    !
//	foo  bar
func (e *SyntaxError) Error() string {
	var out strings.Builder

	out.WriteString(e.Reason)
	out.WriteByte('\n')
	out.WriteString(e.Span.String())
	out.WriteByte('\n')

	// Everything before the span becomes blank, so the markers line up with it
	// even when the line is indented with tabs.
	left := 0
	for _, char := range e.Line {
		if left >= e.Span.Col-1 {
			break
		}
		left++
		if unicode.IsSpace(char) {
			out.WriteRune(char)
		} else {
			out.WriteByte(' ')
		}
	}

	if e.Span.Length > 0 {
		out.WriteString(strings.Repeat("!", e.Span.Length))
	}
	out.WriteByte('\n')
	out.WriteString(e.Line)

	return out.String()
}
