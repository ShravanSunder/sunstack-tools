import { dirname, join } from 'node:path';

import typescript from 'typescript';
import { z } from 'zod';

import {
	parseDeliveryEntries,
	type DeliveryEntry,
} from '../../src/node/design-authoring/delivery-model.ts';
import { readDeliveryEntries } from '../../src/node/design-authoring/read-delivery.ts';
import { readLiteral, unwrapExpression } from '../../src/node/design-authoring/read-design.ts';
import {
	authoredDesignSchema,
	designSchema,
	htmlDestination,
	recordId,
	recordReference,
	type DesignRecord,
} from '../../src/node/design-authoring/schema.ts';
import { containsPath } from '../../src/node/local-site.ts';

const nonempty = z.string().min(1);
const sourceReference = z.strictObject({
	href: nonempty,
	excerpt: nonempty,
	verbatim: z.literal(true).optional(),
});
const relationship = z.strictObject({
	relation: z.enum(['serves', 'governed-by', 'relies-on', 'realizes', 'checks']),
	targetId: recordReference,
});
const legacyFields = {
	id: recordId,
	title: nonempty,
	destination: htmlDestination,
	sources: z.array(sourceReference).min(1),
};
const relationships = z.array(relationship);
const legacyRecordSchema = z.discriminatedUnion('kind', [
	z.strictObject({ ...legacyFields, kind: z.literal('need'), statement: nonempty }),
	z.strictObject({ ...legacyFields, kind: z.literal('rule'), statement: nonempty, relationships }),
	z.strictObject({
		...legacyFields,
		kind: z.literal('realization'),
		status: z.literal('proposed'),
		explanation: nonempty,
		relationships,
	}),
	z.strictObject({
		...legacyFields,
		kind: z.literal('planned-check'),
		status: z.literal('planned'),
		relationships,
		cases: z.array(z.strictObject({ given: nonempty, expect: nonempty })).min(3),
	}),
]);
const legacyDecisionBody = { decision: nonempty, decidedOn: z.iso.date() };
const legacyDecisions = z.array(
	z.strictObject({
		...legacyFields,
		sources: z.tuple([sourceReference], sourceReference),
		kind: z.literal('choice'),
		supersedes: recordReference.optional(),
		authority: z.discriminatedUnion('state', [
			z.strictObject({
				state: z.literal('owner-open'),
				owner: z.literal('owner'),
				question: nonempty,
			}),
			z.strictObject({
				state: z.literal('owner-decided'),
				owner: z.literal('owner'),
				sourceKind: z.enum(['owner-words', 'owner-accepted']),
				...legacyDecisionBody,
			}),
			z.strictObject({
				state: z.literal('lead-decided'),
				lead: nonempty,
				sourceKind: z.enum(['author', 'contract']),
				rationale: nonempty,
				...legacyDecisionBody,
			}),
		]),
	}),
);
const legacyAssumptions = z.array(
	z
		.strictObject({
			...legacyFields,
			kind: z.literal('assumption'),
			statement: nonempty,
			status: z.enum(['open', 'confirmed', 'refuted']),
			resolution: sourceReference.optional(),
		})
		.refine(
			({ status, resolution }) => status !== 'open' || resolution === undefined,
			'An open assumption cannot already have a resolution.',
		),
);
const legacyDesignSchema = authoredDesignSchema.extend({
	records: z.array(legacyRecordSchema),
	acceptances: z
		.array(
			z.strictObject({
				deliverableId: recordId,
				build: nonempty,
				scopeRevision: nonempty,
				evidence: nonempty,
			}),
		)
		.optional(),
});
const storedReceipt = z.looseObject({ scopeRevision: nonempty.optional() });
const legacyDeliverable = z.looseObject({
	verification: storedReceipt.optional(),
	verificationHistory: z.array(storedReceipt).optional(),
});

export type LegacyDesign = z.infer<typeof legacyDesignSchema>;
export type LegacyRecord =
	| LegacyDesign['records'][number]
	| z.infer<typeof legacyDecisions>[number]
	| z.infer<typeof legacyAssumptions>[number];
export type SourceReader = (path: string) => Promise<string | undefined>;

function validated<TSchema extends z.ZodType>(
	schema: TSchema,
	value: unknown,
	path: string,
): z.output<TSchema> {
	const result = schema.safeParse(value);
	if (!result.success) throw new Error(`${path}: ${result.error.message}`);
	return result.data;
}

function literalModule(path: string, text: string): unknown {
	const source = typescript.createSourceFile(path, text, typescript.ScriptTarget.ES2022, true);
	const exports = source.statements.filter(typescript.isExportAssignment);
	const [entry] = exports;
	if (
		exports.length !== 1 ||
		!entry ||
		entry.isExportEquals ||
		source.statements.some(
			(statement): boolean =>
				statement !== entry &&
				!(typescript.isImportDeclaration(statement) && statement.importClause?.isTypeOnly),
		)
	)
		throw new Error(`${path}: expected a legacy default literal and type-only imports.`);
	return readLiteral(unwrapExpression(entry.expression), source);
}

export function readLegacyDesign(path: string, text: string): LegacyDesign {
	return validated(legacyDesignSchema, literalModule(path, text), path);
}

export function readLegacySidecar(
	path: string,
	text: string,
	kind: 'choice' | 'assumption',
): LegacyRecord[] {
	const input = literalModule(path, text);
	return kind === 'choice'
		? validated(legacyDecisions, input, path)
		: validated(legacyAssumptions, input, path);
}

export interface LegacyDeliveryGraph {
	readonly entries: readonly DeliveryEntry[];
	readonly modules: ReadonlyMap<string, string>;
}

