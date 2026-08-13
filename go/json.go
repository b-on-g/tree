package tree2

import (
	"encoding/json"
	"fmt"
	"math"
	"slices"
	"strconv"
	"strings"
	"time"
)

// ToJSON converts a json.tree to plain Go values: nil, bool, float64, string,
// []any and map[string]any — exactly what encoding/json produces when decoding
// into an any.
//
// A node typed `-` is a comment: its container drops it, and at the root it
// converts to nil.
func (t *Tree) ToJSON() (any, error) {
	value, _, err := t.toJSON()
	return value, err
}

// toJSON reports through defined whether the node converts to anything at all,
// which is how `-` comments disappear from their container.
func (t *Tree) toJSON() (value any, defined bool, err error) {

	if t.typ == "" {

		data := true
		for _, kid := range t.kids {
			if kid.typ != "" {
				data = false
				break
			}
		}
		if data {
			return t.Text(), true, nil
		}

		if len(t.kids) != 1 {
			return nil, false, fmt.Errorf("Multiple json root at %s", t.span)
		}

		return t.kids[0].toJSON()
	}

	switch t.typ {

	case "-":
		return nil, false, nil

	case "true":
		return true, true, nil

	case "false":
		return false, true, nil

	case "null":
		return nil, true, nil

	case "*":

		obj := map[string]any{}

		for _, kid := range t.kids {
			if kid.typ == "-" {
				continue
			}

			if len(kid.kids) == 0 {
				// The reference reads kids[-1] here and crashes.
				return nil, false, fmt.Errorf("Missing json value at %s", kid.span)
			}

			key := kid.typ
			if key == "" {
				key = kid.Clone(kid.kids[:len(kid.kids)-1]).Text()
			}

			val, defined, err := kid.kids[len(kid.kids)-1].toJSON()
			if err != nil {
				return nil, false, err
			}
			if defined {
				obj[key] = val
			}
		}

		return obj, true, nil

	case "/":

		arr := []any{}

		for _, kid := range t.kids {
			if kid.typ == "-" {
				continue
			}

			val, defined, err := kid.toJSON()
			if err != nil {
				return nil, false, err
			}
			if defined {
				arr = append(arr, val)
			}
		}

		return arr, true, nil
	}

	num, ok := parseNumber(t.typ)
	if !ok {
		return nil, false, fmt.Errorf("Unknown json type (%s) at %s", t.typ, t.span)
	}

	return num, true, nil
}

// FromJSON converts plain Go values into a json.tree.
//
// Booleans, numbers and nil become struct nodes typed with their literal
// spelling, strings become data nodes, slices become `/` nodes and maps become
// `*` nodes. A []byte becomes uppercase hex, eight bytes to a line, and a
// time.Time an ISO-8601 data node. Anything else is routed through
// encoding/json first, so structs and named types convert the way they
// marshal.
//
// Map keys are visited in sorted order, since a Go map has no order of its
// own. Trees built from single-key or already-alphabetical objects therefore
// round-trip unchanged; others come back sorted.
func FromJSON(value any, span Span) (*Tree, error) {

	switch value := value.(type) {

	case nil:
		return newTree("null", "", nil, span), nil

	case bool:
		return newTree(strconv.FormatBool(value), "", nil, span), nil

	case string:
		return Data(value, nil, span), nil

	case json.Number:
		return newTree(value.String(), "", nil, span), nil

	case float64:
		return newTree(formatNumber(value), "", nil, span), nil

	case float32:
		return newTree(formatNumber(float64(value)), "", nil, span), nil

	case int, int8, int16, int32, int64, uint, uint8, uint16, uint32, uint64:
		// Spelled out rather than routed through float64, which cannot hold
		// every 64 bit integer exactly.
		return newTree(fmt.Sprintf("%d", value), "", nil, span), nil

	case []byte:
		return Data(hexDump(value), nil, span), nil

	case time.Time:
		return newTree("", value.UTC().Format("2006-01-02T15:04:05.000Z"), nil, span), nil

	case []any:

		sub := make([]*Tree, 0, len(value))
		for _, item := range value {
			kid, err := FromJSON(item, span)
			if err != nil {
				return nil, err
			}
			sub = append(sub, kid)
		}

		return newTree("/", "", sub, span), nil

	case map[string]any:

		keys := make([]string, 0, len(value))
		for key := range value {
			keys = append(keys, key)
		}
		slices.Sort(keys)

		sub := make([]*Tree, 0, len(keys))
		for _, key := range keys {

			kid, err := FromJSON(value[key], span)
			if err != nil {
				return nil, err
			}

			if key != "" && !strings.ContainsAny(key, " \n\t\\") {
				sub = append(sub, newTree(key, "", []*Tree{kid}, span))
			} else {
				sub = append(sub, Data(key, []*Tree{kid}, span))
			}
		}

		return newTree("*", "", sub, span), nil
	}

	// Everything else goes through encoding/json, which reduces it to the
	// cases above. The result cannot re-enter this branch.
	raw, err := json.Marshal(value)
	if err != nil {
		return nil, fmt.Errorf("cannot convert %T to tree: %w", value, err)
	}

	var generic any
	if err := json.Unmarshal(raw, &generic); err != nil {
		return nil, fmt.Errorf("cannot convert %T to tree: %w", value, err)
	}

	return FromJSON(generic, span)
}

