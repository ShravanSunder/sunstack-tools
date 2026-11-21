import { builtinTranscriptFormats } from './contracts/contracts.ts';
import type * as Contracts from './contracts/contracts.ts';
import type { DeepReadonly } from './readonly-values.ts';
import { parseExactJson } from './reply-decoding.ts';
import { encodeForNative } from './wire-encoding.ts';

export type TranscriptAdapterMapping = DeepReadonly<Contracts.TranscriptAdapterMapping>;
export type TranscriptRecordMapping = DeepReadonly<Contracts.TranscriptRecordMapping>;

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

/** Schema producers can attach hidden Standard Schema metadata to a JSON Schema document. */
function mappingWithSchemaData(mapping: TranscriptAdapterMapping): unknown {
	const schemaDescriptor = Object.getOwnPropertyDescriptor(mapping, 'schema');
	if (schemaDescriptor === undefined) return mapping;
	if (!Object.hasOwn(schemaDescriptor, 'value')) {
		throw new TypeError('The adapter schema must be a data property.');
	}
	const schema: unknown = schemaDescriptor.value;
	if (
		typeof schema !== 'object' ||
		schema === null ||
		Object.getOwnPropertyDescriptor(schema, '~standard')?.enumerable !== false
	) {
		return mapping;
	}
	const schemaPrototype: unknown = Object.getPrototypeOf(schema);
	const mappingPrototype: unknown = Object.getPrototypeOf(mapping);
	if (
		(schemaPrototype !== Object.prototype && schemaPrototype !== null) ||
		(mappingPrototype !== Object.prototype && mappingPrototype !== null)
	) {
		return mapping;
	}
	// Copy descriptors so every other key remains subject to the ordinary wire encoder.
	const schemaFields = Object.getOwnPropertyDescriptors(schema);
	delete schemaFields['~standard'];
	const schemaData: unknown = Object.create(schemaPrototype, schemaFields);
	const mappingData: unknown = Object.create(mappingPrototype, {
		...Object.getOwnPropertyDescriptors(mapping),
		schema: { ...schemaDescriptor, value: schemaData },
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
	// An open string key set, including a template pattern, requires no concrete property.
	(Record<never, never> extends Record<TAdded, never> ? never : unknown) &
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
		mapping: Omit<TranscriptAdapterMapping, 'format'> & {
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
