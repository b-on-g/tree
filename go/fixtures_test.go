package tree2_test

import (
	"encoding/json"
	"errors"
	"os"
	"path/filepath"
	"reflect"
	"testing"

	tree2 "github.com/b-on-g/tree/go"
)

// The conformance corpus lives in the repository root and is shared by every
// port, so it is read from there rather than vendored.
const corpus = "../fixtures"

func load[Case any](t *testing.T, name string) []Case {
	t.Helper()

	raw, err := os.ReadFile(filepath.Join(corpus, name+".json"))
	if err != nil {
		t.Fatalf("cannot read corpus: %v", err)
	}

	var cases []Case
	if err := json.Unmarshal(raw, &cases); err != nil {
		t.Fatalf("cannot decode %s.json: %v", name, err)
	}
	if len(cases) == 0 {
		t.Fatalf("%s.json is empty", name)
	}

	return cases
}

type spanFixture struct {
	URI      string `json:"uri"`
	Row      int    `json:"row"`
	Col      int    `json:"col"`
	Length   int    `json:"length"`
	ColCP    int    `json:"col_cp"`
	LengthCP int    `json:"length_cp"`
}

type nodeFixture struct {
	Type  string        `json:"type"`
	Value string        `json:"value"`
	Span  *spanFixture  `json:"span"`
	Kids  []nodeFixture `json:"kids"`
}

type treeFixture struct {
	Name  string      `json:"name"`
	URI   string      `json:"uri"`
	Input string      `json:"input"`
	Tree  nodeFixture `json:"tree"`
}

// steps turns a corpus path into path steps: a string selects by type, a
// number by index and a null matches every child.
func steps(t *testing.T, path []any) []tree2.Step {
	t.Helper()

	out := make([]tree2.Step, 0, len(path))
	for _, item := range path {
		switch step := item.(type) {
		case nil:
			out = append(out, tree2.Any())
		case string:
			out = append(out, tree2.ByType(step))
		case float64:
			out = append(out, tree2.ByIndex(int(step)))
		default:
			t.Fatalf("unsupported path step %T", item)
		}
	}

	return out
}

// errorSpec is the hand-written error expectation of reference_bugs.json:
// plain code point coordinates, without the UTF-16 counterpart the generated
// files carry.
type errorSpec struct {
	Reason string `json:"reason"`
	Row    int    `json:"row"`
	Col    int    `json:"col"`
	Length int    `json:"length"`
}

func checkParseFailure(t *testing.T, input string, want errorSpec) {
	t.Helper()

	tree, err := tree2.Parse(input, "test")
	if err == nil {
		t.Fatalf("parsed %q into %q, want %s", input, tree, want.Reason)
	}

	var syntax *tree2.SyntaxError
	if !errors.As(err, &syntax) {
		t.Fatalf("error is %T, want *tree2.SyntaxError", err)
	}

	if syntax.Reason != want.Reason {
		t.Errorf("reason = %q, want %q", syntax.Reason, want.Reason)
	}
	if syntax.Span.Row != want.Row || syntax.Span.Col != want.Col || syntax.Span.Length != want.Length {
		t.Errorf("span = %s, want test#%d:%d/%d", syntax.Span, want.Row, want.Col, want.Length)
	}
}

func mustParse(t *testing.T, input, uri string) *tree2.Tree {
	t.Helper()

	tree, err := tree2.Parse(input, uri)
	if err != nil {
		t.Fatalf("parse %q: %v", input, err)
	}

	return tree
}

func mustStruct(t *testing.T, typ string) *tree2.Tree {
	t.Helper()

	node, err := tree2.Struct(typ, nil, tree2.SpanUnknown)
	if err != nil {
		t.Fatalf("struct %q: %v", typ, err)
	}

	return node
}

// buildShape rebuilds a recorded node through the factories.
func buildShape(t *testing.T, want nodeFixture) *tree2.Tree {
	t.Helper()

	kids := make([]*tree2.Tree, 0, len(want.Kids))
	for _, kid := range want.Kids {
		kids = append(kids, buildShape(t, kid))
	}

	switch {
	case want.Type != "":
		node, err := tree2.Struct(want.Type, kids, tree2.SpanUnknown)
		if err != nil {
			t.Fatalf("struct %q: %v", want.Type, err)
		}
		return node
	case want.Value != "":
		return tree2.Data(want.Value, kids, tree2.SpanUnknown)
	default:
		return tree2.List(kids, tree2.SpanUnknown)
	}
}

