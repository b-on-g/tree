// Generates the language-neutral conformance corpus in ../fixtures
// straight from the reference TypeScript implementation.
//
//   node tools/build-ref.js && node tools/gen-fixtures.js
//
// Never hand-edit fixtures/*.json — add a case here and regenerate.

const fs = require( 'fs' )
const path = require( 'path' )

require( path.join( __dirname, 'ref.js' ) )

const out_dir = path.join( __dirname, '..', 'fixtures' )
fs.mkdirSync( out_dir, { recursive: true } )

const { $mol_tree2 } = $$

const parse = ( str, uri = 'test' )=> $$.$mol_tree2_from_string( str, uri )

const cp_len = str => [ ...str ].length

/**
 * The reference measures columns in UTF-16 code units, because that is what a
 * JS string index is. Ports measure them in Unicode scalar values instead —
 * see SPEC.md "Columns". Both are emitted so a port can pick its side and the
 * two can be diffed.
 */
function span_json( span, source ) {
	// absolute UTF-16 offset of the span inside the source
	let line_start = 0
	for( let r = 1; r < span.row; ++r ) {
		const next = source.indexOf( '\n', line_start )
		if( next < 0 ) break
		line_start = next + 1
	}
	const offset = line_start + span.col - 1

	// A span may reach past the end of the source — the one an "Unexpected EOF"
	// error carries marks a character that is not there. Measuring only the
	// slice would report 0 for it, so the part beyond the end is counted at its
	// nominal width, where a code unit and a code point are the same thing.
	const region = source.slice( offset, offset + span.length )
	const beyond = span.length - region.length

	return {
		uri: span.uri,
		row: span.row,
		col: span.col,
		length: span.length,
		col_cp: 1 + cp_len( source.slice( line_start, offset ) ),
		length_cp: cp_len( region ) + beyond,
	}
}

/** Full node dump including spans. */
function dump( node, source ) {
	return {
		type: node.type,
		value: node.value,
		span: span_json( node.span, source ),
		kids: node.kids.map( kid => dump( kid, source ) ),
	}
}

/** Node dump without spans — for cases where only shape matters. */
function shape( node ) {
	return {
		type: node.type,
		value: node.value,
		kids: node.kids.map( shape ),
	}
}

function write( name, cases ) {
	const file = path.join( out_dir, name + '.json' )
	fs.writeFileSync( file, JSON.stringify( cases, null, '\t' ) + '\n' )
	console.log( `${ name }.json — ${ cases.length } cases` )
}

// ---------------------------------------------------------------- parse

const parse_inputs = [
	[ 'empty', '' ],
	[ 'single struct', 'foo\n' ],
	[ 'two structs', 'foo\nbar\n' ],
	[ 'blank lines are skipped', 'foo\n\n\n' ],
	[ 'only blank lines', '\n\n\n' ],
	[ 'nested by tab', 'foo\n\tbar\n' ],
	[ 'deep nesting', 'a\n\tb\n\t\tc\n\t\t\td\n' ],
	[ 'siblings under parent', 'a\n\tb\n\tc\n' ],
	[ 'dedent back', 'a\n\tb\n\t\tc\nd\n' ],
	[ 'inline chain', 'foo bar baz\n' ],
	[ 'inline chain then data', 'foo bar \\pol\n' ],
	[ 'data node', '\\hello\n' ],
	[ 'empty data node', '\\\n' ],
	[ 'struct then data sibling', '=foo\n\\bar\n' ],
	[ 'multiline data under struct', 'foo bar\n\t\\pol\n\t\\men\n' ],
	[ 'data keeps leading spaces', '\\  spaced\n' ],
	[ 'data keeps trailing spaces', '\\spaced  \n' ],
	[ 'data keeps tabs inside', '\\a\tb\n' ],
	[ 'data keeps backslashes inside', '\\a\\b\\\n' ],
	[ 'type with punctuation', 'a.b-c/d:e\n' ],
	[ 'type with unicode', 'привет мир\n' ],
	[ 'data with unicode', '\\привет мир\n' ],
	[ 'data with astral plane', '\\a😀b\n' ],
	[ 'type with astral plane', 'a😀b\n' ],
	[ 'base indent from first content line', '\n\t\tfoo\n\t\t\tbar\n' ],
	[ 'base indent with deeper first line', '\n\t\t\ta\n\t\t\t\tb\n' ],
	[ 'kids after inline chain', 'a b\n\t\\x\n\t\\y\n' ],
	[ 'struct with both inline and nested kids', 'a b\n\tc\n' ],
	[ 'json object markers', '* a \\1\n' ],
	[ 'json array markers', '/ \\a\n' ],
	[ 'many blank lines between nodes', 'a\n\n\nb\n' ],
	[ 'trailing blank line', 'a\nb\n\n' ],
]

