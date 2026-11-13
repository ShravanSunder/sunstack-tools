/** Pure oracle sensitivity: these records are synthetic, never an installed SDK observation. */
import { createHash } from 'node:crypto';

import { describe, expect, it } from 'vitest';

import { requireSdkDocumentObservation, requireSdkDocumentRoot } from './sdk-document-settings.ts';
import {
	sdkDocumentCases,
	sdkDocumentLinks,
	sdkDocumentLinkText,
	sdkDocumentPage,
	sdkDocumentRootValue,
	type SdkDocumentCase,
} from './sdk-fixtures.ts';

const endpoint = 'http://127.0.0.1:1/systemone';
const origin = { kind: 'inline', id: 'sdk-document-settings', label: 'sdk-document-settings' };
function hash(text: string): string {
	return createHash('sha256').update(text).digest('hex');
}
function span(start: number, end: number): object {
	return {
		byte_start: BigInt(start),
		byte_end: BigInt(end),
		source: { path: 'docs/page.html', content_identity: hash(sdkDocumentPage) },
	};
}
interface SyntheticPiece {
	kind: string;
	where: { file: string; first_line: bigint; span: object };
	fields: Record<string, unknown>;
	model_view: unknown;
}
function positive(selected: SdkDocumentCase): {
	configuration: Record<string, unknown>;
	piece_evidence: Record<string, { piece: SyntheticPiece }>;
	sources: { origin: Record<string, unknown>; content: string; sha256: string }[];
} {
	const content = JSON.stringify(sdkDocumentRootValue(endpoint, selected));
	const piece_evidence: Record<string, { piece: SyntheticPiece }> = {};
	const headings = sdkDocumentCases[selected].headings;
	for (const [index, name] of headings.entries()) {
		const start = sdkDocumentPage.indexOf(
			`<h${name === 'Parent' || name === 'Last' ? 2 : 3}>${name}`,
		);
		const next = headings[index + 1];
		const end =
			name === 'Parent'
				? sdkDocumentPage.indexOf('<h2>Last')
				: next === undefined
					? sdkDocumentPage.length
					: sdkDocumentPage.indexOf(`<h${next === 'Last' ? 2 : 3}>${next}`);
		piece_evidence[`outline-${index}`] = {
			piece: {
				kind: 'outline',
				where: { file: 'docs/page.html', first_line: 1n, span: span(start, end) },
				fields: {
					raw: {
						state: 'text',
						value: sdkDocumentPage.slice(start, end),
						spans: [span(start, end)],
					},
				},
				model_view: {},
			},
		};
	}
	piece_evidence['page'] = {
		piece: {
			kind: 'page',
			where: { file: 'docs/page.html', first_line: 1n, span: span(0, sdkDocumentPage.length) },
			fields: {
				body: { state: 'text', value: sdkDocumentPage, spans: [span(0, sdkDocumentPage.length)] },
			},
			model_view: {
				state: 'bounded',
				fields: { body: sdkDocumentCases[selected].asking },
				cut: [],
			},
		},
	};
	const github = sdkDocumentCases[selected].github;
	const anchors: Record<string, unknown> = {
		heading: github
			? { status: 'found', fragment: 'heading' }
			: { status: 'unsupported', fragment: 'heading', reason: 'heading_scheme_not_configured' },
		missing: github
			? { status: 'missing', fragment: 'missing' }
			: { status: 'unsupported', fragment: 'missing', reason: 'heading_scheme_not_configured' },
		explicit: { status: 'found', fragment: 'explicit' },
		named: { status: 'found', fragment: 'named' },
		html: { status: 'found', fragment: 'html-id' },
		html_missing: { status: 'missing', fragment: 'absent' },
		text: { status: 'unsupported', fragment: 'part', reason: 'unsupported_document' },
	};
	for (const [index, [name, target]] of sdkDocumentLinks.entries()) {
		piece_evidence[`link-${name}`] = {
			piece: {
				kind: 'link',
				where: {
					file: 'docs/links.md',
					first_line: BigInt(index * 2 + 1),
					span: {
						byte_start: 0n,
						byte_end: 1n,
						source: { path: 'docs/links.md', content_identity: hash(sdkDocumentLinkText) },
					},
				},
				fields: {
					targets:
						name === 'outside'
							? { state: 'unavailable', reason: 'outside_root' }
							: {
									state: 'targets',
									values: [
										{
											name: target,
											exists: name !== 'absent',
											...(anchors[name] === undefined ? {} : { anchor: anchors[name] }),
										},
									],
								},
				},
				model_view: {},
			},
		};
	}
	return {
		configuration: { ...origin },
		piece_evidence,
		sources: [{ origin: { ...origin }, content, sha256: hash(content) }],
	};
}
function piece(report: ReturnType<typeof positive>, key: string): SyntheticPiece {
	const value = report.piece_evidence[key];
	if (!value) throw new Error(`Synthetic test setup lacks ${key}`);
	return value.piece;
}
interface SyntheticRequest {
	readonly model: string;
	readonly state: {
		readonly body: string;
		readonly standard: string;
		readonly 'unavailable-fields': Readonly<Record<string, never>>;
	};
	readonly questions: {
		readonly actor: {
			readonly type: 'noul';
			readonly instructions: { readonly check: string; readonly task: string };
		};
	};
}
/** Complete synthetic HTTP body authored independently of the expected request factory. */
function independentRequest(selected: SdkDocumentCase): SyntheticRequest {
	return {
		model: 'jev-latest',
		state: {
			body: sdkDocumentCases[selected].asking,
			standard: 'Document settings are explicit.',
			'unavailable-fields': {},
		},
		questions: {
			actor: { type: 'noul', instructions: { check: 'body', task: 'Is the document complete?' } },
		},
	};
}
function check(
	report: unknown,
	selected: SdkDocumentCase = 'restored',
	requests: readonly unknown[] = [independentRequest(selected)],
): void {
	requireSdkDocumentObservation({ report, selected, requests });
}
function checkRoot(report: unknown, selected: SdkDocumentCase = 'restored'): void {
	requireSdkDocumentRoot({ report, selected, endpoint, origin });
}