func checkNode(t *testing.T, got *tree2.Tree, want nodeFixture, path string) {
	t.Helper()

	if got == nil {
		t.Fatalf("%s: missing node", path)
	}
	if got.Type() != want.Type {
		t.Errorf("%s: type = %q, want %q", path, got.Type(), want.Type)
	}
	if got.Value() != want.Value {
		t.Errorf("%s: value = %q, want %q", path, got.Value(), want.Value)
	}

	if want.Span != nil {
		span := got.Span()
		if span.URI != want.Span.URI {
			t.Errorf("%s: span uri = %q, want %q", path, span.URI, want.Span.URI)
		}
		if span.Row != want.Span.Row {
			t.Errorf("%s: span row = %d, want %d", path, span.Row, want.Span.Row)
		}
		if span.Col != want.Span.ColCP {
			t.Errorf("%s: span col = %d, want %d", path, span.Col, want.Span.ColCP)
		}
		if span.Length != want.Span.LengthCP {
			t.Errorf("%s: span length = %d, want %d", path, span.Length, want.Span.LengthCP)
		}
	}

	if got.KidCount() != len(want.Kids) {
		t.Fatalf("%s: %d kids, want %d", path, got.KidCount(), len(want.Kids))
	}
	for i, kid := range want.Kids {
		checkNode(t, got.Kid(i), kid, path+"/"+kid.Type+kid.Value)
	}
}

func TestParse(t *testing.T) {
	for _, item := range load[treeFixture](t, "parse") {
		t.Run(item.Name, func(t *testing.T) {
			checkNode(t, mustParse(t, item.Input, item.URI), item.Tree, "")
		})
	}
}

func TestSpans(t *testing.T) {
	for _, item := range load[treeFixture](t, "spans") {
		t.Run(item.Name, func(t *testing.T) {
			checkNode(t, mustParse(t, item.Input, item.URI), item.Tree, "")
		})
	}
}

func TestParseErrors(t *testing.T) {
	type errorFixture struct {
		Name    string      `json:"name"`
		URI     string      `json:"uri"`
		Input   string      `json:"input"`
		Reason  string      `json:"reason"`
		Line    string      `json:"line"`
		Span    spanFixture `json:"span"`
		Message string      `json:"message"`
	}

	for _, item := range load[errorFixture](t, "parse_errors") {
		t.Run(item.Name, func(t *testing.T) {

			tree, err := tree2.Parse(item.Input, item.URI)
			if err == nil {
				t.Fatalf("parsed %q into %q, want error", item.Input, tree)
			}

			var syntax *tree2.SyntaxError
			if !errors.As(err, &syntax) {
				t.Fatalf("error is %T, want *tree2.SyntaxError", err)
			}

			if syntax.Reason != item.Reason {
				t.Errorf("reason = %q, want %q", syntax.Reason, item.Reason)
			}
			if syntax.Line != item.Line {
				t.Errorf("line = %q, want %q", syntax.Line, item.Line)
			}
			if syntax.Span.URI != item.Span.URI {
				t.Errorf("span uri = %q, want %q", syntax.Span.URI, item.Span.URI)
			}
			if syntax.Span.Row != item.Span.Row {
				t.Errorf("span row = %d, want %d", syntax.Span.Row, item.Span.Row)
			}
			if syntax.Span.Col != item.Span.ColCP {
				t.Errorf("span col = %d, want %d", syntax.Span.Col, item.Span.ColCP)
			}
			if syntax.Span.Length != item.Span.LengthCP {
				t.Errorf("span length = %d, want %d", syntax.Span.Length, item.Span.LengthCP)
			}

			// Every error in the corpus sits on an ASCII line, so the rendered
			// message must match the reference byte for byte.
			if got := syntax.Error(); got != item.Message {
				t.Errorf("message =\n%q\nwant\n%q", got, item.Message)
			}
		})
	}
}

func TestSerialize(t *testing.T) {
	type serializeFixture struct {
		Name   string `json:"name"`
		Input  string `json:"input"`
		Output string `json:"output"`
	}

	for _, item := range load[serializeFixture](t, "serialize") {
		t.Run(item.Name, func(t *testing.T) {

			got := mustParse(t, item.Input, "test").String()
			if got != item.Output {
				t.Fatalf("serialized %q, want %q", got, item.Output)
			}

			// and serializing is a fixed point
			again := mustParse(t, got, "test").String()
			if again != item.Output {
				t.Fatalf("re-serialized %q, want %q", again, item.Output)
			}
		})
	}
}

