/** @typedef {import('../../../../../../packages/demistify/src/core/decision-record.ts').DecisionRecord} DecisionRecord */
/** @typedef {Extract<DecisionRecord, {kind: 'ruling'}>['authority'] & {state: 'owner-decided'}} OwnerAuthority */

/**
 * @param {OwnerAuthority['sourceKind']} sourceKind
 * @param {OwnerAuthority['source']} source
 * @returns {OwnerAuthority}
 */
const ownerDecision = (sourceKind, source) => ({
	state: 'owner-decided',
	sourceKind,
	decidedOn: '2026-11-25',
	source,
});

/**
 * @param {string} id
 * @param {string} question
 * @param {string} title
 * @param {OwnerAuthority['source']} source
 * @returns {Extract<DecisionRecord, {kind: 'choice'}>}
 */
const acceptedChoice = (id, question, title, source) => ({
	id,
	kind: 'choice',
	question,
	options: [
		{ id: 'keep', title },
		{ id: 'replace', title: 'Replace it' },
	],
	recommended: 'none',
	authority: {
		...ownerDecision('owner-accepted', source),
		chosen: 'keep',
		decision: title,
	},
});

/** @type {DecisionRecord[]} */
const records = [
	{
		id: 'D1',
		kind: 'ruling',
		statement: 'Limit retries to three attempts.',
		authority: ownerDecision('owner-words', {
			href: '#retry-source',
			excerpt: '3  retrys max\npls',
			shown: 'Allow at most three retries, please.',
			verbatim: true,
		}),
	},
	acceptedChoice('D2', 'Which queue do we use?', 'keep the same queu', {
		href: '#queue-source',
		excerpt: 'keep the same queu',
		shown: 'Keep the same queue.',
		verbatim: true,
	}),
	acceptedChoice('D3', 'What happens to the log?', 'Keep the log', {
		href: '#history-source',
		excerpt: 'Keep the log',
	}),
	{
		id: 'D5',
		kind: 'choice',
		title: 'Retry policy',
		question: 'How should a slow request be retried?',
		options: [
			{ id: 'wait', title: 'Wait before retrying' },
			{ id: 'stop', title: 'Stop after the first failure' },
		],
		recommended: { option: 'wait', 'by': { alias: 'Lead', role: 'lead' } },
		authority: { state: 'owner-open' },
	},
	{
		id: 'D4',
		kind: 'ruling',
		statement: 'Use one receipt.',
		authority: ownerDecision('owner-words', {
			href: '#receipt-source',
			excerpt: 'one reciept is fine',
			verbatim: true,
		}),
	},
];

document.currentScript.dataset.demistifyRecords = JSON.stringify({
	version: 1,
	pages: {
		'../owner-words.html': {
			placements: records.map((record) => ({
				reference: record.id,
				statement: record.kind === 'choice' ? record.question : record.statement,
				...(record.kind === 'choice' && { decision: record.id }),
				face: { record, dependents: [] },
			})),
			sectionItems: records.map((record) => ({
				reference: record.id,
				holds: record.authority.state === 'owner-open' ? 'needs-owner' : 'settled',
			})),
			trees: [],
		},
	},
});