describe('document settings oracles reject false greens', () => {
	it('accepts the six independent literal matrices without claiming a native run', () => {
		for (const selected of [
			'standard',
			'restored',
			'custom',
			'skipOnly',
			'attributesOnly',
			'anchorsOnly',
		] as const) {
			const report = positive(selected);
			expect(() => check(report, selected)).not.toThrow();
			expect(() => checkRoot(report, selected)).not.toThrow();
		}
	});
	it.each([
		['missing model', 'missing'],
		['changed model', 'changed'],
		['wrong-type model', 'wrong-type'],
		['extra envelope key', 'extra'],
		['changed state', 'state'],
		['changed question', 'question'],
		['duplicate requests', 'duplicate'],
		['empty requests', 'empty'],
	] as const)('rejects an otherwise-valid complete request with %s', (_label, change) => {
		const request = independentRequest('restored');
		const missingModel: Record<string, unknown> = { ...request };
		delete missingModel['model'];
		const variants: Record<typeof change, readonly unknown[]> = {
			missing: [missingModel],
			changed: [{ ...request, model: 'wrong-model' }],
			'wrong-type': [{ ...request, model: 7 }],
			extra: [{ ...request, extra: true }],
			state: [{ ...request, state: { ...request.state, standard: 'Changed standard.' } }],
			question: [
				{
					...request,
					questions: {
						actor: {
							...request.questions.actor,
							instructions: { ...request.questions.actor.instructions, task: 'Changed task.' },
						},
					},
				},
			],
			duplicate: [request, request],
			empty: [],
		};
		expect(() => check(positive('restored'), 'restored', variants[change])).toThrow(
			'complete asking request/multiplicity',
		);
	});
	it('rejects a different heading with the same count and changed raw ranges', () => {
		const report = positive('restored');
		piece(report, 'outline-1').where.span = span(
			sdkDocumentPage.indexOf('<h3>Island'),
			sdkDocumentPage.indexOf('<h2>Last'),
		);
		expect(() => check(report)).toThrow('heading identity');
		const raw = positive('restored');
		piece(raw, 'outline-0').fields['raw'] = {
			state: 'text',
			value: 'rewritten',
			spans: [span(0, sdkDocumentPage.indexOf('<h2>Last'))],
		};
		expect(() => check(raw)).toThrow('raw');
	});
	it('requires the exact supported and unsupported reason payload for each owner', () => {
		for (const anchor of [
			{ status: 'unsupported', fragment: 'part' },
			{ status: 'unsupported', fragment: 'part', reason: 'unknown' },
			{ status: 'unsupported', fragment: 'part', reason: 'heading_scheme_not_configured' },
		]) {
			const report = positive('restored');
			piece(report, 'link-text').fields['targets'] = {
				state: 'targets',
				values: [{ name: 'notes.txt#part', exists: true, anchor }],
			};
			expect(() => check(report)).toThrow('closed target anchor');
		}
		const legacyDefault = positive('standard');
		piece(legacyDefault, 'link-heading').fields['targets'] = {
			state: 'targets',
			values: [
				{
					name: 'target.md#heading',
					exists: true,
					anchor: { status: 'found', fragment: 'heading' },
				},
			],
		};
		expect(() => check(legacyDefault, 'standard')).toThrow('closed target anchor');
		const explicit = positive('standard');
		piece(explicit, 'link-explicit').fields['targets'] = {
			state: 'targets',
			values: [
				{
					name: 'target.md#explicit',
					exists: true,
					anchor: {
						status: 'unsupported',
						fragment: 'explicit',
						reason: 'heading_scheme_not_configured',
					},
				},
			],
		};
		expect(() => check(explicit, 'standard')).toThrow('closed target anchor');
	});
	it('rejects wrong requests/cuts/raw/source identities and inert or duplicate observations', () => {
		const wrongCut = positive('restored');
		piece(wrongCut, 'page').model_view = {
			state: 'bounded',
			fields: { body: sdkDocumentCases.restored.asking },
			cut: ['body'],
		};
		expect(() => check(wrongCut)).toThrow('model view/cuts');
		for (const requests of [
			[],
			[independentRequest('standard')],
			[independentRequest('restored'), independentRequest('restored')],
		]) {
			expect(() => check(positive('restored'), 'restored', requests)).toThrow('request');
		}
		const raw = positive('restored');
		piece(raw, 'page').fields['body'] = {
			state: 'text',
			value: 'changed raw',
			spans: [span(0, sdkDocumentPage.length)],
		};
		expect(() => check(raw)).toThrow('raw');
		const wrongSource = positive('restored');
		piece(wrongSource, 'page').where.span = {
			byte_start: 0n,
			byte_end: BigInt(sdkDocumentPage.length),
			source: { path: 'docs/page.html', content_identity: '0'.repeat(64) },
		};
		expect(() => check(wrongSource)).toThrow('source identity');
		const missing = positive('restored');
		delete missing.piece_evidence['link-explicit'];
		expect(() => check(missing)).toThrow('inventory');
		const duplicate = positive('restored');
		duplicate.piece_evidence['extra'] = { piece: piece(duplicate, 'page') };
		expect(() => check(duplicate)).toThrow('inventory');
		expect(() => check({})).toThrow();
	});
	it('rejects each isolated dropped root setting even when its digest is internally correct', () => {
		for (const selected of ['skipOnly', 'attributesOnly', 'anchorsOnly'] as const) {
			const report = positive(selected);
			const root = report.sources[0];
			if (!root) throw new Error('Synthetic root absent');
			root.content = JSON.stringify(sdkDocumentRootValue(endpoint, 'restored'));
			root.sha256 = hash(root.content);
			expect(() => checkRoot(report, selected)).toThrow('normalized root');
		}
	});
	it('rejects incorrect root origin/content/hash and missing or duplicate root records', () => {
		const wrongOrigin = positive('restored');
		wrongOrigin.configuration = { kind: 'file', path: '/wrong' };
		expect(() => checkRoot(wrongOrigin)).toThrow('root origin');
		const wrongDigest = positive('restored');
		const root = wrongDigest.sources[0];
		if (!root) throw new Error('Synthetic root absent');
		root.sha256 = '0'.repeat(64);
		expect(() => checkRoot(wrongDigest)).toThrow('hash');
		const duplicate = positive('restored');
		const first = duplicate.sources[0];
		if (!first) throw new Error('Synthetic root absent');
		duplicate.sources.push(first);
		expect(() => checkRoot(duplicate)).toThrow('multiplicity');
		expect(() => checkRoot({ ...positive('restored'), sources: [] })).toThrow('multiplicity');
	});
});
