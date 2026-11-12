// The format of the generated full-width integer map (see wide-integers.ts).

/** Where full-width integers sit inside one serde wire shape. */
export type WideShape =
	| 'bigint'
	| { readonly list: WideShape }
	| { readonly map: WideShape }
	| { readonly object: Readonly<Record<string, WideShape>> }
	| { readonly union: readonly WideShape[] }
	| { readonly all: readonly WideShape[] }
	| { readonly tuple: readonly (WideShape | null)[] }
	| { readonly generic: string }
	| { readonly ref: string; readonly arguments: Readonly<Record<string, WideShape | null>> };

/** A named contract type's generic parameters and full-width integer positions. */
export interface WideDefinition {
	readonly generics: readonly string[];
	readonly shape: WideShape;
}
