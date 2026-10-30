/** Challenge the verification comparators with literal public-contract examples. */
import { expect, test } from 'vitest';

import { objectValue, type JsonObject } from './rulebook-fixture-support.ts';
import {
	qualifiedSelectionQuestion,
	requireCandidateKeys,
	requireSameShared,
	requireSelectionCounts,
	requireSelectionRequest,
} from './selection-checks.ts';
const question: JsonObject = {
	type: 'noul',
	instructions: {
		check: 'body',
		against: 'heading',
		context: ['outline'],
		task: 'Choose the candidate.',
	},
};
function request(): JsonObject {
	return {
		questions: { ['c1']: qualifiedSelectionQuestion({ question, key: 'c1', shared: ['outline'] }) },
		state: {
			where: { file: 'one.md' },
			candidates: { ['c1']: { heading: '## Workflow', body: '1. Read.' } },
			shared: { outline: ['## Workflow', '## Rules'] },
		},
	};
}
function report(): JsonObject {
	return {
		selections: [
			{
				kind: 'section',
				where: {
					file: 'one.md',
					first_line: 2,
					last_line: 4,
					span: { byte_start: 4, byte_end: 40 },
				},
				first_line: '## Workflow',
				outcome: { kind: 'selected', yes: 0.93 },
			},
		],
		file_counts: [
			{
				file: 'empty.md',
				kinds: {
					section: {
						matches: 0,
						pieces: 0,
						picked: 0,
						limited: false,
						selection: {
							candidates: 0,
							asked: 0,
							selected: 0,
							not_selected: 0,
							uncertain: 0,
							incomplete: 0,
						},
					},
				},
			},
			{
				file: 'one.md',
				kinds: {
					section: {
						matches: 2,
						pieces: 1,
						picked: 1,
						limited: false,
						selection: {
							candidates: 2,
							asked: 1,
							selected: 1,
							not_selected: 0,
							uncertain: 0,
							incomplete: 0,
						},
					},
				},
			},
		],
	};
}
function checkCounts(value: JsonObject): void {
	requireSelectionCounts({
		report: value,
		files: ['empty.md', 'one.md'],
		kinds: ['section'],
		selectedKinds: ['section'],
	});
}
function checkRequest(body: JsonObject): void {
	requireSelectionRequest({
		body,
		file: 'one.md',
		question,
		sharedNames: ['outline'],
		candidates: { ['c1']: { heading: '## Workflow', body: '1. Read.' } },
		shared: { outline: ['## Workflow', '## Rules'] },
	});
}
test('unasked candidates and zero-candidate files remain distinguishable', () => {
	checkCounts(report());
});
test.each([
	'missing_empty',
	'missing_asked',
	'false_selected',
	'picked_exceeds_pieces',
	'duplicate_identity',
])('accounting rejects %s', (fault) => {
	const value = report();
	const files = value['file_counts'];
	const entries = value['selections'];
	if (!Array.isArray(files) || !Array.isArray(entries)) throw new Error('Literal fixture corrupt');
	const counted = objectValue(objectValue(objectValue(files[1])['kinds'])['section']);
	if (fault === 'missing_empty') files.shift();
	if (fault === 'missing_asked') entries.length = 0;
	if (fault === 'false_selected') objectValue(counted['selection'])['selected'] = 0;
	if (fault === 'picked_exceeds_pieces') counted['picked'] = 2;
	if (fault === 'duplicate_identity') entries.push(structuredClone(entries[0] ?? null));
	expect(() => checkCounts(value)).toThrow();
});
test('request preserves question wording and qualifies only named field references', () => {
	checkRequest(request());
	expect(qualifiedSelectionQuestion({ question, key: 'c7', shared: ['outline'] })).toEqual({
		type: 'noul',
		instructions: {
			check: 'candidates.c7.body',
			against: 'candidates.c7.heading',
			context: ['shared.outline'],
			task: 'Choose the candidate.',
		},
	});
});
test.each(['raw_text', 'unqualified', 'target_opening', 'wrong_shared', 'extra_candidate'])(
	'request comparator rejects %s',
	(fault) => {
		const body = request();
		const state = objectValue(body['state']);
		if (fault === 'raw_text') state['text'] = 'Whole raw file';
		if (fault === 'unqualified') objectValue(body['questions'])['c1'] = question;
		if (fault === 'target_opening')
			objectValue(objectValue(state['candidates'])['c1'])['target'] = { opening: 'private target' };
		if (fault === 'wrong_shared') state['shared'] = { outline: ['different cut'] };
		if (fault === 'extra_candidate') objectValue(state['candidates'])['c2'] = { body: 'unasked' };
		expect(() => checkRequest(body)).toThrow();
	},
);
test('batch comparator rejects duplicate, reordered or unmatched candidate keys', () => {
	const body = request();
	requireCandidateKeys({ bodies: [body], groups: [['c1']] });
	expect(() => requireCandidateKeys({ bodies: [body, body], groups: [['c1'], ['c1']] })).toThrow(
		'more than once',
	);
	const changed = request();
	objectValue(changed['state'])['candidates'] = { ['c2']: { body: 'other' } };
	expect(() => requireCandidateKeys({ bodies: [changed], groups: [['c1']] })).toThrow();
});
test('shared view equality catches per-candidate recutting', () => {
	const first = request();
	const second = request();
	requireSameShared([first, second]);
	objectValue(second['state'])['shared'] = { outline: ['changed'] };
	expect(() => requireSameShared([first, second])).toThrow('shared view');
});

test('batch membership accepts a lexical JSON key order without changing candidate groups', () => {
	const keys = Array.from({ length: 10 }, (_value, index) => `c${index + 1}`);
	const body: JsonObject = {
		questions: Object.fromEntries(keys.toSorted().map((key) => [key, question])),
		state: {
			where: { file: 'one.md' },
			shared: {},
			candidates: Object.fromEntries(keys.map((key) => [key, { body: key }])),
		},
	};
	requireCandidateKeys({ bodies: [body], groups: [keys] });
});