write( 'parse', parse_inputs.map( ( [ name, input ] )=> ( {
	name,
	uri: 'test',
	input,
	tree: dump( parse( input ), input ),
} ) ) )

// ------------------------------------------------------------ parse errors

const error_inputs = [
	[ 'too many tabs', '\n\t\t\t\tfoo\n\t\t\t\t\t\tbar\n' ],
	[ 'too few tabs', '\n\t\t\t\t\tfoo\n\t\t\t\tbar\n' ],
	[ 'wrong nodes separator at line start', 'foo\n \tbar\n' ],
	[ 'wrong nodes separator in the middle', 'foo  bar\n' ],
	[ 'wrong nodes separator triple space', 'foo   bar\n' ],
	[ 'wrong nodes separator tab inline', 'foo\tbar\n' ],
	[ 'unexpected eof without lf', '\tfoo' ],
	[ 'unexpected eof after data', '\\foo' ],
	[ 'unexpected eof bare type', 'foo' ],
]

write( 'parse_errors', error_inputs.map( ( [ name, input ] )=> {
	let caught = null
	try {
		parse( input )
	} catch( error ) {
		caught = error
	}
	if( !caught ) throw new Error( `Case "${ name }" did not fail` )
	return {
		name,
		uri: 'test',
		input,
		reason: caught.reason,
		line: caught.line,
		span: span_json( caught.span, input ),
		message: caught.message,
	}
} ) )

// ------------------------------------------------------------- serialize

// Round-trip: every parseable input must serialize back to a stable form,
// and re-parsing that form must be a fixed point.
write( 'serialize', parse_inputs.map( ( [ name, input ] )=> {
	const once = String( parse( input ) )
	const twice = String( parse( once ) )
	if( once !== twice ) throw new Error( `Case "${ name }" is not a fixed point` )
	return { name, input, output: once }
} ) )

// Serialization of trees built through the factory API, where the string
// form cannot be produced by the parser at all.
const built = [
	[ 'multiline data splits into kids', ()=> $mol_tree2.data( 'a\nb\nc' ) ],
	[ 'multiline data with extra kids', ()=> $mol_tree2.data( 'a\nb', [ $mol_tree2.struct( 'x' ) ] ) ],
	[ 'struct with no kids', ()=> $mol_tree2.struct( 'foo' ) ],
	[ 'struct with one kid collapses inline', ()=> $mol_tree2.struct( 'a', [ $mol_tree2.struct( 'b' ) ] ) ],
	[ 'struct with two kids goes multiline', ()=> $mol_tree2.struct( 'a', [ $mol_tree2.struct( 'b' ), $mol_tree2.struct( 'c' ) ] ) ],
	[ 'list of structs', ()=> $mol_tree2.list( [ $mol_tree2.struct( 'a' ), $mol_tree2.struct( 'b' ) ] ) ],
	[ 'empty list', ()=> $mol_tree2.list( [] ) ],
	[ 'data at root', ()=> $mol_tree2.list( [ $mol_tree2.data( 'x' ) ] ) ],
	[ 'empty data at root', ()=> $mol_tree2.list( [ $mol_tree2.data( '' ) ] ) ],
	[ 'nested data under struct', ()=> $mol_tree2.struct( 'a', [ $mol_tree2.data( 'x' ), $mol_tree2.data( 'y' ) ] ) ],
]

write( 'serialize_built', built.map( ( [ name, make ] )=> ( {
	name,
	tree: shape( make() ),
	output: String( make() ),
} ) ) )

// ------------------------------------------------------------------ text

const text_cases = [
	[ 'plain value', '\\hello\n' ],
	[ 'value joined with data kids', 'a b\n\t\\x\n\t\\y\n' ],
	[ 'struct kids are ignored', 'a\n\t\\x\n\tb\n\t\\y\n' ],
	[ 'no kids', 'a\n' ],
	[ 'empty data kids', 'a\n\t\\\n\t\\\n' ],
]

