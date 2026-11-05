/** Known wrong captures must fail the same checks used by the real filters journey. */
import { expect, test } from 'vitest';

import {
	excludedFolder,
	filterReply,
	includedFolder,
	requireFilterReport,
	requireFilterRequests,
	type FilterScenario,
} from './rulebook-filters.ts';
import { arrayValue, objectValue, type JsonObject } from './rulebook-fixture-support.ts';
import { type CapturedRequest } from './rulebook-process.ts';

function capture(body: JsonObject): CapturedRequest {
	return { body, raw: Buffer.from(JSON.stringify(body)) };
}
function filterCapture(folder: string, description: string): CapturedRequest {
	return capture({
		questions: {
			filter: {
				type: 'choice',
				instructions: { task: 'Classify this description for the fixture rules.' },
				criteria: { eligible: 'General instructions.', blocked: 'Specific instructions.' },
			},
		},
		state: { where: { folder }, description },
	});
}
function judgedCaptures(): CapturedRequest[] {
	return [
		filterCapture(includedFolder, 'General instructions for an included folder.'),
		filterCapture(excludedFolder, 'Specific instructions for an excluded folder.'),
		...['one.md', 'two.md', 'three.md'].map((name) =>
			capture({
				questions: { verdict: { type: 'noul' } },
				state: {
					where: { file: `${includedFolder}/notes/${name}` },
					body: 'Approved instruction.',
				},
			}),
		),
	];
}

test('request check accepts concurrent arrival order without changing folder/file identities', () => {
	const captures = judgedCaptures().toReversed();
	expect(() => requireFilterRequests(captures, 'judged')).not.toThrow();
});
test.each([
	['a question per gated rule', (rows: CapturedRequest[]) => rows.push(rows[0] ?? capture({}))],
	['a missing folder', (rows: CapturedRequest[]) => rows.splice(1, 1)],
	[
		'capture name used instead of full folder identity',
		(rows: CapturedRequest[]) => {
			const first = rows[0];
			expect(first).toBeDefined();
			if (first) objectValue(objectValue(first.body['state'])['where'])['folder'] = 'review';
		},
	],
	[
		'whole context body sent',
		(rows: CapturedRequest[]) => {
			const first = rows[0];
			if (first) objectValue(first.body['state'])['body'] = 'CONTEXT_ONLY_SENTINEL';
		},
	],
	[
		'authored Choice changed',
		(rows: CapturedRequest[]) => {
			const first = rows[0];
			if (first)
				objectValue(objectValue(objectValue(first.body['questions'])['filter'])['instructions'])[
					'task'
				] = 'wrong';
		},
	],
	['allowed rules never asked', (rows: CapturedRequest[]) => rows.splice(2)],
	[
		'excluded rule asked',
		(rows: CapturedRequest[]) =>
			rows.push(
				capture({
					questions: { verdict: { type: 'noul' } },
					state: { where: { file: `${excludedFolder}/notes/one.md` } },
				}),
			),
	],
])('request check rejects %s', (_name, mutate) => {
	const captures = judgedCaptures();
	mutate(captures);
	expect(() => requireFilterRequests(captures, 'judged')).toThrow();
});
test('folderless check rejects an accidental request instead of trusting exit 3', () => {
	expect(() => requireFilterRequests([], 'folderless')).not.toThrow();
	expect(() => requireFilterRequests(judgedCaptures(), 'folderless')).toThrow();
});
test('scripted Choice deliberately separates confidence, selected probability and maximum', () => {
	const request = filterCapture(excludedFolder, 'Specific instructions for an excluded folder.');
	const reply = filterReply(request.body, 'unsure');
	expect(objectValue(reply['answers'])['filter']).toEqual({
		type: 'choice',
		choice: 'eligible',
		confidence: 0.99,
		probabilities: { eligible: 0.3, blocked: 0.7 },
	});
});

