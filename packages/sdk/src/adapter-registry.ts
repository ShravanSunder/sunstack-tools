import { builtinTranscriptFormats } from './contracts/contracts.ts';
import type * as Contracts from './contracts/contracts.ts';
import type { DeepReadonly } from './readonly-values.ts';
import { parseExactJson } from './reply-decoding.ts';
import { encodeForNative } from './wire-encoding.ts';

export type TranscriptAdapterMapping = DeepReadonly<Contracts.TranscriptAdapterMapping>;
export type TranscriptRecordMapping = DeepReadonly<Contracts.TranscriptRecordMapping>;

interface StandardJsonSchemaDocument {
	readonly [key: string]: unknown;
	readonly '~standard': {
		readonly version: 1;
		readonly vendor: string;
		readonly jsonSchema: {
			readonly input: StandardJsonSchemaConverter;
			readonly output: StandardJsonSchemaConverter;
		};
	};
}

// Identify producer output structurally. Registration encodes the document without calling converters.
type StandardJsonSchemaConverter = (options: {
	readonly target: 'draft-2020-12';
}) => Readonly<Record<string, unknown>>;

type SchemaDataValue =
	| Exclude<Contracts.EvidenceStructuredJson, object>
	| readonly SchemaDataValue[]
	| SchemaDataObject
	| StandardJsonSchemaDocument;

interface SchemaDataObject {
	readonly [key: string]: SchemaDataValue;
}

type TranscriptAdapterRegistration = Omit<TranscriptAdapterMapping, 'schema'> & {
	readonly schema?: boolean | SchemaDataObject | StandardJsonSchemaDocument;
};

export interface ConversationSource<TFormat extends string = Contracts.BuiltinTranscriptFormat> {
	readonly path: string;
	readonly format: TFormat;
	readonly finalReply?: string;
}

type ConversationBinding<TFormat extends string> = DeepReadonly<
	Extract<Contracts.InputBinding, { kind: 'conversation' }>
> & { readonly format: TFormat };

function createConversationBinding<const TFormat extends string>(
	input: ConversationSource<TFormat>,
): ConversationBinding<TFormat> {
	return {
		kind: 'conversation',
		transcript_path: input.path,
		format: input.format,
		...(input.finalReply === undefined ? {} : { final_assistant_text: input.finalReply }),
	};
}

export function conversation<const TFormat extends Contracts.BuiltinTranscriptFormat>(
	input: ConversationSource<TFormat>,
): ConversationBinding<TFormat> {
	return createConversationBinding(input);
}

const encodedRegistries = new WeakMap<object, ReadonlyMap<string, string>>();

function registeredMappings(registry: object): ReadonlyMap<string, string> {
	const mappings = encodedRegistries.get(registry);
	if (mappings === undefined)
		throw new TypeError('Supply an AdapterRegistry from this SDK instance.');
	return mappings;
}

function isBuiltinFormat(format: string): boolean {
	return builtinTranscriptFormats.some((builtin) => builtin === format);
}

/** Remove hidden Standard Schema metadata while keeping other values for the wire encoder. */
function copySchemaData(value: unknown, copies: WeakMap<object, object>): unknown {
	if (typeof value !== 'object' || value === null) return value;
	const existing = copies.get(value);
	if (existing !== undefined) return existing;
	const array = Array.isArray(value);
	if (!array) {
		const prototype: unknown = Object.getPrototypeOf(value);
		if (prototype !== Object.prototype && prototype !== null) return value;
	}
	const copied = array ? [] : {};
	// Keep shared references and cycles intact so the wire encoder handles each as usual.
	copies.set(value, copied);
	for (const key of Reflect.ownKeys(value)) {
		const field = Object.getOwnPropertyDescriptor(value, key);
		if (field === undefined) continue;
		if (!Object.hasOwn(field, 'value')) {
			throw new TypeError('The adapter schema must contain only data properties.');
		}
		if (key === '~standard' && field.enumerable === false) continue;
		Object.defineProperty(copied, key, { ...field, value: copySchemaData(field.value, copies) });
	}
	return copied;
}

function mappingWithSchemaData(mapping: TranscriptAdapterRegistration): unknown {
	const schemaDescriptor = Object.getOwnPropertyDescriptor(mapping, 'schema');
	if (schemaDescriptor === undefined) return mapping;
	if (!Object.hasOwn(schemaDescriptor, 'value')) {
		throw new TypeError('The adapter schema must be a data property.');
	}
	const schema: unknown = schemaDescriptor.value;
	if (typeof schema !== 'object' || schema === null) return mapping;
	const mappingPrototype: unknown = Object.getPrototypeOf(mapping);
	if (mappingPrototype !== Object.prototype && mappingPrototype !== null) return mapping;
	const mappingData: unknown = Object.create(mappingPrototype, {
		...Object.getOwnPropertyDescriptors(mapping),
		schema: { ...schemaDescriptor, value: copySchemaData(schema, new WeakMap()) },
	});
	return mappingData;
}

type IsUnion<TMember, TWhole = TMember> = TMember extends unknown
	? [TWhole] extends [TMember]
		? false
		: true
	: never;

type NewFormat<TAdded extends string, TRegistered extends string> = TAdded &
	(true extends IsUnion<TAdded> ? never : unknown) &
	// A symbol-keyed record has no concrete string property to satisfy a literal name.
	(Record<symbol, never> extends Record<TAdded, never> ? never : unknown) &
	(Extract<TAdded, TRegistered> extends never ? unknown : never);

/** An immutable snapshot of built-in and declarative transcript adapters. */
export class AdapterRegistry<TRegistered extends string> {
	private constructor(mappings: ReadonlyMap<string, string>) {
		encodedRegistries.set(this, mappings);
	}

	static builtins(): AdapterRegistry<Contracts.BuiltinTranscriptFormat> {
		return new AdapterRegistry(new Map());
	}

	withAdapter<const TAdded extends string>(
		mapping: Omit<TranscriptAdapterRegistration, 'format'> & {
			readonly format: NewFormat<TAdded, TRegistered>;
		},
	): AdapterRegistry<TRegistered | TAdded> {
		const current = registeredMappings(this);
		const encoded = encodeForNative(mappingWithSchemaData(mapping));
		if (encoded.kind === 'rejected') throw new TypeError(encoded.message);
		const snapshot: unknown = parseExactJson(encoded.json);
		if (typeof snapshot !== 'object' || snapshot === null || !('format' in snapshot)) {
			throw new TypeError('Supply a transcript adapter definition with a format name.');
		}
		const format = snapshot.format;
		if (typeof format !== 'string' || format.length === 0) {
			throw new TypeError('Supply a nonempty transcript adapter format name.');
		}
		if (isBuiltinFormat(format) || current.has(format)) {
			throw new TypeError(`Transcript format ${format} is already registered.`);
		}
		const added = new Map(current);
		added.set(format, encoded.json);
		return new AdapterRegistry(added);
	}

	conversation<const TFormat extends TRegistered>(
		input: ConversationSource<TFormat>,
	): ConversationBinding<TFormat> {
		if (!isBuiltinFormat(input.format) && !registeredMappings(this).has(input.format)) {
			throw new TypeError(`Transcript format ${input.format} is not registered.`);
		}
		return createConversationBinding(input);
	}
}

/** Combine already encoded definitions without a lossy JavaScript number round trip. */
export function encodeAdapterMappings(registry: AdapterRegistry<string>): string | undefined {
	const mappings = registeredMappings(registry);
	return mappings.size === 0 ? undefined : `[${Array.from(mappings.values()).join(',')}]`;
}
