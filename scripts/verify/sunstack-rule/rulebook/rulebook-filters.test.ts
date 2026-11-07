import { expect, test } from 'vitest';

import {
	descriptions,
	openings,
	excludedFolder,
	includedFolder,
	includedMembers,
	filterQuestions,
	filterReference,
	filterReply,
	requireFilterRequests,
	requireFilterReport,
	requireFilterWalk,
} from './rulebook-filters.ts';
import { arrayValue, textValue, objectValue, type JsonObject } from './rulebook-fixture-support.ts';
import { type CapturedRequest } from './rulebook-process.ts';
function capture(body: JsonObject): CapturedRequest {
	return { body, raw: Buffer.from(JSON.stringify(body)) };
}
function filterCapture(folder: string): CapturedRequest {
	return capture({
		questions: filterQuestions,
		state: {
			description: descriptions.get(folder) ?? '',
			opening: openings.get(folder) ?? '',
			reference: filterReference,
			'unavailable-fields': {},
		},
	});
}
function judgedCaptures(): CapturedRequest[] {
	return [
		filterCapture(includedFolder),
		filterCapture(excludedFolder),
		...includedMembers.map((file) =>
			capture({
				questions: { verdict: { type: 'noul' } },
				state: { where: { file }, body: 'Approved instruction.' },
			}),
		),
	];
}
test('accepts both questions in one request per folder in any arrival order', () => {
	// Arrange / Act / Assert: HTTP completion order is not report order.
	expect(() => requireFilterRequests(judgedCaptures().toReversed(), 'judged')).not.toThrow();
});
test.each([
	[
		'duplicate folder request',
		(rows: CapturedRequest[]) => rows.push(filterCapture(includedFolder)),
	],
	['missing folder', (rows: CapturedRequest[]) => rows.splice(1, 1)],
	[
		'unasked folder path',
		(rows: CapturedRequest[]) => {
			const first = rows[0];
			if (first) objectValue(first.body['state'])['where'] = { folder: includedFolder };
		},
	],
	[
		'undeclared context',
		(rows: CapturedRequest[]) => {
			const first = rows[0];
			if (first) objectValue(first.body['state'])['body'] = 'CONTEXT_ONLY_SENTINEL';
		},
	],
	[
		'reference changed',
		(rows: CapturedRequest[]) => {
			const first = rows[0];
			if (first) objectValue(first.body['state'])['reference'] = { wrong: true };
		},
	],
	[
		'split questions',
		(rows: CapturedRequest[]) => {
			const first = rows[0];
			if (first)
				first.body['questions'] = { scope: objectValue(first.body['questions'])['scope'] ?? null };
		},
	],
	['missing eligible requests', (rows: CapturedRequest[]) => rows.splice(2)],
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
])('rejects %s', (_name, mutate) => {
	const captures = judgedCaptures();
	mutate(captures);
	expect(() => requireFilterRequests(captures, 'judged')).toThrow();
});
test('folderless refusal cannot pass with accidental requests', () => {
	expect(() => requireFilterRequests([], 'folderless')).not.toThrow();
	expect(() => requireFilterRequests(judgedCaptures(), 'folderless')).toThrow();
});
test.each([
	['root-unsure', 'scope'],
	['unsure', 'style'],
] as const)('scripts %s at the intended tree depth', (scenario, name) => {
	const reply = filterReply(filterCapture(excludedFolder).body, scenario);
	const answer = objectValue(objectValue(reply['answers'])[name]);
	expect(answer['confidence']).toBe(0.99);
	expect(Math.max(...Object.values(objectValue(answer['probabilities'])).map(Number))).toBe(0.5);
});

function walkFixture(
	scenario: 'judged' | 'root-unsure' | 'unsure' | 'failed',
): Parameters<typeof requireFilterWalk>[0] {
	const request = filterCapture(excludedFolder);
	return {
		scenario,
		folder: excludedFolder,
		state: scenario === 'judged' ? 'judged' : scenario === 'failed' ? 'incomplete' : 'unsure',
		label: scenario === 'judged' ? 'blocked' : undefined,
		path: scenario === 'failed' ? [] : scenario === 'unsure' ? ['scope', 'style'] : ['scope'],
		step: scenario === 'root-unsure' ? 'scope' : scenario === 'unsure' ? 'style' : undefined,
		answers:
			scenario === 'failed'
				? {}
				: Object.fromEntries(
						Object.entries(objectValue(filterReply(request.body, scenario)['answers'])).map(
							([name, value]) => {
								const raw = objectValue(value);
								return [
									name,
									{
										type: 'choice',
										selected_label: raw['choice'] ?? null,
										confidence: raw['confidence'] ?? null,
										probabilities: raw['probabilities'] ?? null,
									},
								];
							},
						),
					),
	};
}
test.each(['judged', 'root-unsure', 'unsure', 'failed'] as const)(
	'accepts the authored %s tree route',
	(scenario) => {
		expect(() => requireFilterWalk(walkFixture(scenario))).not.toThrow();
	},
);
test.each([
	[
		'wrong terminal label',
		(walk: Parameters<typeof requireFilterWalk>[0]) => {
			walk.label = 'reusable';
		},
	],
	[
		'invented route',
		(walk: Parameters<typeof requireFilterWalk>[0]) => {
			walk.path = ['scope', 'style'];
		},
	],
	[
		'changed answer',
		(walk: Parameters<typeof requireFilterWalk>[0]) => {
			walk.answers['scope'] = {
				type: 'choice',
				choice: 'other',
				confidence: 1,
				probabilities: { reusable: 0, product: 0, other: 1 },
			};
		},
	],
])('rejects %s', (_name, mutate) => {
	const walk = walkFixture('judged');
	mutate(walk);
	expect(() => requireFilterWalk(walk)).toThrow();
});
test.each(['root-unsure', 'unsure'] as const)(
	'rejects fabricated labels and wrong steps for %s',
	(scenario) => {
		const walk = walkFixture(scenario);
		walk.label = 'eligible';
		expect(() => requireFilterWalk(walk)).toThrow();
		walk.label = undefined;
		walk.step = 'made_up';
		expect(() => requireFilterWalk(walk)).toThrow();
	},
);

