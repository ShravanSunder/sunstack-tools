/** Installed document-policy proof. Pure oracles never qualify a native target. */
import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import path from 'node:path';
import { isDeepStrictEqual } from 'node:util';

import { z } from 'zod';

import { boundedSdkWait } from './sdk-controlled-backend.ts';
import {
	createSdkDocumentInputs,
	sdkDocumentCases,
	sdkDocumentConfiguration,
	sdkDocumentFileConfiguration,
	sdkDocumentLinks,
	sdkDocumentLinkText,
	sdkDocumentPage,
	sdkDocumentRequest,
	sdkDocumentRootValue,
	type SdkDocumentCase,
} from './sdk-fixtures.ts';
import type { JourneyContext } from './sdk-journeys.ts';
import { requireSdk, saveSdkEvidence } from './sdk-target.ts';

const recordSchema = z.record(z.string(), z.unknown());
const integerSchema = z.union([z.bigint(), z.number().int()]).transform((value) => BigInt(value));
const spanSchema = z
	.object({
		byte_start: integerSchema,
		byte_end: integerSchema,
		source: z.object({ content_identity: z.string(), path: z.string() }).passthrough(),
	})
	.passthrough();
const pieceSchema = z
	.object({
		kind: z.string(),
		where: z
			.object({ file: z.string(), first_line: integerSchema, span: spanSchema })
			.passthrough(),
		fields: recordSchema,
		model_view: z.unknown(),
	})
	.passthrough();
const reportSchema = z
	.object({
		configuration: recordSchema,
		piece_evidence: z.record(z.string(), z.object({ piece: pieceSchema }).passthrough()),
		sources: z.array(
			z.object({ origin: recordSchema, content: z.string(), sha256: z.string() }).passthrough(),
		),
	})
	.passthrough();
type DocumentReport = z.infer<typeof reportSchema>;

function equal(actual: unknown, expected: unknown, label: string): void {
	requireSdk(isDeepStrictEqual(actual, expected), `Document settings: ${label} differs`);
}
function digest(content: string): string {
	return createHash('sha256').update(content).digest('hex');
}
function pieces(report: DocumentReport, kind: string): z.infer<typeof pieceSchema>[] {
	return Object.values(report.piece_evidence)
		.map((entry) => entry.piece)
		.filter((piece) => piece.kind === kind);
}

/** Exact selected headings and raw source spans; no outline is reconstructed from product output. */
function requireOutline(report: DocumentReport, selected: SdkDocumentCase): void {
	const names = sdkDocumentCases[selected].headings;
	const outline = pieces(report, 'outline').toSorted((left, right) =>
		Number(left.where.span.byte_start - right.where.span.byte_start),
	);
	equal(outline.length, names.length, 'outline count');
	const last = sdkDocumentPage.indexOf('<h2>Last');
	for (const [index, name] of names.entries()) {
		const piece = outline[index];
		requireSdk(piece, 'Missing required outline piece');
		const start = sdkDocumentPage.indexOf(
			`<h${name === 'Parent' || name === 'Last' ? 2 : 3}>${name}`,
		);
		const next = names[index + 1];
		const end =
			name === 'Parent'
				? last
				: next === undefined
					? sdkDocumentPage.length
					: sdkDocumentPage.indexOf(`<h${next === 'Last' ? 2 : 3}>${next}`);
		equal(
			[piece.where.file, piece.where.span.byte_start, piece.where.span.byte_end],
			['docs/page.html', BigInt(start), BigInt(end)],
			'heading identity/order/range',
		);
		const raw = z
			.object({ state: z.literal('text'), value: z.string(), spans: z.array(spanSchema).length(1) })
			.parse(piece.fields['raw']);
		equal(raw.value, sdkDocumentPage.slice(start, end), 'outline raw field');
		equal(
			[raw.spans[0]?.byte_start, raw.spans[0]?.byte_end],
			[BigInt(start), BigInt(end)],
			'outline raw span',
		);
		equal(
			piece.where.span.source.content_identity,
			digest(sdkDocumentPage),
			'outline source digest',
		);
		equal(raw.spans[0]?.source, piece.where.span.source, 'outline raw source');
	}
}

