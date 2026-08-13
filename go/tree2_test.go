package tree2_test

import (
	"errors"
	"math"
	"strings"
	"testing"
	"time"

	tree2 "github.com/b-on-g/tree/go"
)

func TestSpanConstructors(t *testing.T) {
	source := "привет\n"

	if got, want := tree2.SpanBegin("test", source).String(), "test#1:1/0"; got != want {
		t.Errorf("begin = %s, want %s", got, want)
	}
	// seven code points, thirteen bytes
	if got, want := tree2.SpanEnd("test", source).String(), "test#1:8/0"; got != want {
		t.Errorf("end = %s, want %s", got, want)
	}
	if got, want := tree2.SpanEntire("test", source).String(), "test#1:1/7"; got != want {
		t.Errorf("entire = %s, want %s", got, want)
	}
	if got, want := tree2.SpanUnknown.String(), "?#1:1/0"; got != want {
		t.Errorf("unknown = %s, want %s", got, want)
	}
}

func TestSpanOperations(t *testing.T) {
	span := tree2.SpanEntire("test", "0123456789")

	if got, want := span.Span(2, 3, 4).String(), "test#2:3/4"; got != want {
		t.Errorf("span = %s, want %s", got, want)
	}
	if got, want := span.After(2).String(), "test#1:11/2"; got != want {
		t.Errorf("after = %s, want %s", got, want)
	}

	sliced, err := span.Slice(2, 5)
	if err != nil {
		t.Fatalf("slice: %v", err)
	}
	if got, want := sliced.String(), "test#1:3/3"; got != want {
		t.Errorf("slice = %s, want %s", got, want)
	}

	// negative bounds count from the end
	tail, err := span.Slice(-3, -1)
	if err != nil {
		t.Fatalf("slice: %v", err)
	}
	if got, want := tail.String(), "test#1:8/2"; got != want {
		t.Errorf("slice = %s, want %s", got, want)
	}

	for _, item := range []struct {
		name        string
		begin, end  int
		wantMessage string
	}{
		{"begin past end", 11, 11, "Begin value '11' out of range (test#1:1/10)"},
		{"begin below start", -11, 0, "Begin value '-1' out of range (test#1:1/10)"},
		{"end past end", 0, 11, "End value '11' out of range (test#1:1/10)"},
		{"end before begin", 5, 2, "End value '2' can't be less than begin value (test#1:1/10)"},
	} {
		t.Run(item.name, func(t *testing.T) {
			if _, err := span.Slice(item.begin, item.end); err == nil || err.Error() != item.wantMessage {
				t.Fatalf("error = %v, want %s", err, item.wantMessage)
			}
		})
	}
}

func TestSpanError(t *testing.T) {
	err := tree2.SpanBegin("test", "").Error("Something happened")

	if got, want := err.Error(), "Something happened (test#1:1/0)"; got != want {
		t.Fatalf("error = %q, want %q", got, want)
	}

	var spanned *tree2.SpanError
	if !errors.As(err, &spanned) {
		t.Fatalf("error is %T, want *tree2.SpanError", err)
	}
	if spanned.Span.URI != "test" {
		t.Fatalf("span uri = %q, want %q", spanned.Span.URI, "test")
	}
}

func TestStructValidation(t *testing.T) {
	for _, typ := range []string{"a b", "a\tb", "a\nb", "a\\b", " ", "\\"} {
		if _, err := tree2.Struct(typ, nil, tree2.SpanUnknown); err == nil {
			t.Errorf("struct %q was accepted", typ)
		}
	}

	_, err := tree2.Struct("a b", nil, tree2.SpanUnknown)
	if got, want := err.Error(), `Wrong type "a b" (?#1:1/0)`; got != want {
		t.Fatalf("error = %q, want %q", got, want)
	}

	if _, err := tree2.Struct("a.b-c/d:e", nil, tree2.SpanUnknown); err != nil {
		t.Fatalf("struct: %v", err)
	}
}

