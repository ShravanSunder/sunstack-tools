// Decode native replies exactly: full-width integers become bigint, and only the
// closed outcome variants of each call are accepted.
import type {
	CheckOutcome,
	DeserializationOutcome,
	InspectionOutcome,
	LoadOutcome,
	RunOutcome,
} from './contracts/contracts.ts';
import type { WideDefinition, WideShape } from './contracts/wide-integer-shapes.ts';
import { documentRootShapes, wideIntegerDefinitions } from './contracts/wide-integers.ts';

interface SourceContext {
	readonly source?: string;
}

const integerToken = /^-?\d+$/u;

/** Parse reply JSON, keeping integers beyond the safe range as exact bigint. */
export function parseExactJson(text: string): unknown {
	return JSON.parse(text, (_key: string, value: unknown, context?: SourceContext): unknown => {
		if (
			typeof value === 'number' &&
			context?.source !== undefined &&
			integerToken.test(context.source) &&
			!Number.isSafeInteger(value)
		) {
			return BigInt(context.source);
		}
		return value;
	});
}

interface BoundShape {
	readonly shape: WideShape;
	readonly bindings: ReadonlyMap<string, BoundShape | null>;
}

function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null && !Array.isArray(value);
}

/** Convert integers at full-width positions to bigint, in place. */
function widen(
	value: unknown,
	shape: WideShape,
	bindings: ReadonlyMap<string, BoundShape | null>,
	definitions: Readonly<Record<string, WideDefinition>>,
): unknown {
	if (shape === 'bigint') {
		return typeof value === 'number' && Number.isSafeInteger(value) ? BigInt(value) : value;
	}
	if ('list' in shape) {
		if (Array.isArray(value)) {
			value.forEach((item: unknown, index) => {
				value[index] = widen(item, shape.list, bindings, definitions);
			});
		}
		return value;
	}
	if ('map' in shape) {
		if (isRecord(value)) {
			for (const [key, item] of Object.entries(value)) {
				value[key] = widen(item, shape.map, bindings, definitions);
			}
		}
		return value;
	}
	if ('object' in shape) {
		if (isRecord(value)) {
			for (const [key, fieldShape] of Object.entries(shape.object)) {
				if (key in value) value[key] = widen(value[key], fieldShape, bindings, definitions);
			}
		}
		return value;
	}
	if ('union' in shape) {
		return shape.union.reduce<unknown>(
			(current, alternative) => widen(current, alternative, bindings, definitions),
			value,
		);
	}
	if ('all' in shape) {
		return shape.all.reduce<unknown>(
			(current, member) => widen(current, member, bindings, definitions),
			value,
		);
	}
	if ('tuple' in shape) {
		if (Array.isArray(value)) {
			shape.tuple.forEach((elementShape, index) => {
				if (elementShape !== null && index < value.length) {
					value[index] = widen(value[index], elementShape, bindings, definitions);
				}
			});
		}
		return value;
	}
	if ('generic' in shape) {
		const bound = bindings.get(shape.generic);
		return bound ? widen(value, bound.shape, bound.bindings, definitions) : value;
	}
	const definition = definitions[shape.ref];
	if (definition === undefined) return value;
	const inner = new Map<string, BoundShape | null>();
	definition.generics.forEach((generic) => {
		const argument = shape.arguments[generic];
		inner.set(
			generic,
			argument === undefined || argument === null ? null : { shape: argument, bindings },
		);
	});
	return widen(value, definition.shape, inner, definitions);
}

/** Apply a named root's full-width integer positions to a parsed reply. */
export function widenNamed(value: unknown, root: string): unknown {
	return widen(value, { ref: root, arguments: {} }, new Map(), wideIntegerDefinitions);
}

// Only the matched native addon can produce these envelopes. This is not a
// validator for arbitrary caller-supplied JSON.
function isDeserializationOutcome(value: unknown): value is DeserializationOutcome {
	if (!isRecord(value)) return false;
	switch (value['kind']) {
		case 'decoded':
			return (
				isRecord(value['value']) &&
				typeof value['value']['kind'] === 'string' &&
				Object.hasOwn(documentRootShapes, value['value']['kind']) &&
				isRecord(value['value']['value'])
			);
		case 'incompatible':
			return (
				typeof value['expected_contract_revision'] === 'number' &&
				typeof value['actual_contract_revision'] === 'number'
			);
		case 'invalid':
			return (
				typeof value['reason'] === 'string' &&
				['malformed_json', 'invalid_document', 'invalid_value', 'unsupported_number'].includes(
					value['reason'],
				) &&
				isArray(value['diagnostics'])
			);
		case 'failed':
			return isRecord(value['failure']);
		default:
			return false;
	}
}