function expectedAnchor(name: string, github: boolean): unknown {
	if (name === 'heading' || name === 'missing') {
		return github
			? { status: name === 'heading' ? 'found' : 'missing', fragment: name }
			: { status: 'unsupported', fragment: name, reason: 'heading_scheme_not_configured' };
	}
	if (name === 'text')
		return { status: 'unsupported', fragment: 'part', reason: 'unsupported_document' };
	if (name === 'file' || name === 'absent') return undefined;
	return {
		status: name === 'html_missing' ? 'missing' : 'found',
		fragment: name === 'html' ? 'html-id' : name === 'html_missing' ? 'absent' : name,
	};
}

/** Per-owner results prevent aggregate incomplete from hiding a failed standard anchor. */
function requireTargets(report: DocumentReport, selected: SdkDocumentCase): void {
	const links = pieces(report, 'link').toSorted((left, right) =>
		Number(left.where.first_line - right.where.first_line),
	);
	equal(links.length, sdkDocumentLinks.length, 'target piece count');
	for (const [index, [name, target]] of sdkDocumentLinks.entries()) {
		const piece = links[index];
		requireSdk(piece, 'Missing required target piece');
		equal(
			[piece.where.file, piece.where.first_line],
			['docs/links.md', BigInt(index * 2 + 1)],
			'target owner',
		);
		equal(
			piece.where.span.source.content_identity,
			digest(sdkDocumentLinkText),
			'target source digest',
		);
		const field = recordSchema.parse(piece.fields['targets']);
		if (name === 'outside') {
			equal(field, { state: 'unavailable', reason: 'outside_root' }, 'outside-root state');
			continue;
		}
		equal(field['state'], 'targets', 'target state');
		const values = z.array(recordSchema).length(1).parse(field['values']);
		const value = values[0];
		requireSdk(value, 'Missing target value');
		equal([value['name'], value['exists']], [target, name !== 'absent'], 'target name/existence');
		equal(
			value['anchor'],
			expectedAnchor(name, sdkDocumentCases[selected].github),
			'closed target anchor',
		);
	}
}

export interface DocumentObservationInput {
	readonly report: unknown;
	readonly requests: readonly unknown[];
	readonly selected: SdkDocumentCase;
}
/** Compare real report/request fields with independent source literals and the adopted policy. */
export function requireSdkDocumentObservation(input: DocumentObservationInput): void {
	const report = reportSchema.parse(input.report);
	equal(
		Object.keys(report.piece_evidence).length,
		sdkDocumentCases[input.selected].headings.length + sdkDocumentLinks.length + 1,
		'complete piece inventory',
	);
	requireOutline(report, input.selected);
	requireTargets(report, input.selected);
	const [page] = pieces(report, 'page');
	requireSdk(page, 'Missing page piece');
	equal(page.where.file, 'docs/page.html', 'page owner');
	equal(pieces(report, 'page').length, 1, 'page multiplicity');
	const raw = z
		.object({ state: z.literal('text'), value: z.string(), spans: z.array(spanSchema).length(1) })
		.parse(page.fields['body']);
	equal(raw.value, sdkDocumentPage, 'page raw bytes');
	equal(
		[page.where.span.byte_start, page.where.span.byte_end, page.where.span.source.content_identity],
		[0n, BigInt(sdkDocumentPage.length), digest(sdkDocumentPage)],
		'page span/source identity',
	);
	equal(
		[raw.spans[0]?.byte_start, raw.spans[0]?.byte_end],
		[0n, BigInt(sdkDocumentPage.length)],
		'page raw span',
	);
	equal(raw.spans[0]?.source, page.where.span.source, 'page raw source');
	equal(
		page.model_view,
		{ state: 'bounded', fields: { body: sdkDocumentCases[input.selected].asking }, cut: [] },
		'asking model view/cuts',
	);
	equal(
		input.requests,
		[sdkDocumentRequest(input.selected)],
		'complete asking request/multiplicity',
	);
}

