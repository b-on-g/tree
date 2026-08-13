// Transpiles the reference TypeScript implementation into tools/ref.js,
// which gen-fixtures.js loads to produce the conformance corpus.
//
// Needs a `tsc` on PATH or in ./node_modules/.bin. Sources live in
// tools/ref-src/ and are verbatim copies of the $mol originals plus a
// small prelude that stubs the parts of the $mol runtime they touch.

const { execFileSync } = require( 'child_process' )
const fs = require( 'fs' )
const path = require( 'path' )

const root = path.join( __dirname, '..' )
const src = path.join( __dirname, 'ref-src' )
const out = path.join( __dirname, 'ref.js' )

const local = path.join( root, 'node_modules', '.bin', 'tsc' )
const tsc = fs.existsSync( local ) ? local : 'tsc'

const files = fs.readdirSync( src ).filter( f => f.endsWith( '.ts' ) ).sort()
	.map( f => path.join( src, f ) )

execFileSync( tsc, [
	'--ignoreConfig',
	'--ignoreDeprecations', '6.0',
	'--noCheck',
	'--target', 'es2022',
	'--lib', 'es2023',
	'--module', 'none',
	'--outFile', out,
	...files,
], { stdio: 'inherit' } )

console.log( `built ${ path.relative( root, out ) }` )