func TestDataSplitSpans(t *testing.T) {
	node := tree2.Data("ab\ncde", nil, tree2.SpanBegin("test", ""))

	if node.Value() != "" {
		t.Errorf("value = %q, want empty", node.Value())
	}
	if got, want := node.String(), "\\ab\n\\cde\n"; got != want {
		t.Errorf("serialized %q, want %q", got, want)
	}
	if got, want := node.Kid(0).Span().String(), "test#1:1/2"; got != want {
		t.Errorf("first line span = %s, want %s", got, want)
	}
	if got, want := node.Kid(1).Span().String(), "test#1:3/3"; got != want {
		t.Errorf("second line span = %s, want %s", got, want)
	}

	// lines are measured in code points
	astral := tree2.Data("😀\nx", nil, tree2.SpanBegin("test", ""))
	if got, want := astral.Kid(1).Span().String(), "test#1:2/1"; got != want {
		t.Errorf("second line span = %s, want %s", got, want)
	}

	// a single-line value is kept as is
	plain := tree2.Data("ab", nil, tree2.SpanUnknown)
	if plain.Value() != "ab" || plain.KidCount() != 0 {
		t.Errorf("plain data = %q with %d kids", plain.Value(), plain.KidCount())
	}
}

func TestDataKeepsExtraKids(t *testing.T) {
	kid := mustStruct(t, "x")
	node := tree2.Data("a\nb", []*tree2.Tree{kid}, tree2.SpanUnknown)

	if got, want := node.String(), "\\a\n\\b\nx\n"; got != want {
		t.Fatalf("serialized %q, want %q", got, want)
	}
	if node.Kid(2) != kid {
		t.Fatal("extra kid was copied instead of shared")
	}
}

func TestTreeError(t *testing.T) {
	root := mustParse(t, "a b\n", "test")

	err := root.Kid(0).Error("Bad node")
	if got, want := err.Error(), "Bad node\na\n (test#1:1/1)"; got != want {
		t.Fatalf("error = %q, want %q", got, want)
	}
}

func TestKidsAreCopied(t *testing.T) {
	root := mustParse(t, "a\n\tb\n\tc\n", "test")

	kids := root.Kid(0).Kids()
	kids[0] = mustStruct(t, "z")

	if got, want := root.String(), "a\n\tb\n\tc\n"; got != want {
		t.Fatalf("tree changed to %q", got)
	}
	if root.Kid(0).Kid(2) != nil {
		t.Fatal("Kid past the end must be nil")
	}
}

func TestEditsShareUntouchedSubtrees(t *testing.T) {
	root := mustParse(t, "a\n\tb x\n\tc y\n", "test")

	updated, err := root.Insert(mustStruct(t, "z"), tree2.ByType("a"), tree2.ByType("b"), tree2.ByType("x"))
	if err != nil {
		t.Fatalf("insert: %v", err)
	}

	if got, want := updated.String(), "a\n\tb z\n\tc y\n"; got != want {
		t.Fatalf("updated %q, want %q", got, want)
	}
	if got, want := root.String(), "a\n\tb x\n\tc y\n"; got != want {
		t.Fatalf("original changed to %q", got)
	}
	if updated.Kid(0).Kid(1) != root.Kid(0).Kid(1) {
		t.Fatal("untouched subtree was rebuilt instead of shared")
	}
}

func TestInsertNothingAtRoot(t *testing.T) {
	root := mustParse(t, "a b\n", "test")

	got, err := root.Insert(nil)
	if err != nil {
		t.Fatalf("insert: %v", err)
	}
	if got != nil {
		t.Fatalf("insert = %q, want nil", got)
	}
	if got.String() != "" {
		t.Fatalf("nil tree serialized to %q", got)
	}
}

