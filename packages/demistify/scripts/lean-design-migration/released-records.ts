import typescript from 'typescript';
import { z } from 'zod';

import {
	assumptionRecord,
	choiceRecord,
	decisionActorField,
	decisionSource,
	recordActor,
	rulingRecord,
} from '../../src/core/decision-record.ts';
import { readLiteral, unwrapExpression } from '../../src/node/design-authoring/read-design.ts';

const text = z.string().refine((value) => value.trim().length > 0);
const releasedSource = decisionSource.extend({ href: text, excerpt: text });
const weight = z.enum(['design', 'process', 'minor']);
const identity = z.object({
	id: text,
	title: text.optional(),
	sources: z.array(releasedSource).optional(),
});
const answer = { decision: text, decidedOn: z.iso.date() };
const releasedDecision = identity
	.extend({
		kind: z.literal('choice').optional(),
		supersedes: text.optional(),
		authority: z.discriminatedUnion('state', [
			z.strictObject({ state: z.literal('owner-open'), owner: z.literal('owner'), question: text }),
			z.strictObject({
				state: z.literal('owner-decided'),
				owner: z.literal('owner'),
				sourceKind: z.enum(['owner-words', 'owner-accepted']),
				...answer,
			}),
			z.strictObject({
				state: z.literal('lead-decided'),
				lead: text,
				sourceKind: z.enum(['author', 'contract']),
				rationale: text,
				...answer,
			}),
		]),
	})
	.strict();
const releasedAssumption = identity
	.extend({
		kind: z.literal('assumption').optional(),
		statement: text,
		status: z.enum(['open', 'confirmed', 'refuted']),
		resolution: releasedSource.optional(),
	})
	.strict();

export const releasedRecordInput = z.record(
	z.string(),
	z.strictObject({
		weight: weight.nullish(),
		[decisionActorField]: recordActor.nullish(),
		question: text.nullish(),
		options: choiceRecord.shape.options.nullish(),
		recommended: choiceRecord.shape.recommended.nullish(),
		source: decisionSource.nullish(),
		settles: text.nullish(),
		settledBy: text.nullish(),
		resolution: decisionSource.nullish(),
	}),
);

export type ReleasedRecordInput = z.infer<typeof releasedRecordInput>;
type RecordInput = ReleasedRecordInput[string];
type ConvertedRecord = z.infer<typeof choiceRecord | typeof rulingRecord | typeof assumptionRecord>;
type Converted = { readonly record: ConvertedRecord } | { readonly missing: RecordInput };
type RecordAnnotations = Pick<RecordInput, 'weight' | 'by' | 'settles' | 'settledBy'>;

function validatedRecord(record: ConvertedRecord): Converted {
	const schema =
		record.kind === 'assumption'
			? assumptionRecord
			: record.kind === 'ruling'
				? rulingRecord
				: choiceRecord;
	schema.parse(record);
	return { record };
}

function convertDecision(
	value: unknown,
	input: RecordInput,
	commentedWeight: RecordInput['weight'],
): Converted {
	const { authority, kind: _kind, ...identityFields } = releasedDecision.parse(value);
	if (authority.state === 'owner-open') {
		const { question = authority.question, options, recommended } = input;
		if (!options || !recommended)
			return {
				missing: {
					question: question ?? authority.question,
					...(!options && { options: null }),
					...(!recommended && { recommended: null }),
				},
			};
		return validatedRecord({
			...identityFields,
			kind: 'choice',
			question: question ?? authority.question,
			options,
			recommended,
			authority: { state: 'owner-open' },
		});
	}
	const { decision: statement, decidedOn } = authority;
	const source = identityFields.sources?.[0] ?? input.source;
	if (authority.state === 'owner-decided') {
		if (!source) return { missing: { source: null } };
		return validatedRecord({
			...identityFields,
			kind: 'ruling',
			statement,
			authority: { state: 'owner-decided', sourceKind: authority.sourceKind, decidedOn, source },
		});
	}
	const recordWeight = commentedWeight ?? input.weight;
	if (!recordWeight || (authority.sourceKind === 'contract' && !source))
		return {
			missing: {
				...(!recordWeight && { weight: null }),
				...(authority.sourceKind === 'contract' && !source && { source: null }),
			},
		};
	return validatedRecord({
		...identityFields,
		kind: 'ruling',
		statement,
		authority: {
			state: 'agent-decided',
			[decisionActorField]: { alias: authority.lead, role: 'lead' },
			weight: recordWeight,
			rationale: authority.rationale,
			decidedOn,
			...(authority.sourceKind === 'contract' && source
				? { sourceKind: 'contract', source }
				: { sourceKind: 'author' }),
		},
	});
}

function convertAssumption(
	value: unknown,
	input: RecordInput,
	commentedWeight: RecordInput['weight'],
): Converted {
	const { resolution, status, ...fields } = releasedAssumption.parse(value);
	const recordWeight = commentedWeight ?? input.weight;
	const { by: author, settledBy } = input;
	const settles = input.settles?.trim().toLowerCase() === 'unknown' ? undefined : input.settles;
	const source = resolution ?? input.resolution;
	const absent = {
		...(!recordWeight && { weight: null }),
		...(!author && { [decisionActorField]: null }),
	};
	if (status === 'open') {
		if (resolution)
			throw new Error(`${fields.id}: an open assumption cannot already have a resolution.`);
		if (!recordWeight || !author || !settles)
			return { missing: { ...absent, ...(!settles && { settles: null }) } };
		return validatedRecord({
			...fields,
			kind: 'assumption',
			status,
			[decisionActorField]: author,
			weight: recordWeight,
			settles,
		});
	}
	if (!recordWeight || !author || !settledBy || !source)
		return {
			missing: {
				...absent,
				...(!settledBy && { settledBy: null }),
				...(!source && { resolution: null }),
			},
		};
	return validatedRecord({
		...fields,
		kind: 'assumption',
		status,
		[decisionActorField]: author,
		weight: recordWeight,
		settledBy,
		resolution: source,
	});
}

