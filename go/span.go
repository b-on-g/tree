package tree2

import "fmt"

// Span marks a region of a source resource.
//
// Row is 1-based. Col is a 1-based offset inside that row and Length is a
// region size, both measured in Unicode code points — the reference TypeScript
// implementation measures them in UTF-16 code units, which is a JavaScript
// artifact (see SPEC.md §2.3).
//
// A Span is a value: copying it is cheap and copies never alias.
type Span struct {
	// URI names the source resource.
	URI string
	// Source is the full text of that resource.
	Source string
	// Row is the 1-based line number.
	Row int
	// Col is the 1-based column, in code points.
	Col int
	// Length is the size of the marked region, in code points.
	Length int
}

// SpanUnknown marks the beginning of an unknown resource. It is the span of
// nodes built without any source behind them.
var SpanUnknown = SpanBegin("?", "")

// NewSpan makes a span over an explicit region.
func NewSpan(uri, source string, row, col, length int) Span {
	return Span{URI: uri, Source: source, Row: row, Col: col, Length: length}
}

// SpanBegin makes an empty span at the beginning of a resource.
func SpanBegin(uri, source string) Span {
	return Span{URI: uri, Source: source, Row: 1, Col: 1, Length: 0}
}

// SpanEnd makes an empty span just past the end of a resource.
func SpanEnd(uri, source string) Span {
	return Span{URI: uri, Source: source, Row: 1, Col: runeLen(source) + 1, Length: 0}
}

// SpanEntire makes a span covering a whole resource.
func SpanEntire(uri, source string) Span {
	return Span{URI: uri, Source: source, Row: 1, Col: 1, Length: runeLen(source)}
}

// Span makes another span over the same resource.
func (s Span) Span(row, col, length int) Span {
	return Span{URI: s.URI, Source: s.Source, Row: row, Col: col, Length: length}
}

// After makes a span of the given length starting right after this one.
func (s Span) After(length int) Span {
	return s.Span(s.Row, s.Col+s.Length, length)
}

// Slice makes a sub-span of this one. Negative bounds count from the end, so
// Slice(0, -1) drops the last code point — the same default the reference uses.
// The bounds must stay inside the span and end must not precede begin.
func (s Span) Slice(begin, end int) (Span, error) {
	if begin < 0 {
		begin += s.Length
	}
	if end < 0 {
		end += s.Length
	}

	if begin < 0 || begin > s.Length {
		return Span{}, s.Error(fmt.Sprintf("Begin value '%d' out of range", begin))
	}
	if end < 0 || end > s.Length {
		return Span{}, s.Error(fmt.Sprintf("End value '%d' out of range", end))
	}
	if end < begin {
		return Span{}, s.Error(fmt.Sprintf("End value '%d' can't be less than begin value", end))
	}

	return s.Span(s.Row, s.Col+begin, end-begin), nil
}

// Error makes an error pointing at this span.
func (s Span) Error(message string) error {
	return &SpanError{Message: message, Span: s}
}

// String renders the span as `uri#row:col/length`.
func (s Span) String() string {
	return fmt.Sprintf("%s#%d:%d/%d", s.URI, s.Row, s.Col, s.Length)
}

// runeLen counts code points, which is what columns and lengths are measured
// in. Continuation bytes are the ones that do not start a code point.
func runeLen(str string) int {
	count := 0
	for i := 0; i < len(str); i++ {
		if str[i]&0xC0 != 0x80 {
			count++
		}
	}
	return count
}

// SpanError is an error carrying the position it happened at.
type SpanError struct {
	Message string
	Span    Span
}

func (e *SpanError) Error() string {
	return fmt.Sprintf("%s (%s)", e.Message, e.Span)
}