function removeScopeRevision({
	scopeRevision: _scopeRevision,
	...receipt
}: z.infer<typeof storedReceipt>): Record<string, unknown> {
	return receipt;
}

function leanDeliverable(raw: unknown, path: string): Record<string, unknown> {
	const { verification, verificationHistory, ...fields } = validated(legacyDeliverable, raw, path);
	if (Object.hasOwn(fields, 'kind') || !Object.hasOwn(fields, 'scope'))
		throw new Error(`${path}: expected a legacy scoped deliverable.`);
	const deliverable = {
		...fields,
		kind: 'core',
		...(verification && { verification: removeScopeRevision(verification) }),
		...(verificationHistory && {
			verificationHistory: verificationHistory.map(removeScopeRevision),
		}),
	};
	try {
		parseDeliveryEntries([{ deliverable }]);
	} catch (error) {
		throw new Error(`${path}: ${error instanceof Error ? error.message : String(error)}`, {
			cause: error,
		});
	}
	return deliverable;
}

/** Reuse the product reader to validate the import graph and interpret the old SDK calls as data. */
export async function readLegacyDelivery(
	designFile: string,
	designText: string,
	read: SourceReader,
): Promise<LegacyDeliveryGraph> {
	const directory = dirname(designFile);
	const deliveryDirectory = join(directory, 'deliverables');
	const originals = new Map<string, string>();
	const initial = await readDeliveryEntries(
		designFile,
		designText,
		async (path) => {
			const text = await read(path);
			if (text !== undefined && containsPath(deliveryDirectory, path)) originals.set(path, text);
			return text;
		},
		new Map(),
	);
	const modules = new Map<string, string>();
	for (const [path, text] of originals) {
		const source = typescript.createSourceFile(path, text, typescript.ScriptTarget.ES2022, true);
		const exported = source.statements.find(typescript.isExportAssignment);
		const call = exported && unwrapExpression(exported.expression);
		const literal = call && typescript.isCallExpression(call) ? call.arguments[0] : undefined;
		if (!literal) throw new Error(`${path}: expected a legacy deliverable declaration.`);
		const data = leanDeliverable(readLiteral(literal, source), path);
		modules.set(
			path,
			`${text.slice(0, literal.getStart(source))}${JSON.stringify(data, null, 2)}${text.slice(literal.end)}`,
		);
	}
	const entrySchema = z.array(
		z.strictObject({ deliverable: z.unknown(), dependsOn: z.array(recordReference).optional() }),
	);
	const entries = validated(
		entrySchema,
		initial.entries,
		join(directory, 'design-delivery.ts'),
	).map((entry) =>
		Object.assign({}, entry, { deliverable: leanDeliverable(entry.deliverable, designFile) }),
	);
	const rewritten = await readDeliveryEntries(
		designFile,
		designText,
		async (path) => modules.get(path) ?? read(path),
		new Map(),
	);
	try {
		parseDeliveryEntries(rewritten.entries);
	} catch (error) {
		throw new Error(
			`${directory}: expected all legacy deliverables to be declared in imported modules.`,
			{ cause: error },
		);
	}
	return { entries: parseDeliveryEntries(entries), modules };
}

const helperByKind = {
	need: 'defineNeed',
	rule: 'defineRule',
	realization: 'defineRealization',
	'planned-check': 'defineCheck',
	choice: 'defineChoice',
	ruling: 'defineRuling',
	assumption: 'defineAssumption',
} satisfies Record<DesignRecord['kind'], string>;

function recordCall({ kind, ...fields }: DesignRecord): string {
	return `${helperByKind[kind]}(${JSON.stringify(fields, null, 2)})`;
}

function recordCalls(records: readonly DesignRecord[]): string {
	return records
		.map((record): string =>
			recordCall(record)
				.split('\n')
				.map((line): string => `  ${line}`)
				.join('\n'),
		)
		.join(',\n');
}

export function emitDesign(legacy: LegacyDesign, records: readonly DesignRecord[]): string {
	const { records: _previousRecords, acceptances, ...navigation } = legacy;
	const fields = {
		...navigation,
		...(acceptances && {
			acceptances: acceptances.map(({ scopeRevision: _scopeRevision, ...receipt }) => receipt),
		}),
	};
	const helpers = new Set([
		'defineDesign',
		...records.map(({ kind }): string => helperByKind[kind]),
	]);
	return `import { ${[...helpers].join(', ')} } from './design-sdk.generated.ts';\n\nexport default defineDesign(${JSON.stringify(fields, null, 2).slice(0, -2)},\n  records: [\n${recordCalls(records)}\n  ],\n});\n`;
}

export function emitSidecar(
	records: readonly DesignRecord[],
	original: string,
	shared: boolean,
): string {
	if (shared) {
		const parsed = typescript.createSourceFile(
			'shared-records.ts',
			original,
			typescript.ScriptTarget.ES2022,
			true,
		);
		const entry = parsed.statements.find(typescript.isExportAssignment);
		if (!entry || entry.isExportEquals) throw new Error('A shared sidecar needs a default export.');
		const imports = original.slice(0, entry.getStart(parsed));
		return `${imports}export default ${JSON.stringify(records, null, 2)} as const;\n`;
	}
	const helpers = new Set(records.map(({ kind }): string => helperByKind[kind]));
	return `${helpers.size ? `import { ${[...helpers].join(', ')} } from './design-sdk.generated.ts';\n\n` : ''}export default [\n${recordCalls(records)}\n] as const;\n`;
}

export function checkLeanRecords(records: readonly DesignRecord[]): void {
	designSchema.shape.records.parse(records);
}