function recordAnnotations(leadingText: string): RecordAnnotations {
	const annotations: RecordAnnotations = {};
	const lines = leadingText.split(/\r?\n/u).slice(0, -1).toReversed();
	for (const line of lines) {
		const match = line.match(/^\s*\/\/\s*(weight|by|settles|settledBy):\s*(.+?)\s*$/u);
		if (!match?.[2]) break;
		const [, field, content] = match;
		if (field === 'weight') annotations.weight = weight.parse(content);
		if (field === 'by') {
			const actorMatch = content.match(/^(\S+)\s+(\S+)$/u);
			annotations[decisionActorField] = recordActor.parse({
				alias: actorMatch?.[1],
				role: actorMatch?.[2],
			});
		}
		if (field === 'settles' && content.toLowerCase() !== 'unknown') annotations.settles = content;
		if (field === 'settledBy') annotations.settledBy = content;
	}
	return annotations;
}

interface ReleasedSidecar {
	readonly file: string;
	readonly source: string;
	readonly namespace: string;
	readonly type: 'decision' | 'assumption';
	readonly input: ReleasedRecordInput;
}

export function migrateReleasedSidecar({ file, source, namespace, type, input }: ReleasedSidecar): {
	readonly output: string;
	readonly records: readonly { id: string; kind: ConvertedRecord['kind'] }[];
	readonly missing: readonly (readonly [string, RecordInput])[];
} {
	const parsed = typescript.createSourceFile(file, source, typescript.ScriptTarget.ES2022, true);
	const exported = parsed.statements.filter(typescript.isExportAssignment);
	const [entry] = exported;
	const list = entry && unwrapExpression(entry.expression);
	if (
		exported.length !== 1 ||
		!entry ||
		entry.isExportEquals ||
		!list ||
		!typescript.isArrayLiteralExpression(list)
	)
		throw new Error(`${file}: expected one exported array of released records.`);
	const helperName = type === 'decision' ? 'defineDecision' : 'defineAssumption';
	const helperImport = parsed.statements.find(
		(statement) =>
			typescript.isImportDeclaration(statement) &&
			typescript.isStringLiteral(statement.moduleSpecifier) &&
			statement.moduleSpecifier.text.endsWith('/design-sdk.generated.ts') &&
			statement.importClause?.namedBindings &&
			typescript.isNamedImports(statement.importClause.namedBindings) &&
			statement.importClause.namedBindings.elements.some(
				(binding) => (binding.propertyName ?? binding.name).text === helperName,
			),
	);
	const binding =
		helperImport && typescript.isImportDeclaration(helperImport)
			? helperImport.importClause?.namedBindings
			: undefined;
	const helper =
		binding && typescript.isNamedImports(binding)
			? binding.elements.find(
					(candidate) => (candidate.propertyName ?? candidate.name).text === helperName,
				)
			: undefined;
	const helpers = new Set<string>();
	const records: { id: string; kind: ConvertedRecord['kind'] }[] = [];
	const missing: [string, RecordInput][] = [];
	const edits: { start: number; end: number; value: string }[] = [];
	const seen = new Set<string>();
	for (const item of list.elements) {
		const call = unwrapExpression(item);
		const isCall = typescript.isCallExpression(call);
		const inputRecord = isCall ? call.arguments[0] : item;
		if (
			!inputRecord ||
			(isCall &&
				(call.arguments.length !== 1 ||
					!typescript.isIdentifier(call.expression) ||
					call.expression.text !== helper?.name.text))
		)
			throw new Error(`${file}: expected a literal record or the released SDK helper.`);
		const literal = readLiteral(inputRecord, parsed);
		const id = `${namespace}/${identity.parse(literal).id}`;
		if (seen.has(id)) throw new Error(`${file}: duplicate record ${id}.`);
		seen.add(id);
		const annotations = recordAnnotations(source.slice(item.getFullStart(), item.getStart(parsed)));
		const provided = Object.hasOwn(input, id) ? (input[id] ?? {}) : {};
		const converted =
			type === 'decision'
				? convertDecision(literal, provided, annotations.weight)
				: convertAssumption(literal, { ...provided, ...annotations }, annotations.weight);
		if ('missing' in converted) {
			missing.push([id, converted.missing]);
			continue;
		}
		const { record } = converted;
		records.push({ id, kind: record.kind });
		const { kind, ...fields } = record;
		const newHelper =
			kind === 'choice' ? 'defineChoice' : kind === 'ruling' ? 'defineRuling' : 'defineAssumption';
		if (isCall) helpers.add(newHelper);
		const value = JSON.stringify(isCall ? fields : record, null, 2).replaceAll('\n', '\n  ');
		edits.push({
			start: item.getStart(parsed),
			end: item.getEnd(),
			value: isCall ? `${newHelper}(${value})` : value,
		});
	}
	if (missing.length) return { output: source, records, missing };
	if (helper && helpers.size && type === 'decision')
		edits.push({
			start: helper.getStart(parsed),
			end: helper.getEnd(),
			value: [...helpers].join(', '),
		});
	let output = source;
	for (const edit of edits.toSorted((left, right) => right.start - left.start))
		output = output.slice(0, edit.start) + edit.value + output.slice(edit.end);
	return { output, records, missing };
}
