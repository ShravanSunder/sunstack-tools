/** Sensitivity checks for the exact assertions used by the live locator journey. */
import { expect, test } from 'vitest';

import {
	expectedCriteria,
	requireLocatorReport,
	requireLocatorRequests,
	type LocatorScenario,
} from './locator-limits.ts';
import { arrayValue, objectValue, type JsonObject } from './rulebook-fixture-support.ts';
import { type CapturedRequest } from './rulebook-process.ts';

const subjectFile = 'documents/subject.md';
function capture(count: number): CapturedRequest {
	const locator = {
		type: 'choice',
		instructions: { check: 'body', task: 'Choose the relevant candidate line.' },
		criteria: expectedCriteria(count),
	};
	const body: JsonObject = {
		questions: {
			verdict: {
				type: 'noul',
				instructions: { check: 'body', task: 'Does this text meet the standard?' },
			},
			focus_line: locator,
			alternative_line: locator,
		},
		state: { where: { file: subjectFile } },
	};
	return { body, raw: Buffer.from(JSON.stringify(body)) };
}
function report(scenario: LocatorScenario): JsonObject {
	const omitted = scenario === 'over-limit';
	const evidence = {
		locator: 'focus_line',
		field: 'body',
		reason: 'line_limit',
		candidate_lines: 256,
		limit: 255,
	};
	const row = (file: string, affected: boolean): JsonObject => ({
		name: 'located_subject',
		file,
		outcome: scenario === 'pass' ? 'pass' : 'finding',
		basis: scenario === 'mechanical' ? 'mechanical' : 'judged',
		path: ['gate'],
		...(affected ? { locator_omitted: evidence } : {}),
		...(scenario === 'mechanical'
			? {}
			: {
					answers:
						affected || ['no-locator', 'pass'].includes(scenario)
							? { verdict: {} }
							: { verdict: {}, focus_line: {} },
				}),
	});
	const finding = (file: string, affected: boolean): JsonObject => ({
		file,
		line: affected || ['low', 'no-locator', 'mechanical'].includes(scenario) ? 3 : 5,
		anchor:
			affected || ['low', 'no-locator', 'mechanical'].includes(scenario) ? 'default' : 'resolved',
		...(affected ? { locator_omitted: evidence } : {}),
	});
	return {
		status: { kind: scenario === 'pass' ? 'clean' : 'findings' },
		incomplete: [],
		results: omitted
			? [row('documents/peer.md', false), row(subjectFile, true)]
			: [row(subjectFile, false)],
		findings:
			scenario === 'pass'
				? []
				: omitted
					? [finding('documents/peer.md', false), finding(subjectFile, true)]
					: [finding(subjectFile, false)],
	};
}

test('wire comparator accepts255 options and rejects a truncated line map', () => {
	const request = capture(255);
	expect(() => requireLocatorRequests([request], 'lines255')).not.toThrow();
	delete objectValue(objectValue(objectValue(request.body['questions'])['focus_line'])['criteria'])[
		'L257'
	];
	expect(() => requireLocatorRequests([request], 'lines255')).toThrow();
});
test('wire comparator rejects changed authored questions and extra dispatch', () => {
	const request = capture(254);
	objectValue(request.body['questions'])['verdict'] = { type: 'noul', instructions: {} };
	expect(() => requireLocatorRequests([request], 'lines254')).toThrow();
	expect(() => requireLocatorRequests([capture(254), capture(254)], 'lines254')).toThrow();
});
test.each([
	'lines254',
	'lines255',
	'over-limit',
	'low',
	'no-locator',
	'pass',
	'mechanical',
] as const)('accepts independent %s result', (scenario) => {
	expect(() => requireLocatorReport(report(scenario), scenario)).not.toThrow();
});
test.each([
	['locator', 'alternative_line'],
	['field', 'wrong'],
	['reason', 'budget'],
	['candidate_lines', 255],
	['limit', 254],
] as const)('omission check rejects wrong %s', (field, value) => {
	const result = report('over-limit');
	const row = objectValue(arrayValue(result['results'])[1]);
	objectValue(row['locator_omitted'])[field] = value;
	expect(() => requireLocatorReport(result, 'over-limit')).toThrow();
});
test('omission check rejects missing evidence and leakage into another piece', () => {
	const missing = report('over-limit');
	delete objectValue(arrayValue(missing['results'])[1])['locator_omitted'];
	expect(() => requireLocatorReport(missing, 'over-limit')).toThrow();
	const leaked = report('over-limit');
	objectValue(arrayValue(leaked['results'])[0])['locator_omitted'] = { reason: 'line_limit' };
	expect(() => requireLocatorReport(leaked, 'over-limit')).toThrow();
});
test.each(['low', 'no-locator', 'pass', 'mechanical'] as const)(
	'rejects false omission on %s',
	(scenario) => {
		const result = report(scenario);
		objectValue(arrayValue(result['results'])[0])['locator_omitted'] = { locator: 'focus_line' };
		expect(() => requireLocatorReport(result, scenario)).toThrow();
	},
);
test('rejects default fallback presented as resolved and an invented read answer', () => {
	const result = report('over-limit');
	objectValue(arrayValue(result['findings'])[1])['anchor'] = 'resolved';
	expect(() => requireLocatorReport(result, 'over-limit')).toThrow();
	const invented = report('over-limit');
	objectValue(objectValue(arrayValue(invented['results'])[1])['answers'])['focus_line'] = {};
	expect(() => requireLocatorReport(invented, 'over-limit')).toThrow();
});