write( 'text', text_cases.map( ( [ name, input ] )=> ( {
	name,
	input,
	// text() of the first kid of the parsed root
	text: parse( input ).kids[ 0 ].text(),
	root_text: parse( input ).text(),
} ) ) )

// ---------------------------------------------------------------- select

const select_cases = [
	[ 'by type', 'a b c d\n', [ 'a' ] ],
	[ 'by type chain', 'a b c d\n', [ 'a', 'b' ] ],
	[ 'missing type', 'a b c d\n', [ 'z' ] ],
	[ 'by index', 'a\n\tx\n\ty\n\tz\n', [ 'a', 1 ] ],
	[ 'index out of range', 'a\n\tx\n', [ 'a', 5 ] ],
	[ 'null takes all kids', 'a\n\tx\n\ty\n', [ 'a', null ] ],
	[ 'null at root', 'a\nb\n', [ null ] ],
	[ 'deep null', 'a b c d\n', [ 'a', 'b', 'c', null ] ],
	[ 'empty path returns self as list', 'a b\n', [] ],
	[ 'multiple same-type siblings', 'a\n\tb x\n\tb y\n', [ 'a', 'b' ] ],
	[ 'mixed path', 'a\n\tb\n\t\tc\n\t\td\n', [ 'a', 'b', 1 ] ],
]

write( 'select', select_cases.map( ( [ name, input, path ] )=> ( {
	name,
	input,
	path,
	output: String( parse( input ).select( ...path ) ),
} ) ) )

// ---------------------------------------------------------------- filter

const filter_cases = [
	[ 'kids having path', 'r\n\ta\n\t\tx\n\tb\n\t\ty\n', [ 'x' ], undefined ],
	[ 'kids having path with value', 'r\n\ta x \\1\n\tb x \\2\n', [ 'x', null ], '1' ],
	[ 'no match', 'r\n\ta\n\tb\n', [ 'z' ], undefined ],
	[ 'value mismatch', 'r\n\ta x \\1\n', [ 'x', null ], '9' ],
]

write( 'filter', filter_cases.map( ( [ name, input, path, value ] )=> ( {
	name,
	input,
	path,
	value: value === undefined ? null : value,
	has_value: value !== undefined,
	output: String( parse( input ).kids[ 0 ].filter( path, value ) ),
} ) ) )

// ------------------------------------------------------------ insert/update

const insert_cases = [
	[ 'replace by type path', 'a b c d\n', 'x', [ 'a', 'b', 'c' ] ],
	[ 'create missing type path', 'a b\n', 'x', [ 'a', 'b', 'c', 'd' ] ],
	[ 'replace by index path', 'a b c d\n', 'x', [ 0, 0, 0 ] ],
	[ 'extend by index path', 'a b\n', 'x', [ 0, 0, 0, 0 ] ],
	[ 'replace by null path', 'a b c d\n', 'x', [ null, null, null ] ],
	[ 'extend by null path', 'a b\n', 'x', [ null, null, null, null ] ],
	[ 'delete by type path', 'a b c d\n', null, [ 'a', 'b', 'c' ] ],
	[ 'delete by index path', 'a b c d\n', null, [ 0, 0, 0 ] ],
	[ 'empty path replaces root', 'a b\n', 'x', [] ],
]

write( 'insert', insert_cases.map( ( [ name, input, type, path ] )=> ( {
	name,
	input,
	// struct node to insert, by type; null means delete
	insert: type,
	path,
	output: String( parse( input ).insert( type === null ? null : $mol_tree2.struct( type ), ...path ) ),
} ) ) )

const update_cases = [
	[ 'update to empty deletes', 'a b c d\n', [], [ 'a', 'b', 'c' ] ],
	[ 'update root', 'a b c d\n', [ 'x' ], [] ],
	[ 'update to two nodes', 'a b c d\n', [ 'x', 'y' ], [ 'a', 'b', 'c' ] ],
	[ 'update missing path creates', 'a b\n', [ 'x' ], [ 'a', 'b', 'c' ] ],
]

