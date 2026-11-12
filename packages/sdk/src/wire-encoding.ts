// Lossless JSON transport for configuration data crossing into Rust.
//
// Values are data: functions, symbols, cycles, non-finite numbers, unsafe integer
// numbers and non-plain objects are refused rather than silently dropped or
// rounded. `bigint` crosses as an exact decimal token.

export type WireEncoding =
	| { readonly kind: 'encoded'; readonly json: string }
	| { readonly kind: 'rejected'; readonly message: string };

interface RawJsonCapableJson extends JSON {
	rawJSON(text: string): unknown;
}

function supportsRawJson(json: JSON): json is RawJsonCapableJson {
	return 'rawJSON' in json && typeof json.rawJSON === 'function';
}

class TransportRefusal extends Error {}

function describePath(path: readonly string[]): string {
	return path.length === 0 ? 'the value' : path.join('.');
}

function isPlainObject(value: object): boolean {
	const prototype: unknown = Object.getPrototypeOf(value);
	return prototype === Object.prototype || prototype === null;
}

/** Define an own data property, so keys such as `__proto__` stay ordinary data. */
function setData(record: Record<string, unknown>, key: string, value: unknown): void {
	Object.defineProperty(record, key, {
		value,
		enumerable: true,
		writable: true,
		configurable: true,
	});
}

/** Copy a value into plain JSON-safe data, refusing anything that cannot cross exactly. */
function copyData(
	value: unknown,
	path: string[],
	ancestors: Set<object>,
	rawJson: RawJsonCapableJson,
): unknown {
	switch (typeof value) {
		case 'string':
		case 'boolean':
			return value;
		case 'bigint':
			return rawJson.rawJSON(value.toString());
		case 'number':
			if (!Number.isFinite(value)) {
				throw new TransportRefusal(`${describePath(path)} is not a finite number`);
			}
			if (Number.isInteger(value) && !Number.isSafeInteger(value)) {
				throw new TransportRefusal(
					`${describePath(path)} is an unsafe integer; use bigint for exact values`,
				);
			}
			return value;
		case 'undefined':
			throw new TransportRefusal(`${describePath(path)} is undefined`);
		case 'function':
		case 'symbol':
			throw new TransportRefusal(`${describePath(path)} is a ${typeof value}, not data`);
		case 'object': {
			if (value === null) return null;
			if (ancestors.has(value)) {
				throw new TransportRefusal(`${describePath(path)} is a cycle`);
			}
			ancestors.add(value);
			try {
				if (Array.isArray(value)) {
					if (Object.getOwnPropertySymbols(value).length > 0) {
						throw new TransportRefusal(`${describePath(path)} has symbol-keyed properties`);
					}
					const items: unknown[] = [];
					for (let index = 0; index < value.length; index += 1) {
						if (!Object.hasOwn(value, index)) {
							throw new TransportRefusal(
								`${describePath([...path, String(index)])} is a sparse array hole`,
							);
						}
						items.push(copyData(value[index], [...path, String(index)], ancestors, rawJson));
					}
					return items;
				}
				if (!isPlainObject(value)) {
					throw new TransportRefusal(`${describePath(path)} is not a plain object`);
				}
				if (Object.getOwnPropertySymbols(value).length > 0) {
					throw new TransportRefusal(`${describePath(path)} has symbol-keyed properties`);
				}
				const copied: Record<string, unknown> = {};
				for (const key of Object.getOwnPropertyNames(value)) {
					const item: unknown = Reflect.get(value, key);
					// An absent optional property may be written as undefined.
					if (item === undefined) continue;
					setData(copied, key, copyData(item, [...path, key], ancestors, rawJson));
				}
				return copied;
			} finally {
				ancestors.delete(value);
			}
		}
	}
	throw new TransportRefusal(`${describePath(path)} has an unsupported type`);
}

/** Encode configuration data for the native contract decoder. */
export function encodeForNative(value: unknown): WireEncoding {
	if (!supportsRawJson(JSON)) {
		return { kind: 'rejected', message: 'This runtime lacks JSON.rawJSON for exact integers.' };
	}
	try {
		const data = copyData(value, [], new Set(), JSON);
		return { kind: 'encoded', json: JSON.stringify(data) };
	} catch (error) {
		// Throwing getters, proxies and similar inspection failures are refusals too.
		const message = error instanceof Error ? error.message : String(error);
		return {
			kind: 'rejected',
			message:
				error instanceof TransportRefusal
					? message
					: `the value could not be inspected as data: ${message}`,
		};
	}
}