export interface DocumentRootInput {
	readonly report: unknown;
	readonly selected: SdkDocumentCase;
	readonly endpoint: string;
	readonly origin: Readonly<Record<string, unknown>>;
	readonly originalContent?: string;
}
/** File roots retain exact authored bytes; edited/inline roots match independent normalized fields. */
export function requireSdkDocumentRoot(input: DocumentRootInput): void {
	const report = reportSchema.parse(input.report);
	equal(report.configuration, input.origin, 'root origin');
	const roots = report.sources.filter((source) => isDeepStrictEqual(source.origin, input.origin));
	equal(roots.length, 1, 'root source multiplicity');
	const root = roots[0];
	requireSdk(root, 'Missing root source');
	for (const source of report.sources)
		equal(source.sha256, digest(source.content), 'source content/hash');
	if (input.originalContent !== undefined)
		equal(root.content, input.originalContent, 'original root bytes');
	else
		equal(
			JSON.parse(root.content),
			sdkDocumentRootValue(input.endpoint, input.selected),
			'normalized root fields/settings',
		);
}

interface RunDocumentCaseInput {
	readonly context: JourneyContext;
	readonly input: unknown;
	readonly root: string;
	readonly selected: SdkDocumentCase;
	readonly label: string;
	readonly origin: DocumentRootInput['origin'];
	readonly originalContent?: string;
}
async function runDocumentCase(props: RunDocumentCaseInput): Promise<void> {
	const { context, input, root, selected, label, origin, originalContent } = props;
	const cancellation = new AbortController();
	const before = context.backend.captures.length;
	const pending = context.sdk.run(input, {
		root,
		scope: { kind: 'all' },
		signal: cancellation.signal,
	});
	try {
		const outcome = await boundedSdkWait(pending, `document settings ${label}`);
		const requests = context.backend.captures.slice(before).map((capture) => capture.body);
		saveSdkEvidence(path.join(context.evidence, `document-settings-${label}.json`), {
			outcome,
			requests,
		});
		const ending = z
			.object({ kind: z.enum(['completed', 'incomplete']), report: reportSchema })
			.passthrough()
			.parse(outcome);
		requireSdkDocumentObservation({ report: ending.report, requests, selected });
		requireSdkDocumentRoot({
			report: ending.report,
			selected,
			endpoint: context.backend.endpoint,
			origin,
			...(originalContent === undefined ? {} : { originalContent }),
		});
	} finally {
		cancellation.abort();
		await boundedSdkWait(Promise.allSettled([pending]), 'owned document settings settlement');
	}
}

/** Shared listener observations are deliberately serial: a promise chain owns each capture window. */
async function sequentialCases<TValue>(
	values: readonly TValue[],
	operation: (value: TValue, index: number) => Promise<void>,
): Promise<void> {
	await values.reduce<Promise<void>>(async (previous, value, index) => {
		await previous;
		await operation(value, index);
	}, Promise.resolve());
}

/** Existing inline journey: actual direct/project calls, alternating policies in one addon process. */
export async function observeSdkInlineDocumentSettings(context: JourneyContext): Promise<string[]> {
	const root = path.join(context.evidence, 'document-inline-root');
	const base = path.join(context.evidence, 'document-inline-definitions');
	filesystem.mkdirSync(base);
	createSdkDocumentInputs(root);
	const origin = { kind: 'inline', id: 'sdk-document-settings', label: 'sdk-document-settings' };
	await sequentialCases(
		['standard', 'restored', 'custom', 'standard'] as const,
		async (selected, index) => {
			await runDocumentCase({
				context,
				input: sdkDocumentConfiguration(base, context.backend.endpoint, selected),
				root,
				selected,
				label: `inline-${index}`,
				origin,
			});
		},
	);
	equal(filesystem.readdirSync(root), ['docs'], 'inline checked-root contents');
	equal(filesystem.readdirSync(base), [], 'inline definitions remain absent');
	createSdkDocumentInputs(root, base);
	await runDocumentCase({
		context,
		input: sdkDocumentConfiguration(base, context.backend.endpoint, 'custom', true),
		root,
		selected: 'custom',
		label: 'project-custom',
		origin,
	});
	return [
		'installed document defaults/restored/custom, exact outline/request/raw/reasons and project isolation',
	];
}