write( 'update', update_cases.map( ( [ name, input, types, path ] )=> ( {
	name,
	input,
	update: types,
	path,
	output: String( parse( input ).update( types.map( t => $mol_tree2.struct( t ) ), ...path )[ 0 ] ),
} ) ) )

// ----------------------------------------------------------------- json

const to_json_cases = [
	[ 'string', '\\hello\n' ],
	[ 'multiline string', '\\a\n\\b\n' ],
	[ 'number', '1\n' ],
	[ 'negative number', '-1.5\n' ],
	[ 'true', 'true\n' ],
	[ 'false', 'false\n' ],
	[ 'null', 'null\n' ],
	[ 'object', '* a \\1\n' ],
	[ 'object with two keys', '* \n\ta \\1\n\tb \\2\n' ],
	[ 'nested object', '* a * b \\1\n' ],
	[ 'array', '/ \n\t\\a\n\t\\b\n' ],
	[ 'array of numbers', '/ \n\t1\n\t2\n' ],
	[ 'object with commented key', '* \n\ta \\1\n\t- b \\2\n' ],
	[ 'array with commented item', '/ \n\t\\a\n\t- \\b\n' ],
	[ 'empty object', '*\n' ],
	[ 'empty array', '/\n' ],
]

write( 'to_json', to_json_cases.map( ( [ name, input ] )=> ( {
	name,
	input,
	json: $$.$mol_tree2_to_json( parse( input ) ) ?? null,
} ) ) )

const from_json_cases = [
	[ 'string', 'hello' ],
	[ 'multiline string', 'a\nb' ],
	[ 'number', 1 ],
	[ 'float', 1.5 ],
	[ 'true', true ],
	[ 'false', false ],
	[ 'null', null ],
	[ 'empty object', {} ],
	[ 'object', { a: 1 } ],
	[ 'object with two keys', { a: 1, b: 2 } ],
	[ 'nested object', { a: { b: 1 } } ],
	[ 'key needing escape', { 'a b': 1 } ],
	[ 'key with newline', { 'a\nb': 1 } ],
	[ 'empty array', [] ],
	[ 'array', [ 'a', 'b' ] ],
	[ 'array of numbers', [ 1, 2 ] ],
	[ 'mixed', { list: [ 1, 'two', true, null ] } ],
]

write( 'from_json', from_json_cases.map( ( [ name, json ] )=> ( {
	name,
	json,
	output: String( $$.$mol_tree2_from_json( json ) ),
} ) ) )

// -------------------------------------------------------------- json round

// Every from_json output must parse back to the same JSON.
for( const [ name, json ] of from_json_cases ) {
	const str = String( $$.$mol_tree2_from_json( json ) )
	const back = $$.$mol_tree2_to_json( parse( str ) )
	const a = JSON.stringify( back ), b = JSON.stringify( json )
	if( a !== b ) console.warn( `  ! json round-trip differs for "${ name }": ${ b } -> ${ a }` )
}

// ----------------------------------------------------------------- spans

const span_cases = [
	[ 'inline chain', 'foo bar \\baz\n' ],
	[ 'nested', 'a\n\tb\n\t\tc\n' ],
	[ 'second row', 'a\nb\n' ],
	[ 'data after types', 'a b \\text\n' ],
	[ 'base indent stripped', '\n\t\ta b\n' ],
	[ 'unicode columns', 'привет \\мир\n' ],
]

write( 'spans', span_cases.map( ( [ name, input ] )=> ( {
	name,
	uri: 'test',
	input,
	tree: dump( parse( input ), input ),
} ) ) )

// ------------------------------------------------------- reference bugs