func TestSerializeBuilt(t *testing.T) {
	type builtFixture struct {
		Name   string      `json:"name"`
		Tree   nodeFixture `json:"tree"`
		Output string      `json:"output"`
	}

	// The corpus records what the factories produced; these rebuild it.
	builders := map[string]func(t *testing.T) *tree2.Tree{

		"multiline data splits into kids": func(t *testing.T) *tree2.Tree {
			return tree2.Data("a\nb\nc", nil, tree2.SpanUnknown)
		},
		"multiline data with extra kids": func(t *testing.T) *tree2.Tree {
			return tree2.Data("a\nb", []*tree2.Tree{mustStruct(t, "x")}, tree2.SpanUnknown)
		},
		"struct with no kids": func(t *testing.T) *tree2.Tree {
			return mustStruct(t, "foo")
		},
		"struct with one kid collapses inline": func(t *testing.T) *tree2.Tree {
			node, err := tree2.Struct("a", []*tree2.Tree{mustStruct(t, "b")}, tree2.SpanUnknown)
			if err != nil {
				t.Fatal(err)
			}
			return node
		},
		"struct with two kids goes multiline": func(t *testing.T) *tree2.Tree {
			node, err := tree2.Struct("a", []*tree2.Tree{mustStruct(t, "b"), mustStruct(t, "c")}, tree2.SpanUnknown)
			if err != nil {
				t.Fatal(err)
			}
			return node
		},
		"list of structs": func(t *testing.T) *tree2.Tree {
			return tree2.List([]*tree2.Tree{mustStruct(t, "a"), mustStruct(t, "b")}, tree2.SpanUnknown)
		},
		"empty list": func(t *testing.T) *tree2.Tree {
			return tree2.List(nil, tree2.SpanUnknown)
		},
		"data at root": func(t *testing.T) *tree2.Tree {
			return tree2.List([]*tree2.Tree{tree2.Data("x", nil, tree2.SpanUnknown)}, tree2.SpanUnknown)
		},
		"empty data at root": func(t *testing.T) *tree2.Tree {
			return tree2.List([]*tree2.Tree{tree2.Data("", nil, tree2.SpanUnknown)}, tree2.SpanUnknown)
		},
		"nested data under struct": func(t *testing.T) *tree2.Tree {
			node, err := tree2.Struct("a", []*tree2.Tree{
				tree2.Data("x", nil, tree2.SpanUnknown),
				tree2.Data("y", nil, tree2.SpanUnknown),
			}, tree2.SpanUnknown)
			if err != nil {
				t.Fatal(err)
			}
			return node
		},
	}

	for _, item := range load[builtFixture](t, "serialize_built") {
		t.Run(item.Name, func(t *testing.T) {

			// A case added to the corpus after this test was written still
			// gets rebuilt through the factories, just from its recorded
			// shape instead of a hand-written recipe.
			build, known := builders[item.Name]
			if !known {
				build = func(t *testing.T) *tree2.Tree { return buildShape(t, item.Tree) }
			}

			tree := build(t)

			// The corpus dumps the shape the factories produced, without spans.
			checkNode(t, tree, item.Tree, "")

			if got := tree.String(); got != item.Output {
				t.Fatalf("serialized %q, want %q", got, item.Output)
			}
		})
	}
}

func TestText(t *testing.T) {
	type textFixture struct {
		Name     string `json:"name"`
		Input    string `json:"input"`
		Text     string `json:"text"`
		RootText string `json:"root_text"`
	}

	for _, item := range load[textFixture](t, "text") {
		t.Run(item.Name, func(t *testing.T) {

			root := mustParse(t, item.Input, "test")

			if got := root.Text(); got != item.RootText {
				t.Errorf("root text = %q, want %q", got, item.RootText)
			}
			if got := root.Kid(0).Text(); got != item.Text {
				t.Errorf("text = %q, want %q", got, item.Text)
			}
		})
	}
}

func TestSelect(t *testing.T) {
	type selectFixture struct {
		Name   string `json:"name"`
		Input  string `json:"input"`
		Path   []any  `json:"path"`
		Output string `json:"output"`
	}

	for _, item := range load[selectFixture](t, "select") {
		t.Run(item.Name, func(t *testing.T) {

			got := mustParse(t, item.Input, "test").Select(steps(t, item.Path)...).String()
			if got != item.Output {
				t.Fatalf("selected %q, want %q", got, item.Output)
			}
		})
	}
}

func TestFilter(t *testing.T) {
	type filterFixture struct {
		Name     string `json:"name"`
		Input    string `json:"input"`
		Path     []any  `json:"path"`
		Value    string `json:"value"`
		HasValue bool   `json:"has_value"`
		Output   string `json:"output"`
	}

	for _, item := range load[filterFixture](t, "filter") {
		t.Run(item.Name, func(t *testing.T) {

			// The corpus filters the first kid of the parsed root.
			node := mustParse(t, item.Input, "test").Kid(0)
			path := steps(t, item.Path)

			var got string
			if item.HasValue {
				got = node.FilterValue(item.Value, path...).String()
			} else {
				got = node.Filter(path...).String()
			}

			if got != item.Output {
				t.Fatalf("filtered %q, want %q", got, item.Output)
			}
		})
	}
}

