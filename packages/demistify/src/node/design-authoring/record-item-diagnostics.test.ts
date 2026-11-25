import { expect, test } from 'vitest';

import { choiceRecord, decisionActorField, rulingRecord } from '../../core/decision-record.ts';
import { readPageMarkup } from './design-page-markup.ts';
import { readDesignItems } from './read-design.ts';
import { prepareRecordPage, projectRecords } from './record-projection.ts';
import type { DesignRecord } from './schema.ts';

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
	[decisionActorField]: { alias: 'Central Lead', role: 'lead' },
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

const recordPagePath = '/project/index.html';
const sections = `<ex-section id="settled" holds="settled"></ex-section>
<ex-section id="agent" holds="decided-by-agent"></ex-section>`;
const nativeChoice = `<ex-choice><ex-option data-option="wait"><template slot="summary">Wait.</template></ex-option>
<ex-option data-option="resume"><template slot="summary">Resume.</template></ex-option></ex-choice>`;

function checkRecordPage(body: string): string {
	const html = sections + body;
	const markup = readPageMarkup(html, recordPagePath);
	const records: DesignRecord[] = [choiceRecord.parse(answered), rulingRecord.parse(ruling)];
	const projection = projectRecords(records, [{ file: recordPagePath, markup }]);
	return prepareRecordPage({ html, file: recordPagePath, markup, projection }).html;
}

test('a record placed without every section it can reach names the missing section and why', () => {
	const html = `<ex-section id="settled" holds="settled"></ex-section>
<ex-section-item record="D7"><p>The table keeps a time floor.</p></ex-section-item>`;
	const markup = readPageMarkup(html, recordPagePath);
	const records: DesignRecord[] = [rulingRecord.parse(ruling)];

	expect(() => projectRecords(records, [{ file: recordPagePath, markup }])).toThrow(
		'needs a section with holds="decided-by-agent". A record moves between sections as it is decided, so its page declares every section it can reach: decided-by-agent, settled.',
	);
});

test('one authored record item preserves Choice fragments and accepts a bare ruling body', () => {
	const html = checkRecordPage(`<ex-section-item record="D5">${nativeChoice}</ex-section-item>
<ex-section-item record="D7"><p>The table keeps a time floor.</p></ex-section-item>`);
	expect(html).toContain('<ex-choice id="D5" data-decision="D5"');
	expect(html).toContain('id="D5--option--resume--details"');
	expect(html).toContain(
		'<ex-section-item record="D7" id="D7"><p>The table keeps a time floor.</p>',
	);
});

test('Choice explanation and Details expose their links and stable fragments to typed checking', () => {
	const html =
		sections +
		'<ex-section-item record="D5">' +
		nativeChoice
			.replace(
				'<ex-choice>',
				'<ex-choice><template data-record-body><p id="context"><a href="missing.html">Context</a></p></template>',
			)
			.replace(
				'</ex-option>',
				'<template slot="details"><p id="more"><a href="#context">Back</a></p></template></ex-option>',
			) +
		'</ex-section-item>';
	const markup = readPageMarkup(html, recordPagePath);
	expect(markup.links).toEqual(['missing.html', '#context']);
	expect(markup.ids.has('context')).toBe(true);
	expect(markup.ids.has('more')).toBe(true);
	expect(checkRecordPage(html.slice(sections.length))).toContain('<template data-record-body>');
});

test.each([
	[
		`<ex-section-item record="D5">${nativeChoice.replace('</ex-option>', '<div slot="details"><p>Old carrier.</p></div></ex-option>')}</ex-section-item>`,
		'<template slot="details">',
	],
	[
		'<ex-card data-design-record="D7"><p>Old wrapper.</p></ex-card>',
		'ex-section-item with a bare authored body',
	],
	[
		'<ex-section-item record="D5"><p>No answer control.</p></ex-section-item>',
		'exactly one native ex-choice',
	],
	[
		`<ex-section-item record="D5">${nativeChoice}${nativeChoice}</ex-section-item>`,
		'exactly one native ex-choice',
	],
	[
		`<ex-section-item record="D5">${nativeChoice.replace(' data-option="resume"', ' data-option="other"')}</ex-section-item>`,
		'unknown option other',
	],
	[
		`<ex-section-item record="D5">${nativeChoice.replace('<ex-choice>', '<ex-choice view="table">')}</ex-section-item>`,
		'Remove multiple, details or a non-card view',
	],
	[
		'<ex-section-item record="D7"><p>One.</p></ex-section-item><ex-section-item record="D7"><p>Two.</p></ex-section-item>',
		'Duplicate HTML id or record placement',
	],
	[
		'<ex-section-item record="D7" data-design-record="D7"><p>One.</p></ex-section-item>',
		'Write the record ID once',
	],
])('section record rejects invalid shape %#', (html, message) => {
	expect(() => checkRecordPage(html)).toThrow(message);
});