function reportFixture(scenario: 'judged' | 'root-unsure' | 'unsure' | 'failed'): JsonObject {
	const judgedAnswer: JsonObject = {
		type: 'choice',
		selected_label: 'reusable',
		confidence: 0.99,
		probabilities: { reusable: 0.9, product: 0.05, other: 0.05 },
	};
	const styleAnswer: JsonObject = {
		type: 'choice',
		selected_label: 'approved',
		confidence: 0.99,
		probabilities: { approved: 0.9, blocked: 0.05, other: 0.05 },
	};
	const included: JsonObject = {
		filter: 'category',
		folder: includedFolder,
		input_cut: false,
		requested_model: 'typesafe/requested-filters',
		resolved_model: 'typesafe/controlled-rulebook',
		state: 'judged',
		label: 'eligible',
		path: ['scope', 'style'],
		answers: { scope: judgedAnswer, style: styleAnswer },
	};
	const excludedWalk = walkFixture(scenario);
	const excluded: JsonObject = {
		filter: 'category',
		folder: excludedFolder,
		input_cut: false,
		requested_model: 'typesafe/requested-filters',
		resolved_model: scenario === 'failed' ? null : 'typesafe/controlled-rulebook',
		state: textValue(excludedWalk.state),
		path: scenario === 'failed' ? [] : scenario === 'unsure' ? ['scope', 'style'] : ['scope'],
		answers: excludedWalk.answers,
	};
	if (scenario === 'judged') excluded['label'] = 'blocked';
	if (scenario === 'root-unsure' || scenario === 'unsure')
		excluded['step'] = scenario === 'root-unsure' ? 'scope' : 'style';
	if (scenario === 'failed')
		excluded['reason'] = {
			kind: 'request',
			reason: {
				provider_failure: { failure_class: 'provider_rejected_request', http_status: 400 },
			},
		};
	const results: JsonObject[] = [];
	for (const name of ['mechanical_gate', 'asking_gate']) {
		for (const file of includedMembers) results.push({ name, file, outcome: 'pass' });
		if (scenario === 'judged')
			for (const firstLine of [1, 3, 5])
				results.push({
					name,
					file: `${excludedFolder}/notes/one.md`,
					first_line: firstLine,
					outcome: 'not_applicable',
					reason: { kind: 'filter', filter: 'category', label: 'blocked' },
				});
	}
	return {
		status: {
			kind: scenario === 'judged' ? 'clean' : scenario === 'failed' ? 'incomplete' : 'uncertain',
		},
		findings: [],
		uncertain: [],
		counts: { error: 0, warn: 0 },
		exclusions: { not_applicable: scenario === 'judged' ? 6 : 0 },
		results,
		filter_answers: [included, excluded],
		filter_skips:
			scenario === 'judged'
				? []
				: ['mechanical_gate', 'asking_gate'].map((rule) => ({
						rule,
						file: `${excludedFolder}/notes/one.md`,
						filter: 'category',
						folder: excludedFolder,
						why: scenario === 'failed' ? 'incomplete' : 'unsure',
						path: excluded['path'] ?? [],
						answers: excludedWalk.answers,
						locations: [1, 3, 5].map((firstLine) => ({
							file: `${excludedFolder}/notes/one.md`,
							first_line: firstLine,
							last_line: firstLine,
						})),
					})),
		incomplete: scenario === 'failed' ? [{ reason: 'controlled failure' }] : [],
	};
}
test.each(['judged', 'root-unsure', 'unsure', 'failed'] as const)(
	'accepts the public %s report',
	(scenario) => {
		expect(() => requireFilterReport(reportFixture(scenario), scenario)).not.toThrow();
	},
);
test.each([
	[
		'lost skipped piece locations',
		(report: JsonObject) => {
			objectValue(arrayValue(report['filter_skips'])[0])['locations'] = [];
		},
	],
	[
		'duplicate skip',
		(report: JsonObject) => {
			const skips = arrayValue(report['filter_skips']);
			skips.push(skips[0] ?? null);
		},
	],
	[
		'lost skip route',
		(report: JsonObject) => {
			objectValue(arrayValue(report['filter_skips'])[0])['path'] = [];
		},
	],
	[
		'invented unsure label',
		(report: JsonObject) => {
			objectValue(arrayValue(report['filter_answers'])[1])['label'] = 'eligible';
		},
	],
	[
		'changed answering model',
		(report: JsonObject) => {
			objectValue(arrayValue(report['filter_answers'])[0])['resolved_model'] =
				'typesafe/requested-filters';
		},
	],
	[
		'fabricated combined probability',
		(report: JsonObject) => {
			objectValue(arrayValue(report['filter_answers'])[0])['probability'] = 0.9;
		},
	],
	[
		'missing included pairs',
		(report: JsonObject) => {
			report['results'] = [];
		},
	],
	[
		'skipped rule walked',
		(report: JsonObject) => {
			arrayValue(report['results']).push({
				name: 'asking_gate',
				file: `${excludedFolder}/notes/one.md`,
				outcome: 'pass',
			});
		},
	],
])('public report rejects %s', (_name, mutate) => {
	const report = reportFixture('unsure');
	mutate(report);
	expect(() => requireFilterReport(report, 'unsure')).toThrow();
});