func TestUpdateIndexOutsideKids(t *testing.T) {
	root := mustParse(t, "a\n\tb\n\tc\n", "test")

	for _, item := range []struct {
		name  string
		index int
		want  string
	}{
		{"past the end appends", 5, "a\n\tb\n\tc\n\tz\n"},
		{"negative prepends", -1, "a\n\tz\n\tb\n\tc\n"},
		{"inside replaces", 0, "a\n\tz\n\tc\n"},
	} {
		t.Run(item.name, func(t *testing.T) {

			got, err := root.Insert(mustStruct(t, "z"), tree2.ByType("a"), tree2.ByIndex(item.index))
			if err != nil {
				t.Fatalf("insert: %v", err)
			}
			if got.String() != item.want {
				t.Fatalf("inserted %q, want %q", got, item.want)
			}
		})
	}
}

func TestUpdateRejectsUnserializableType(t *testing.T) {
	root := mustParse(t, "a b\n", "test")

	_, err := root.Insert(mustStruct(t, "z"), tree2.ByType("a"), tree2.ByType("bad type"))
	if err == nil {
		t.Fatal("a type with a space was accepted")
	}
	if !strings.Contains(err.Error(), "Wrong type") {
		t.Fatalf("error = %q, want a Wrong type report", err)
	}
}

func TestSelectStepKinds(t *testing.T) {
	root := mustParse(t, "a\n\tb\n\tc\n", "test")

	if got, want := root.Select().String(), "\\\n\ta\n\t\tb\n\t\tc\n"; got != want {
		t.Errorf("empty path selected %q, want %q", got, want)
	}
	if got, want := root.Select(tree2.ByType("a"), tree2.Any()).String(), "b\nc\n"; got != want {
		t.Errorf("wildcard selected %q, want %q", got, want)
	}
	if got, want := root.Select(tree2.ByType("a"), tree2.ByIndex(1)).String(), "c\n"; got != want {
		t.Errorf("index selected %q, want %q", got, want)
	}

	// the zero Step is the wildcard
	if got, want := root.Select(tree2.ByType("a"), tree2.Step{}).String(), "b\nc\n"; got != want {
		t.Errorf("zero step selected %q, want %q", got, want)
	}
	if got, want := tree2.ByType("a").String(), `"a"`; got != want {
		t.Errorf("step = %s, want %s", got, want)
	}
}

func TestHack(t *testing.T) {
	root := mustParse(t, "list\n\tgroup\n\t\ta\n\t\tb\n\tc\n", "test")

	belt := tree2.Belt{
		// splice the children of every group in place of it
		"group": func(input *tree2.Tree, belt tree2.Belt, ctx *tree2.HackContext) ([]*tree2.Tree, error) {
			return input.Hack(belt, ctx)
		},
	}

	got, err := root.Hack(belt, nil)
	if err != nil {
		t.Fatalf("hack: %v", err)
	}
	if len(got) != 1 {
		t.Fatalf("hack returned %d nodes, want 1", len(got))
	}
	if want := "list\n\ta\n\tb\n\tc\n"; got[0].String() != want {
		t.Fatalf("hacked %q, want %q", got[0], want)
	}
}

func TestHackFallbackAndContextSpan(t *testing.T) {
	root := mustParse(t, "a b\n", "test")

	span := tree2.SpanBegin("other", "")
	got, err := root.Hack(tree2.Belt{}, &tree2.HackContext{Span: &span})
	if err != nil {
		t.Fatalf("hack: %v", err)
	}
	if got[0].Span().String() != span.String() {
		t.Fatalf("span = %s, want %s", got[0].Span(), span)
	}

	// a handler under "" catches everything without one of its own
	catchAll := tree2.Belt{
		"": func(input *tree2.Tree, belt tree2.Belt, ctx *tree2.HackContext) ([]*tree2.Tree, error) {
			return nil, nil
		},
	}
	dropped, err := root.Hack(catchAll, nil)
	if err != nil {
		t.Fatalf("hack: %v", err)
	}
	if len(dropped) != 0 {
		t.Fatalf("hack returned %d nodes, want none", len(dropped))
	}
}