interface DocumentReplyMetadata {
	readonly roots: Readonly<Record<string, WideShape | null>>;
	readonly definitions: Readonly<Record<string, WideDefinition>>;
}

const generatedDocumentMetadata: DocumentReplyMetadata = {
	roots: documentRootShapes,
	definitions: wideIntegerDefinitions,
};

/** Decode matched-native data; private fixture tests may substitute its generated shapes. */
export function decodeDeserializationReply(
	text: string,
	metadata: DocumentReplyMetadata = generatedDocumentMetadata,
): DeserializationOutcome | undefined {
	let parsed: unknown;
	try {
		parsed = parseExactJson(text);
	} catch {
		return undefined;
	}
	if (isRecord(parsed) && parsed['kind'] === 'decoded' && isRecord(parsed['value'])) {
		const document = parsed['value'];
		const root = Object.entries(metadata.roots).find(([kind]) => kind === document['kind']);
		if (root === undefined || !('value' in document)) return undefined;
		const shape: WideShape | null = root[1];
		if (shape !== null)
			document['value'] = widen(document['value'], shape, new Map(), metadata.definitions);
	}
	return isDeserializationOutcome(parsed) ? parsed : undefined;
}

function isArray(value: unknown): value is unknown[] {
	return Array.isArray(value);
}

function isObjectOrNull(value: unknown): boolean {
	return value === null || isRecord(value);
}

function hasStatus(report: unknown, statuses: readonly string[]): boolean {
	return (
		isRecord(report) &&
		isRecord(report['status']) &&
		typeof report['status']['kind'] === 'string' &&
		statuses.includes(report['status']['kind'])
	);
}

// These guards check the closed discriminant and the variant's envelope. The body
// comes from the matched Rust contract, verified by the release handshake.
export function isLoadOutcome(value: unknown): value is LoadOutcome {
	if (!isRecord(value)) return false;
	switch (value['kind']) {
		case 'loaded':
			return (
				isRecord(value['value']) &&
				isRecord(value['value']['configuration']) &&
				isArray(value['value']['sources'])
			);
		case 'invalid':
		case 'unavailable':
			return isArray(value['diagnostics']);
		case 'failed':
			return isRecord(value['failure']);
		default:
			return false;
	}
}

export function isCheckOutcome(value: unknown): value is CheckOutcome {
	if (!isRecord(value)) return false;
	switch (value['kind']) {
		case 'valid':
			return isRecord(value['summary']) && isArray(value['sources']);
		case 'invalid':
		case 'unavailable':
		case 'cancelled':
			return isArray(value['diagnostics']);
		case 'failed':
			return isRecord(value['failure']);
		default:
			return false;
	}
}

export function isRunOutcome(value: unknown): value is RunOutcome {
	if (!isRecord(value)) return false;
	const report = value['report'];
	switch (value['kind']) {
		case 'completed':
			return hasStatus(report, ['clean', 'nothing_selected', 'findings', 'uncertain']);
		case 'invalid':
			return isArray(value['diagnostics']) && (report === null || hasStatus(report, ['invalid']));
		case 'unavailable':
			return isArray(value['diagnostics']);
		case 'incomplete':
			return hasStatus(report, ['incomplete']);
		case 'cancelled':
			return report === null || hasStatus(report, ['cancelled']);
		case 'failed':
			return isRecord(value['failure']) && isObjectOrNull(report);
		default:
			return false;
	}
}

/** Check the envelope of a reply from the matched native inspection operation. */
export function isInspectionOutcome(value: unknown): value is InspectionOutcome {
	if (!isRecord(value)) return false;
	const report = value['report'];
	switch (value['kind']) {
		case 'inspected':
			return hasStatus(report, ['clean', 'uncertain']);
		case 'invalid':
		case 'unavailable':
			return isArray(value['diagnostics']);
		case 'incomplete':
			return hasStatus(report, ['incomplete']);
		case 'cancelled':
			return report === null || hasStatus(report, ['cancelled']);
		case 'failed':
			return isRecord(value['failure']) && isObjectOrNull(report);
		default:
			return false;
	}
}
