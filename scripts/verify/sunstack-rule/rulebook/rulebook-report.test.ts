import { expect, test } from 'vitest';

import { type JsonObject } from './rulebook-fixture-support.ts';
import { requireHumanSummary } from './rulebook-output-comparison.ts';
import { requireSavedRequest } from './rulebook-report.ts';

function savedRequestFixture(): Parameters<typeof requireSavedRequest>[0] {
	return {
		sent: {
			model: 'typesafe/requested-report',
			state: { body: 'BODY_SENTINEL', reference: { example: 'Portable practice.' } },
			questions: {
				first: { type: 'noul', instructions: { check: 'body' } },
				unused: { type: 'noul', instructions: { check: 'body' } },
			},
		},
		savedState: { reference: { example: 'Portable practice.' }, body: 'BODY_SENTINEL' },
		savedQuestions: {
			unused: { instructions: { check: 'body' }, type: 'noul' },
			first: { instructions: { check: 'body' }, type: 'noul' },
		},
		savedAnswers: {
			first: { type: 'noul', yes_probability: 0.9 },
			unused: { type: 'noul', yes_probability: 0.1 },
		},
		requestedModel: 'typesafe/requested-report',
		resolvedModel: 'typesafe/responded-report',
		reply: {
			status: 200,
			body: {
				model: 'typesafe/responded-report',
				answers: { first: { type: 'noul', noul: 0.9 }, unused: { type: 'noul', noul: 0.1 } },
			},
		},
	};
}
test('saved requests preserve every answer and accept allowed object-key order', () => {
	expect(() => requireSavedRequest(savedRequestFixture())).not.toThrow();
});
test.each([
	[
		'missing unused answer',
		(entry: Parameters<typeof requireSavedRequest>[0]) => {
			entry.savedAnswers = { first: { type: 'noul', yes_probability: 0.9 } };
		},
	],
	[
		'altered question',
		(entry: Parameters<typeof requireSavedRequest>[0]) => {
			entry.savedQuestions = {};
		},
	],
	[
		'altered state',
		(entry: Parameters<typeof requireSavedRequest>[0]) => {
			entry.savedState = { body: 'changed' };
		},
	],
	[
		'requested model substituted for responding model',
		(entry: Parameters<typeof requireSavedRequest>[0]) => {
			entry.resolvedModel = entry.requestedModel;
		},
	],
])('saved requests reject %s', (_name, mutate) => {
	const entry = savedRequestFixture();
	mutate(entry);
	expect(() => requireSavedRequest(entry)).toThrow();
});
test('human summary accepts distinct IDs and pointers without request bodies', () => {
	expect(() =>
		requireHumanSummary(
			'skills/a.md:17:1: warn [rule] Read and decide. #3\n  target: refs/a.md:1\n1 unsure [rule]; details: lint show <report> 7 (save with --report <report>)\n',
			[
				{ id: 3, file: 'skills/a.md', line: 17, kind: 'warn' },
				{ id: 7, file: 'skills/a.md', line: 40, kind: 'unsure', rule: 'rule' },
			],
		),
	).not.toThrow();
});
test.each([
	['copied request text', 'skills/a.md:17:1: warn Read BODY_SENTINEL. #3'],
	['missing ID', 'skills/a.md:17:1: warn Read and decide.'],
	['unlocated ID', 'warn Read and decide. #3'],
	['wrong line', 'skills/a.md:1:1: warn Read and decide. #3'],
	[
		'whole-report ID duplication',
		'skills/a.md:17:1: warn Read and decide. #3\nskills/b.md:1:1: warn Another entry. #3',
	],
])('human summary rejects %s', (_name, output) => {
	expect(() =>
		requireHumanSummary(output, [{ id: 3, file: 'skills/a.md', line: 17, kind: 'warn' }]),
	).toThrow();
});
// Keep the unknown JSON boundary explicit: no cast supplies a report shape.
const requestedState: JsonObject = { body: 'literal fixture state' };
test('refused requests do not fabricate an answering model', () => {
	const entry = savedRequestFixture();
	entry.sent['state'] = requestedState;
	entry.savedState = requestedState;
	entry.reply = { status: 400, body: { error: 'controlled refusal' } };
	entry.resolvedModel = null;
	entry.savedAnswers = undefined;
	expect(() => requireSavedRequest(entry)).not.toThrow();
	entry.resolvedModel = 'typesafe/invented';
	expect(() => requireSavedRequest(entry)).toThrow();
});