func TestHackWrapsErrors(t *testing.T) {
	root := mustParse(t, "a bad\n", "test")
	boom := errors.New("boom")

	belt := tree2.Belt{
		"bad": func(input *tree2.Tree, belt tree2.Belt, ctx *tree2.HackContext) ([]*tree2.Tree, error) {
			return nil, boom
		},
	}

	_, err := root.Hack(belt, nil)
	if err == nil {
		t.Fatal("hack succeeded, want the handler error")
	}
	if !errors.Is(err, boom) {
		t.Fatalf("error %q does not wrap the handler error", err)
	}
	if want := "boom\nbad\ntest#1:3/3"; !strings.Contains(err.Error(), want) {
		t.Fatalf("error = %q, want it to contain %q", err, want)
	}
}

func TestParseKeepsCarriageReturn(t *testing.T) {
	root := mustParse(t, "a\r\n", "test")

	if got, want := root.Kid(0).Type(), "a\r"; got != want {
		t.Fatalf("type = %q, want %q", got, want)
	}
}

func TestParseTooFewTabsIgnoresTheFinalNewline(t *testing.T) {
	// Appending a newline to a source must not change which error it reports.
	// The reference suppresses "Too few tabs" on an unterminated last line and
	// then drops the line or crashes; see SPEC.md, Known reference bugs.
	for _, input := range []string{"\t\tfoo\n\tbar", "\t\tfoo\n\tbar\n", "\t\ta\nb", "\t\ta\nb\n"} {

		_, err := tree2.Parse(input, "test")

		var syntax *tree2.SyntaxError
		if !errors.As(err, &syntax) {
			t.Fatalf("%q: error is %T, want *tree2.SyntaxError", input, err)
		}
		if syntax.Reason != "Too few tabs" {
			t.Errorf("%q: reason = %q, want %q", input, syntax.Reason, "Too few tabs")
		}
		if syntax.Span.Row != 2 || syntax.Span.Col != 1 {
			t.Errorf("%q: span = %s, want row 2 col 1", input, syntax.Span)
		}
	}
}

func TestParseNeverPanics(t *testing.T) {
	inputs := []string{
		"",
		"\n",
		"\t",
		"\\",
		" ",
		"  \n",
		"\t\t\t\n",
		"a\n\t\t\tb\n",
		"\t\tfoo\n\tbar",
		"\t\tfoo\n\tbar\n",
		"a b\tc\n",
		"\xff\xfe\n",
		"a\x00b\n",
		"a \n",
		"\\\\\\\n",
		strings.Repeat("\t", 1000) + "x\n",
		strings.Repeat("a ", 1000) + "\n",
		"😀\n\t😀\n",
	}

	for _, input := range inputs {
		tree, err := tree2.Parse(input, "test")
		if err != nil {
			continue
		}

		// whatever parsed must serialize and re-parse identically
		out := tree.String()
		again, err := tree2.Parse(out, "test")
		if err != nil {
			t.Fatalf("re-parsing %q failed: %v", out, err)
		}
		if again.String() != out {
			t.Fatalf("%q is not a fixed point", out)
		}
	}
}

func TestToJSONErrors(t *testing.T) {
	for _, item := range []struct {
		name, input, want string
	}{
		{"multiple roots", "a\nb\n", "Multiple json root"},
		{"unknown type", "zz\n", "Unknown json type"},
		{"key without value", "*\n\ta\n", "Missing json value"},
	} {
		t.Run(item.name, func(t *testing.T) {

			_, err := mustParse(t, item.input, "test").ToJSON()
			if err == nil {
				t.Fatalf("converted %q, want an error", item.input)
			}
			if !strings.Contains(err.Error(), item.want) {
				t.Fatalf("error = %q, want it to contain %q", err, item.want)
			}
		})
	}
}