/** Existing snapshot journey: each setting must independently decide behavior AND provenance. */
export async function observeSdkSnapshotDocumentSettings(
	context: JourneyContext,
	requireFrozen: (value: unknown) => void,
): Promise<string[]> {
	const root = path.join(context.evidence, 'document-snapshot-root');
	const base = path.join(context.evidence, 'document-snapshot-definitions');
	createSdkDocumentInputs(root, base);
	const configPath = path.join(filesystem.realpathSync(base), 'sunstack.lint.json');
	const contentA = JSON.stringify(
		sdkDocumentFileConfiguration(context.backend.endpoint, 'restored'),
	);
	const contentB = JSON.stringify(
		sdkDocumentFileConfiguration(context.backend.endpoint, 'standard'),
	);
	filesystem.writeFileSync(configPath, contentA);
	const replyA = await boundedSdkWait(
		context.sdk.load({ directory: base }),
		'load configured document A',
	);
	requireFrozen(recordSchema.parse(replyA)['value']);
	const loaded = z
		.object({
			kind: z.literal('loaded'),
			value: z.object({ configuration: recordSchema, sources: z.unknown() }).passthrough(),
		})
		.parse(replyA);
	const held = loaded.value;
	const id = z.string().parse(held.configuration['id']);
	const fileOrigin = { kind: 'file', path: configPath };
	const editedOrigin = { kind: 'edited', id, based_on: fileOrigin };
	filesystem.writeFileSync(configPath, contentB);
	await runDocumentCase({
		context,
		input: held,
		root,
		selected: 'restored',
		label: 'held-A',
		origin: fileOrigin,
		originalContent: contentA,
	});
	await sequentialCases(
		['custom', 'skipOnly', 'attributesOnly', 'anchorsOnly'] as const,
		async (selected) => {
			const edited = structuredClone(held.configuration);
			edited['evidence'] = {
				...recordSchema.parse(edited['evidence']),
				...sdkDocumentCases[selected].settings,
			};
			await runDocumentCase({
				context,
				input: edited,
				root,
				selected,
				label: `edit-${selected}`,
				origin: editedOrigin,
			});
		},
	);
	await runDocumentCase({
		context,
		input: { ...held, sources: [] },
		root,
		selected: 'restored',
		label: 'receipt-only',
		origin: fileOrigin,
		originalContent: contentA,
	});
	const replyB = await boundedSdkWait(
		context.sdk.load({ directory: base }),
		'explicit reload document B',
	);
	requireFrozen(recordSchema.parse(replyB)['value']);
	const reloaded = z
		.object({
			kind: z.literal('loaded'),
			value: z.object({ configuration: recordSchema }).passthrough(),
		})
		.parse(replyB);
	await runDocumentCase({
		context,
		input: reloaded.value,
		root,
		selected: 'standard',
		label: 'reload-B',
		origin: fileOrigin,
		originalContent: contentB,
	});
	await sequentialCases(
		[
			['omitted', {}],
			['empty', { html: {}, markdown: {} }],
			[
				'null-scheme',
				{ html: { skip_tags: [], text_attributes: {} }, markdown: { heading_anchors: null } },
			],
		] as const,
		async ([label, value]) => {
			const same = structuredClone(reloaded.value.configuration);
			const evidence = recordSchema.parse(same['evidence']);
			delete evidence['html'];
			delete evidence['markdown'];
			same['evidence'] = { ...evidence, ...value };
			await runDocumentCase({
				context,
				input: same,
				root,
				selected: 'standard',
				label: `equivalent-${label}`,
				origin: fileOrigin,
				originalContent: contentB,
			});
		},
	);
	return [
		'installed held/disk/edited/reload document settings; each isolated setting and default equivalence owns root provenance',
	];
}
