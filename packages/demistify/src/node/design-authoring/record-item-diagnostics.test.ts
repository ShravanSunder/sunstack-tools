import { expect, test } from 'vitest';

import { decisionActorField } from '../../core/decision-record.ts';
import { readDesignItems } from './read-design.ts';

const project = '/private/tmp/design-check';
const decisions = project + '/examples/design-decisions.ts';
const assumptions = project + '/examples/design-assumptions.ts';

const answered = {
	id: 'D5',
	kind: 'choice',
	question: 'When can a request pick up leftover work?',
	options: [
		{ id: 'wait', title: 'Wait for the next mark' },
		{ id: 'resume', title: 'Resume overdue work' },
	],
	recommended: 'none',
	authority: {
		state: 'owner-decided',
		sourceKind: 'owner-words',
		decidedOn: '2026-11-23',
		chosen: 'resume',
		decision: 'Resume overdue work.',
		source: { href: 'index.html#answer', excerpt: 'Resume overdue work.' },
	},
};

const ruling = {
	id: 'D7',
	kind: 'ruling',
	statement: 'Keep a time floor for approvals.',
	authority: {
		state: 'agent-decided',
		[decisionActorField]: { alias: 'Central Lead', role: 'lead' },
		sourceKind: 'author',
		rationale: 'The clock can move backward.',
		decidedOn: '2026-11-23',
		weight: 'design',
	},
};

const assumed = {
	id: 'A1',
	kind: 'assumption',
	statement: 'The fixture can reproduce a restart.',
	[decisionActorField]: 'Central Lead',
	weight: 'design',
};
const open = {
	...assumed,
	status: 'open',
	settles: 'Run the fixture with an eviction.',
};
const resolved = {
	...assumed,
	status: 'confirmed',
	resolution: { href: 'index.html#result', excerpt: 'The fixture reproduced a restart.' },
	settledBy: 'Verifier',
};

test.each([
	[
		{ ...answered, options: [answered.options[0]] },
		'D5: a choice needs at least two options and has one. Add an option, or write this as a ruling.',
	],
	[{ ...answered, question: undefined }, 'D5: a choice needs a question.'],
	[{ ...answered, recommended: undefined }, 'D5: say whether an option is recommended.'],
	[
		{ ...answered, authority: { ...answered.authority, chosen: undefined } },
		'D5: a settled choice needs a chosen option.',
	],
	[
		{ ...answered, authority: { ...answered.authority, decision: undefined } },
		'D5: a settled choice needs decision text.',
	],
	[
		{ ...answered, authority: { ...answered.authority, source: undefined } },
		'D5: this decision needs a source.',
	],
	[
		{ ...ruling, authority: { ...ruling.authority, weight: undefined } },
		"D7: an agent's decision needs a weight. Add weight: 'design', 'process' or 'minor'.",
	],
	[
		{ ...ruling, authority: { ...ruling.authority, [decisionActorField]: undefined } },
		'D7: an agent’s decision needs an author.',
	],
	[
		{ ...ruling, authority: { ...ruling.authority, rationale: undefined } },
		'D7: an agent’s decision needs a rationale.',
	],
	[
		{ ...open, settles: undefined },
		'A1: an open assumption needs settles: one sentence saying what observation would settle it.',
	],
	[{ ...open, [decisionActorField]: undefined }, 'A1: an assumption needs an author.'],
	[
		{ ...resolved, resolution: undefined },
		'A1: a confirmed or refuted assumption needs resolution evidence.',
	],
	[{ ...resolved, settledBy: undefined }, 'A1: a confirmed or refuted assumption needs settledBy.'],
])('reports the record and fix for an invalid authored item %#', (input, message): void => {
	const file = input.kind === 'assumption' ? assumptions : decisions;
	const kind = input.kind === 'assumption' ? 'assumption' : 'choice';
	const source = 'export default ' + JSON.stringify([input]);
	expect(() => readDesignItems(file, source, kind, project)).toThrow(
		`examples/design-${kind === 'assumption' ? 'assumptions' : 'decisions'}.ts: ${message}`,
	);
});