func TestInsert(t *testing.T) {
	type insertFixture struct {
		Name   string  `json:"name"`
		Input  string  `json:"input"`
		Insert *string `json:"insert"`
		Path   []any   `json:"path"`
		Output string  `json:"output"`
	}

	for _, item := range load[insertFixture](t, "insert") {
		t.Run(item.Name, func(t *testing.T) {

			var value *tree2.Tree
			if item.Insert != nil {
				value = mustStruct(t, *item.Insert)
			}

			got, err := mustParse(t, item.Input, "test").Insert(value, steps(t, item.Path)...)
			if err != nil {
				t.Fatalf("insert: %v", err)
			}

			if got.String() != item.Output {
				t.Fatalf("inserted %q, want %q", got, item.Output)
			}
		})
	}
}

func TestUpdate(t *testing.T) {
	type updateFixture struct {
		Name   string   `json:"name"`
		Input  string   `json:"input"`
		Update []string `json:"update"`
		Path   []any    `json:"path"`
		Output string   `json:"output"`
	}

	for _, item := range load[updateFixture](t, "update") {
		t.Run(item.Name, func(t *testing.T) {

			values := make([]*tree2.Tree, 0, len(item.Update))
			for _, typ := range item.Update {
				values = append(values, mustStruct(t, typ))
			}

			got, err := mustParse(t, item.Input, "test").Update(values, steps(t, item.Path)...)
			if err != nil {
				t.Fatalf("update: %v", err)
			}
			if len(got) == 0 {
				t.Fatalf("update returned nothing, want %q", item.Output)
			}

			if got[0].String() != item.Output {
				t.Fatalf("updated %q, want %q", got[0], item.Output)
			}
		})
	}
}

func TestToJSON(t *testing.T) {
	type toJSONFixture struct {
		Name  string `json:"name"`
		Input string `json:"input"`
		JSON  any    `json:"json"`
	}

	for _, item := range load[toJSONFixture](t, "to_json") {
		t.Run(item.Name, func(t *testing.T) {

			got, err := mustParse(t, item.Input, "test").ToJSON()
			if err != nil {
				t.Fatalf("to json: %v", err)
			}

			if !reflect.DeepEqual(got, item.JSON) {
				t.Fatalf("json = %#v, want %#v", got, item.JSON)
			}
		})
	}
}

func TestFromJSON(t *testing.T) {
	type fromJSONFixture struct {
		Name   string `json:"name"`
		JSON   any    `json:"json"`
		Output string `json:"output"`
	}

	for _, item := range load[fromJSONFixture](t, "from_json") {
		t.Run(item.Name, func(t *testing.T) {

			tree, err := tree2.FromJSON(item.JSON, tree2.SpanUnknown)
			if err != nil {
				t.Fatalf("from json: %v", err)
			}

			if got := tree.String(); got != item.Output {
				t.Fatalf("built %q, want %q", got, item.Output)
			}

			// and it reads back as the same json
			back, err := mustParse(t, tree.String(), "test").ToJSON()
			if err != nil {
				t.Fatalf("to json: %v", err)
			}
			if !reflect.DeepEqual(back, item.JSON) {
				t.Fatalf("round trip gave %#v, want %#v", back, item.JSON)
			}
		})
	}
}

func TestReferenceBugs(t *testing.T) {
	type bugFixture struct {
		Name   string     `json:"name"`
		Op     string     `json:"op"`
		Input  string     `json:"input"`
		Path   []any      `json:"path"`
		Update []string   `json:"update"`
		Insert *string    `json:"insert"`
		Output string     `json:"output"`
		Error  *errorSpec `json:"error"`
	}

	for _, item := range load[bugFixture](t, "reference_bugs") {
		t.Run(item.Name, func(t *testing.T) {

			// A parse case fails by design, so it cannot go through mustParse.
			if item.Op == "parse" {
				checkParseFailure(t, item.Input, *item.Error)
				return
			}

			root := mustParse(t, item.Input, "test")
			path := steps(t, item.Path)

			var got string

			switch item.Op {

			case "select":
				got = root.Select(path...).String()

			case "update":
				values := make([]*tree2.Tree, 0, len(item.Update))
				for _, typ := range item.Update {
					values = append(values, mustStruct(t, typ))
				}

				next, err := root.Update(values, path...)
				if err != nil {
					t.Fatalf("update: %v", err)
				}
				if len(next) == 0 {
					t.Fatalf("update returned nothing, want %q", item.Output)
				}
				got = next[0].String()

			case "insert":
				var value *tree2.Tree
				if item.Insert != nil {
					value = mustStruct(t, *item.Insert)
				}

				next, err := root.Insert(value, path...)
				if err != nil {
					t.Fatalf("insert: %v", err)
				}
				got = next.String()

			default:
				t.Fatalf("unsupported op %q", item.Op)
			}

			if got != item.Output {
				t.Fatalf("got %q, want %q", got, item.Output)
			}
		})
	}
}