func TestToJSONNumbers(t *testing.T) {
	for _, item := range []struct {
		input string
		want  float64
	}{
		{"1\n", 1},
		{"-1.5\n", -1.5},
		{"1e3\n", 1000},
		{"0x10\n", 16},
	} {
		got, err := mustParse(t, item.input, "test").ToJSON()
		if err != nil {
			t.Fatalf("to json %q: %v", item.input, err)
		}
		if got != item.want {
			t.Errorf("json of %q = %v, want %v", item.input, got, item.want)
		}
	}

	got, err := mustParse(t, "NaN\n", "test").ToJSON()
	if err != nil {
		t.Fatalf("to json: %v", err)
	}
	if num, ok := got.(float64); !ok || !math.IsNaN(num) {
		t.Fatalf("json = %v, want NaN", got)
	}
}

func TestFromJSONNumbers(t *testing.T) {
	for _, item := range []struct {
		value any
		want  string
	}{
		{1.0, "1\n"},
		{1.5, "1.5\n"},
		{1000000.0, "1000000\n"},
		{1e21, "1e+21\n"},
		{1e-7, "1e-7\n"},
		{0.000001, "0.000001\n"},
		{math.Copysign(0, -1), "0\n"},
		{int64(9007199254740993), "9007199254740993\n"},
		{uint8(7), "7\n"},
		{math.NaN(), "NaN\n"},
		{math.Inf(-1), "-Infinity\n"},
	} {
		tree, err := tree2.FromJSON(item.value, tree2.SpanUnknown)
		if err != nil {
			t.Fatalf("from json %v: %v", item.value, err)
		}
		if got := tree.String(); got != item.want {
			t.Errorf("json %v built %q, want %q", item.value, got, item.want)
		}
	}
}

func TestFromJSONGoValues(t *testing.T) {
	type point struct {
		X int    `json:"x"`
		Y string `json:"y"`
	}

	tree, err := tree2.FromJSON(point{X: 1, Y: "two"}, tree2.SpanUnknown)
	if err != nil {
		t.Fatalf("from json: %v", err)
	}
	if got, want := tree.String(), "*\n\tx 1\n\ty \\two\n"; got != want {
		t.Fatalf("built %q, want %q", got, want)
	}

	bytes, err := tree2.FromJSON([]byte{0, 1, 255, 16, 32, 48, 64, 80, 96}, tree2.SpanUnknown)
	if err != nil {
		t.Fatalf("from json: %v", err)
	}
	if got, want := bytes.String(), "\\00 01 FF 10 20 30 40 50\n\\60\n"; got != want {
		t.Fatalf("built %q, want %q", got, want)
	}

	moment, err := tree2.FromJSON(time.Date(2020, 1, 2, 3, 4, 5, 0, time.UTC), tree2.SpanUnknown)
	if err != nil {
		t.Fatalf("from json: %v", err)
	}
	if got, want := moment.String(), "\\2020-01-02T03:04:05.000Z\n"; got != want {
		t.Fatalf("built %q, want %q", got, want)
	}
}

func FuzzParse(f *testing.F) {
	for _, seed := range []string{
		"", "a\n", "a b c\n", "\\x\n", "a\n\tb\n", "\t\tfoo\n\tbar", "a  b\n", "😀 \\😀\n",
	} {
		f.Add(seed)
	}

	f.Fuzz(func(t *testing.T, src string) {
		tree, err := tree2.Parse(src, "fuzz")
		if err != nil {
			var syntax *tree2.SyntaxError
			if !errors.As(err, &syntax) {
				t.Fatalf("error is %T, want *tree2.SyntaxError", err)
			}
			_ = syntax.Error()
			return
		}

		out := tree.String()
		again, err := tree2.Parse(out, "fuzz")
		if err != nil {
			t.Fatalf("re-parsing %q failed: %v", out, err)
		}
		if again.String() != out {
			t.Fatalf("%q is not a fixed point", out)
		}
	})
}