function reportFixture(scenario: FilterScenario = 'judged'): JsonObject {
	const answer = {
		filter: 'category',
		folder: includedFolder,
		requested_model: 'typesafe/requested-filters',
		resolved_model: 'typesafe/controlled-rulebook',
		input_cut: false,
		state: 'judged',
		answer: 'eligible',
		probability: 0.7,
		probabilities: { eligible: 0.7, blocked: 0.3 },
	};
	const excluded: JsonObject = {
		...answer,
		folder: excludedFolder,
		answer: 'blocked',
		probability: 0.9,
		probabilities: { eligible: 0.1, blocked: 0.9 },
	};
	if (scenario === 'unsure') {
		excluded['state'] = 'unsure';
		excluded['probabilities'] = { eligible: 0.3, blocked: 0.7 };
		excluded['answer'] = 'eligible';
		excluded['probability'] = 0.3;
	}
	if (scenario === 'failed') {
		excluded['state'] = 'incomplete';
		excluded['resolved_model'] = null;
		excluded['reason'] = {
			kind: 'request',
			reason: {
				provider_failure: { failure_class: 'provider_rejected_request', http_status: 400 },
			},
		};
	}
	const rows: JsonObject[] = [];
	for (const [name, marker, basis] of [
		['mechanical_gate', 'K001', 'mechanical'],
		['asking_gate', 'K002', 'judged'],
	] as const) {
		if (scenario !== 'folderless')
			for (const file of scenario === 'member' ? ['one.md'] : ['one.md', 'three.md', 'two.md'])
				rows.push({
					name,
					marker,
					file: `${includedFolder}/notes/${file}`,
					first_line: 1,
					outcome: 'pass',
					basis,
				});
		if (scenario === 'judged')
			for (const firstLine of [1, 3, 5])
				rows.push({
					name,
					marker,
					file: `${excludedFolder}/notes/one.md`,
					first_line: firstLine,
					outcome: 'not_applicable',
					basis: 'judged',
					reason: { kind: 'filter', filter: 'category', answer: 'blocked', probability: 0.9 },
				});
	}
	const skipped = ['unsure', 'failed', 'folderless'].includes(scenario);
	return {
		status: {
			kind:
				scenario === 'unsure'
					? 'uncertain'
					: scenario === 'failed' || scenario === 'folderless'
						? 'incomplete'
						: 'clean',
		},
		findings: [],
		uncertain: [],
		counts: { error: 0, warn: 0 },
		exclusions: { not_applicable: scenario === 'judged' ? 6 : 0 },
		results: rows,
		filter_answers:
			scenario === 'folderless' ? [] : scenario === 'member' ? [answer] : [answer, excluded],
		filter_skips: skipped
			? ['mechanical_gate', 'asking_gate'].map((rule) => ({
					rule,
					filter: 'category',
					folder: scenario === 'folderless' ? null : excludedFolder,
					file:
						scenario === 'folderless' ? 'elsewhere/unowned.md' : `${excludedFolder}/notes/one.md`,
					why:
						scenario === 'folderless'
							? 'no_folder'
							: scenario === 'failed'
								? 'incomplete'
								: 'unsure',
					probabilities: scenario === 'unsure' ? { eligible: 0.3, blocked: 0.7 } : null,
				}))
			: [],
		incomplete:
			scenario === 'failed'
				? [{ rule: null, file: excludedFolder, reason: { kind: 'filter', filter: 'category' } }]
				: scenario === 'folderless'
					? ['mechanical_gate', 'asking_gate'].map((rule) => ({
							rule,
							file: 'elsewhere/unowned.md',
							reason: { kind: 'no_filter_folder', filter: 'category' },
						}))
					: [],
	};
}

test.each(['judged', 'unsure', 'failed', 'folderless', 'member'] as const)(
	'report check accepts independently authored %s outcome',
	(scenario) => {
		expect(() => requireFilterReport(reportFixture(scenario), scenario)).not.toThrow();
	},
);
test.each([
	[
		'all gated pairs omitted',
		(report: JsonObject) => {
			report['results'] = [];
		},
	],
	[
		'wrong status',
		(report: JsonObject) => {
			report['status'] = { kind: 'nothing_selected' };
		},
	],
	[
		'missing answer',
		(report: JsonObject) => {
			arrayValue(report['filter_answers']).pop();
		},
	],
	[
		'missing probabilities',
		(report: JsonObject) => {
			delete objectValue(arrayValue(report['filter_answers'])[0])['probabilities'];
		},
	],
	[
		'resolved model copied from requested',
		(report: JsonObject) => {
			objectValue(arrayValue(report['filter_answers'])[0])['resolved_model'] =
				'typesafe/requested-filters';
		},
	],
	[
		'mechanical exclusion basis',
		(report: JsonObject) => {
			objectValue(arrayValue(report['results'])[3])['basis'] = 'mechanical';
		},
	],
	[
		'lost exclusion reason',
		(report: JsonObject) => {
			delete objectValue(arrayValue(report['results'])[3])['reason'];
		},
	],
	[
		'changed result order',
		(report: JsonObject) => {
			arrayValue(report['results']).reverse();
		},
	],
])('report check rejects %s', (_name, mutate) => {
	const report = reportFixture();
	mutate(report);
	expect(() => requireFilterReport(report, 'judged')).toThrow();
});
test('unsure skip deduplication check rejects one skip per piece and a fabricated result', () => {
	const report = reportFixture('unsure');
	const skips = arrayValue(report['filter_skips']);
	skips.push(skips[0] ?? null);
	expect(() => requireFilterReport(report, 'unsure')).toThrow();
	skips.pop();
	arrayValue(report['results']).push({ file: `${excludedFolder}/notes/one.md`, outcome: 'pass' });
	expect(() => requireFilterReport(report, 'unsure')).toThrow();
});
test('skip ordering is not an oracle when identity and contents are unchanged', () => {
	const report = reportFixture('unsure');
	arrayValue(report['filter_skips']).reverse();
	expect(() => requireFilterReport(report, 'unsure')).not.toThrow();
});

test.each([
	[
		'missing selected answer',
		(answer: JsonObject) => {
			delete answer['answer'];
		},
	],
	[
		'missing selected probability',
		(answer: JsonObject) => {
			delete answer['probability'];
		},
	],
	[
		'maximum-probability label substituted',
		(answer: JsonObject) => {
			answer['answer'] = 'blocked';
			answer['probability'] = 0.7;
		},
	],
	[
		'confidence substituted for selected probability',
		(answer: JsonObject) => {
			answer['probability'] = 0.99;
		},
	],
])('unsure report rejects %s', (_name, mutate) => {
	const report = reportFixture('unsure');
	mutate(objectValue(arrayValue(report['filter_answers'])[1]));
	expect(() => requireFilterReport(report, 'unsure')).toThrow();
});

test.each([
	['http_status', 429],
	['failure_class', 'connection'],
] as const)(
	'failed filter check rejects changed %s in the native external-tag envelope',
	(field, value) => {
		const report = reportFixture('failed');
		const reason = objectValue(objectValue(arrayValue(report['filter_answers'])[1])['reason']);
		objectValue(objectValue(reason['reason'])['provider_failure'])[field] = value;
		expect(() => requireFilterReport(report, 'failed')).toThrow();
	},
);