// hexDump renders bytes as uppercase hex, space separated, eight per line.
func hexDump(data []byte) string {

	var out strings.Builder

	for i, b := range data {
		switch {
		case i == 0:
		case i%8 == 0:
			out.WriteByte('\n')
		default:
			out.WriteByte(' ')
		}
		fmt.Fprintf(&out, "%02X", b)
	}

	return out.String()
}

// parseNumber reads a node type as a number the way JavaScript's Number() does
// for the spellings a tree can hold.
func parseNumber(str string) (float64, bool) {

	if str == "NaN" {
		return math.NaN(), true
	}

	if strings.ContainsRune(str, '_') {
		return 0, false
	}

	if len(str) > 2 && str[0] == '0' {
		switch str[1] {
		case 'x', 'X', 'b', 'B', 'o', 'O':
			num, err := strconv.ParseInt(str, 0, 64)
			if err != nil {
				return 0, false
			}
			return float64(num), true
		}
	}

	num, err := strconv.ParseFloat(str, 64)
	if err != nil {
		return 0, false
	}

	return num, true
}

// formatNumber renders a float the way JavaScript's String() does: the
// shortest decimal that reads back exactly, written plainly while the decimal
// point stays within the range JavaScript keeps it in, and in exponential
// notation outside of it.
func formatNumber(num float64) string {

	switch {
	case math.IsNaN(num):
		return "NaN"
	case math.IsInf(num, 1):
		return "Infinity"
	case math.IsInf(num, -1):
		return "-Infinity"
	case num == 0:
		return "0" // also covers negative zero, which JavaScript prints as 0
	}

	sign := ""
	if num < 0 {
		sign = "-"
		num = -num
	}

	// Shortest round-tripping form, e.g. "1.5e+00".
	shortest := strconv.FormatFloat(num, 'e', -1, 64)
	mantissa, exponent, _ := strings.Cut(shortest, "e")
	power, err := strconv.Atoi(exponent)
	if err != nil {
		return sign + shortest
	}

	digits := strings.Replace(mantissa, ".", "", 1)

	// point is where the decimal point sits among the digits.
	point := power + 1
	count := len(digits)

	switch {

	case count <= point && point <= 21:
		return sign + digits + strings.Repeat("0", point-count)

	case 0 < point && point <= 21:
		return sign + digits[:point] + "." + digits[point:]

	case -6 < point && point <= 0:
		return sign + "0." + strings.Repeat("0", -point) + digits

	case count == 1:
		return sign + digits + "e" + exponentSign(point-1)

	default:
		return sign + digits[:1] + "." + digits[1:] + "e" + exponentSign(point-1)
	}
}

func exponentSign(power int) string {
	if power >= 0 {
		return "+" + strconv.Itoa(power)
	}
	return strconv.Itoa(power)
}
