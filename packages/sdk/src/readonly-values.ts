// Deeply readonly views of contract data, enforced at runtime for loaded values.

/** A value whose objects, arrays and records are readonly all the way down. */
export type DeepReadonly<TValue> = TValue extends
	| bigint
	| boolean
	| number
	| string
	| null
	| undefined
	? TValue
	: TValue extends readonly (infer TItem)[]
		? readonly DeepReadonly<TItem>[]
		: { readonly [TKey in keyof TValue]: DeepReadonly<TValue[TKey]> };

/** Freeze a freshly decoded value so a held snapshot cannot be changed in place. */
export function deepFreeze<TValue>(value: TValue): DeepReadonly<TValue>;
export function deepFreeze(value: unknown): unknown {
	if (typeof value === 'object' && value !== null && !Object.isFrozen(value)) {
		for (const item of Object.values(value)) deepFreeze(item);
		Object.freeze(value);
	}
	return value;
}