// Cases where the reference is demonstrably wrong. These expectations are
// hand-written — they describe what a PORT must do, not what the reference
// does. See SPEC.md "Known reference bugs".
write( 'reference_bugs', [
	{
		name: 'negative index in select matches nothing',
		op: 'select',
		input: 'a\n\tx\n\ty\n',
		path: [ 'a', -1 ],
		output: '',
		reference: 'pushes an undefined kid, then crashes on serialization',
	},
	{
		name: 'negative index in select, deeper',
		op: 'select',
		input: 'a b c d\n',
		path: [ 'a', -2 ],
		output: '',
		reference: 'pushes an undefined kid, then crashes on serialization',
	},
	{
		name: 'empty update does not create a missing two-step tail',
		op: 'update',
		input: 'a b\n',
		update: [],
		path: [ 'a', 'z', 'q' ],
		output: 'a b\n',
		reference: 'creates `z`, because an empty array is truthy in JS',
	},
	{
		name: 'empty update does not create a missing three-step tail',
		op: 'update',
		input: 'a b\n',
		update: [],
		path: [ 'a', 'z', 'q', 'w' ],
		output: 'a b\n',
		reference: 'creates `z q`, because an empty array is truthy in JS',
	},
	{
		name: 'dedented last line without a terminator still fails',
		op: 'parse',
		input: '\t\tfoo\n\tbar',
		error: { reason: 'Too few tabs', row: 2, col: 1, length: 1 },
		reference: 'returns the tree parsed so far, silently dropping the line',
	},
	{
		name: 'deeply dedented last line without a terminator still fails',
		op: 'parse',
		input: '\t\ta\nb',
		error: { reason: 'Too few tabs', row: 2, col: 1, length: 0 },
		reference: 'throws RangeError: Invalid array length',
	},
	{
		// Adding a newline must not change which error is reported.
		name: 'dedented last line with a terminator fails the same way',
		op: 'parse',
		input: '\t\tfoo\n\tbar\n',
		error: { reason: 'Too few tabs', row: 2, col: 1, length: 1 },
		reference: 'agrees',
	},
	{
		name: 'deleting an absent path is a no-op',
		op: 'insert',
		input: 'config\n\tport \\8080\n',
		insert: null,
		path: [ 'config', 'theme', 'dark' ],
		output: 'config port \\8080\n',
		reference: 'creates an empty `theme`, since insert(null) is update([])',
	},
	{
		// The reference is already correct here — pinned so a port does not
		// over-correct and start skipping legitimate creation.
		name: 'empty update leaves a missing one-step tail alone',
		op: 'update',
		input: 'a b\n',
		update: [],
		path: [ 'a', 'z' ],
		output: 'a b\n',
		reference: 'agrees',
	},
	{
		// Likewise: a non-empty update MUST still create the missing path.
		// Note the last step is replaced by the value rather than created,
		// because update with an empty path returns the value itself.
		name: 'non-empty update still creates a missing path',
		op: 'update',
		input: 'a b\n',
		update: [ 'x' ],
		path: [ 'a', 'z', 'q' ],
		output: 'a\n\tb\n\tz x\n',
		reference: 'agrees',
	},
] )

// Cases tagged `reference: 'agrees'` are pinned so that a port does not
// over-correct. Assert they really do match the reference, so a typo in an
// expectation above cannot quietly send every port chasing a phantom.
{
	const bugs = JSON.parse( fs.readFileSync( path.join( out_dir, 'reference_bugs.json' ), 'utf8' ) )
	const run = {
		select: c => String( parse( c.input ).select( ...c.path ) ),
		update: c => String( parse( c.input ).update( c.update.map( t => $mol_tree2.struct( t ) ), ...c.path )[ 0 ] ),
		insert: c => String( parse( c.input ).insert( c.insert === null ? null : $mol_tree2.struct( c.insert ), ...c.path ) ),
		// For a case that must fail, the "output" being compared is the reason.
		parse: c => {
			parse( c.input )
			return '<parsed without error>'
		},
	}
	for( const c of bugs ) {
		const expected = c.op === 'parse' ? c.error.reason : c.output
		let actual
		try {
			actual = run[ c.op ]( c )
		} catch( error ) {
			actual = error.reason ?? `<${ error.constructor.name }>`
		}
		const agrees = c.reference === 'agrees'
		if( agrees && actual !== expected ) {
			throw new Error(
				`reference_bugs "${ c.name }" is tagged as agreeing with the reference, `
				+ `but the reference gives ${ JSON.stringify( actual ) } `
				+ `and the fixture expects ${ JSON.stringify( expected ) }`
			)
		}
		if( !agrees && actual === expected ) {
			throw new Error(
				`reference_bugs "${ c.name }" claims the reference diverges, but it agrees`
			)
		}
	}
	console.log( `reference_bugs.json — ${ bugs.filter( c => c.reference !== 'agrees' ).length } divergences, `
		+ `${ bugs.filter( c => c.reference === 'agrees' ).length } agreements re-checked` )
}

console.log( '\nfixtures regenerated' )
