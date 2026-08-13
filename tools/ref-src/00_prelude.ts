type $ = any
namespace $ {
	export const $mol_ambient_ref = Symbol('$mol_ambient_ref')
	export class $mol_object2 {
		[Symbol.toStringTag]: any
		get $(): any { return $ }
		set $( next: any ) {}
	}
	export function $mol_fail( error: any ): never { throw error }
	export function $mol_fail_hidden( error: any ): never { throw error }
	export function $mol_maybe< V >( value: V | null | undefined ): V[] { return ( value == null ) ? [] : [ value ] }
}
