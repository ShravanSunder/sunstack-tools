import { expect, it } from 'vitest';

import { parseDeliverablesView } from '../../core/deliverables-view.ts';
import { parseDeliveryEntries, projectDelivery } from './delivery-model.ts';
import { deliverySdkSource } from './delivery-sdk-source.ts';
import { designTypeSource } from './design-type-source.ts';
import { checkAuthoringTypes } from './read-design.ts';
import { designSchema } from './schema.ts';

const design = designSchema.parse({
	pages: [{ id: 'design', title: 'Design', href: 'design.html', icon: 'file' }],
	mapGroups: [],
	records: [
		{
			id: 'NEED-01',
			kind: 'need',
			title: 'Cancel queued work',
			statement: 'Operators can cancel queued work.',
		},
		{
			id: 'RULE-01',
			kind: 'rule',
			title: 'Respect cancellation',
			statement: 'Cancelled work stays cancelled.',

			serves: 'NEED-01',
		},
		{
			id: 'CHECK-01',
			kind: 'planned-check',
			title: 'Try cancelled work',
			statement: 'Exercise cancellation on the worker.',
			journey: 'Worker queue tests',
			unverified: 'Live traffic',

			checks: 'RULE-01',
			cases: [
				{ given: 'Cancelled before claim', expect: 'Job is skipped' },
				{ given: 'Cancelled during claim', expect: 'Claim is rejected' },
				{ given: 'No cancellation', expect: 'Job begins' },
			],
		},
	],
});

it('type-checks scoped SDK results and included prerequisites against literal IDs', (): void => {
	const file = '/design.ts';
	const authoringSource = `import { defineDelivery, defineDeliverable, defineDesign, defineNeed } from './design-sdk.generated.ts';
const context = {
	kind: 'core',
	title: 'Skip cancelled work',
	destination: { page: 'plan.html', fragment: 'DEL-01' },
	scope: { issues: [{ id: 'TASK-01', title: 'TASK 01' }], verificationChecks: ['CHECK-01'] },
} as const;
const first = defineDeliverable({
	...context,
	id: 'DEL-01',
	implementation: { closedIssues: ['TASK-01'] },
	verification: { build: 'build-a', outcomes: { 'CHECK-01': { kind: 'blocked', reason: 'Unavailable' } } },
	verificationHistory: [{ build: 'build-old', outcomes: { 'CHECK-OLD': { kind: 'passed', observation: 'Previous check passed.', evidence: 'old.txt' } } }],
});
const firstId: 'DEL-01' = first.id;
const second = defineDeliverable({ ...context, id: 'DEL-02' });
const draft = defineDeliverable({ ...context, id: 'DEL-03', scope: {}, implementation: {} });
defineDelivery({}, [{ deliverable: second, blockedBy: [firstId] }, { deliverable: first }, { deliverable: draft }]);

// @ts-expect-error issue outside scope
defineDeliverable({ ...context, id: 'DEL-01', implementation: { closedIssues: ['TASK-99'] } });
// @ts-expect-error verification check outside scope
defineDeliverable({ ...context, id: 'DEL-01', verification: { build: 'build-a', outcomes: { 'CHECK-99': { kind: 'blocked', reason: 'Unavailable' } } } });
// @ts-expect-error a passed outcome requires an observation and evidence
defineDeliverable({ ...context, id: 'DEL-01', verification: { build: 'build-a', outcomes: { 'CHECK-01': { kind: 'passed' } } } });

// @ts-expect-error blocking deliverable omitted from this delivery
defineDelivery({}, [{ deliverable: second, blockedBy: [firstId] }]);
const need = defineNeed({ id: 'N1', statement: 'Read the records.' });
const thematic = { pages: [{ id: 'home', href: 'index.html', title: 'Home', icon: 'file' }], records: [need] } as const;
defineDesign({ ...thematic, mapGroups: [{ description: 'Known', icon: 'file', target: { record: 'N1' } }] });
defineDesign({ ...thematic, mapGroups: [
	// @ts-expect-error unknown record ID
	{ description: 'Unknown', icon: 'file', target: { record: 'N2' } },
] });
`;
	expect(() =>
		checkAuthoringTypes(
			[file],
			new Map([
				[file, authoringSource],
				['/design-sdk.generated.ts', deliverySdkSource],
				['/design-schema.d.ts', designTypeSource],
			]),
		),
	).not.toThrow();
});
const recordDestinations = new Map(
	design.records.map(({ id }) => [id, { page: 'design.html', fragment: id }]),
);

const candidateDraft = {
	kind: 'core',
	id: 'DEL-01',
	title: 'Skip cancelled work',
	destination: { page: 'plan.html', fragment: 'DEL-01' },
	scope: {
		rules: ['RULE-01'],
		issues: [{ id: 'TASK-01', title: 'TASK 01' }],
		verificationChecks: ['CHECK-01'],
	},
	implementation: { closedIssues: ['TASK-01'], candidate: 'build-a', evidence: ['change.md'] },
};
const dependentDraft = { ...candidateDraft, id: 'DEL-02', title: 'Report already started work' };
const verification = {
	build: 'build-a',

	outcomes: {
		'CHECK-01': { kind: 'passed', observation: 'Job was skipped.', evidence: 'receipt.md' },
	},
};
const candidate = { ...candidateDraft, verification };
const dependent = {
	...dependentDraft,
	verification,
};
const acceptance = {
	deliverableId: 'DEL-01',
	build: 'build-a',

	evidence: 'acceptance.md',
};
const entries = [{ deliverable: dependent, blockedBy: ['DEL-01'] }, { deliverable: candidate }];

it('rejects an unrelated check and blocks a matching acceptance for uncovered rules', (): void => {
	const rule = design.records.find((record) => record.kind === 'rule');
	const check = design.records.find((record) => record.kind === 'planned-check');
	if (!rule || !check) throw new Error('Expected a rule and a planned check in the fixture.');
	const otherRule = {
		...rule,
		id: 'RULE-02',
	};
	const otherCheck = {
		...check,
		checks: otherRule.id,
	};

	const destinations = recordDestinations;
	const twoRules = { ...design, records: [...design.records, otherRule] };
	const unrelatedDesign = {
		...twoRules,
		records: twoRules.records.map((record) => (record.id === check.id ? otherCheck : record)),
	};

	expect(() =>
		projectDelivery(unrelatedDesign, [{ deliverable: candidateDraft }], destinations),
	).toThrow(/DEL-01.*CHECK-01.*scoped rule/i);

	const expanded = {
		...candidateDraft,
		scope: { ...candidateDraft.scope, rules: ['RULE-01', 'RULE-02'] },
	};
	const completed = { ...expanded, verification };
	const receipt = acceptance;
	const uncovered = projectDelivery(
		{ ...twoRules, acceptances: [receipt] },
		[{ deliverable: completed }],
		destinations,
	);
	expect(uncovered.deliverables['DEL-01']?.state).toBe('awaiting-verification');
	expect(uncovered.deliverables['DEL-01']).toMatchObject({
		reasons: ['Assign planned verification checks for scoped rules RULE-02.'],
	});

	const sharedCheck = {
		...check,
		checks: ['RULE-01', 'RULE-02'],
	};
	const sharedDesign = {
		...twoRules,
		records: twoRules.records.map((record) => (record.id === check.id ? sharedCheck : record)),
	};
	const covered = projectDelivery(
		{ ...sharedDesign, acceptances: [receipt] },
		[{ deliverable: completed }],
		destinations,
	);
	expect(covered.deliverables['DEL-01']?.state).toBe('accepted');
});

it('preserves archived verification and Lead evidence on the versioned wire', (): void => {
	const check = design.records.find((record) => record.kind === 'planned-check');
	if (!check) throw new Error('Expected a planned check in the fixture.');
	const replacement = {
		...check,
		id: 'CHECK-02',
	};
	const revisedDesign = {
		...design,
		records: [...design.records.filter(({ id }): boolean => id !== check.id), replacement],
	};

	const destinations = recordDestinations;
	const draftWithHistory = {
		...candidateDraft,
		scope: { ...candidateDraft.scope, verificationChecks: [replacement.id] },
		verificationHistory: [verification],
	};
	const view = projectDelivery(revisedDesign, [{ deliverable: draftWithHistory }], destinations);

	const currentVerification = {
		...verification,
		outcomes: { [replacement.id]: verification.outcomes['CHECK-01'] },
	};
	const currentAcceptance = { ...acceptance };
	const result = projectDelivery(
		{ ...revisedDesign, acceptances: [currentAcceptance] },
		[{ deliverable: { ...draftWithHistory, verification: currentVerification } }],
		destinations,
	);

	const snapshot = parseDeliverablesView(
		JSON.stringify({ version: 2, deliverables: result.deliverables }),
	);

	expect(view.deliverables['DEL-01']).toMatchObject({
		state: 'awaiting-verification',
		verification: { checks: [{ id: replacement.id, state: 'missing' }], history: [verification] },
	});
	expect(view.evidence).toContain('receipt.md');
	expect(snapshot).toMatchObject({
		version: 2,
		deliverables: {
			'DEL-01': {
				state: 'accepted',
				progress: { acceptance: currentAcceptance },
				verification: { history: [verification] },
			},
		},
	});
});

it('retains acceptance across statement edits on the same build', (): void => {
	const changed = {
		...design,
		acceptances: [acceptance],
		records: design.records.map((record) =>
			record.kind === 'rule' ? { ...record, statement: 'Cancelled work stays terminal.' } : record,
		),
	};

	expect(
		projectDelivery(changed, entries, recordDestinations).deliverables['DEL-01'],
	).toMatchObject({
		state: 'accepted',
		progress: { acceptance },
	});
});

it('reports a scoped rule without a need and lists later work without scope checks', (): void => {
	const changed = {
		...design,
		acceptances: [acceptance],
		records: design.records.map((record) =>
			record.kind === 'rule' ? { ...record, serves: undefined } : record,
		),
	};
	const later = { kind: 'later', id: 'DEL-LATER', title: 'Consider additional queues' };
	const result = projectDelivery(changed, [...entries, { deliverable: later }], recordDestinations);
	expect(result.deliverables['DEL-01']).toMatchObject({
		state: 'awaiting-verification',
		reasons: ['Link scoped rules to a need: RULE-01.'],
	});
	expect(result.deliverables['DEL-LATER']).toEqual({ ...later, state: 'later' });
});

it('requires a Lead receipt for the current build to clear a prerequisite', (): void => {
	const before = projectDelivery(design, entries, recordDestinations);
	const after = projectDelivery(
		{ ...design, acceptances: [acceptance] },
		entries,
		recordDestinations,
	);

	expect(before.deliverables).toMatchObject({
		'DEL-01': { state: 'ready-for-acceptance' },
		'DEL-02': {
			state: 'dependency-blocked',
			blockedBy: ['DEL-01'],
			verification: { checks: [{ id: 'CHECK-01', state: 'passed' }] },
		},
	});
	expect(after.deliverables).toMatchObject({
		'DEL-01': { state: 'accepted' },
		'DEL-02': {
			state: 'ready-for-acceptance',
			links: {
				verificationChecks: [
					{
						id: 'CHECK-01',
						label: 'Try cancelled work',
						destination: { page: 'design.html', fragment: 'CHECK-01' },
					},
				],
			},
		},
	});
	expect(after.destinations).toContainEqual({ page: 'plan.html', fragment: 'DEL-01' });
	expect(after.evidence).toEqual(
		expect.arrayContaining(['change.md', 'receipt.md', 'acceptance.md']),
	);
});

it('keeps stale verification visible, and replaces its credit with the current check status', (): void => {
	const designWithAcceptance = { ...design, acceptances: [acceptance] };
	const next = {
		...candidate,
		implementation: { ...candidate.implementation, candidate: 'build-b' },
	};
	const currentBlock = {
		...next,
		verification: {
			build: 'build-b',

			outcomes: { 'CHECK-01': { kind: 'blocked', reason: 'Environment unavailable.' } },
		},
	};

	expect(
		projectDelivery(designWithAcceptance, [{ deliverable: next }], recordDestinations).deliverables,
	).toMatchObject({
		'DEL-01': {
			state: 'awaiting-verification',
			verification: {
				build: 'build-a',
				checks: [{ id: 'CHECK-01', state: 'stale', outcome: { kind: 'passed' } }],
			},
		},
	});
	expect(
		projectDelivery(designWithAcceptance, [{ deliverable: currentBlock }], recordDestinations)
			.deliverables,
	).toMatchObject({
		'DEL-01': {
			state: 'verification-blocked',
			reasons: ['CHECK-01 is blocked. Environment unavailable.', expect.any(String)],
			verification: {
				checks: [
					{ id: 'CHECK-01', state: 'blocked', outcome: { reason: 'Environment unavailable.' } },
				],
			},
		},
	});
});

it('keeps changed candidates and incomplete scoped work open despite a Lead receipt', (): void => {
	const designWithAcceptance = { ...design, acceptances: [acceptance] };
	const next = {
		...candidate,
		implementation: { ...candidate.implementation, candidate: 'build-b' },
		verification: { ...candidate.verification, build: 'build-b' },
	};
	const unfinished = {
		...candidate,
		scope: {
			...candidate.scope,
			issues: [
				{ id: 'TASK-01', title: 'TASK 01' },
				{ id: 'TASK-02', title: 'TASK 02' },
			],
		},
	};
	const failed = {
		...candidate,
		verification: {
			...candidate.verification,
			outcomes: {
				'CHECK-01': { kind: 'failed', observation: 'Job began.', evidence: 'receipt.md' },
			},
		},
	};
	const project = (deliverable: typeof candidate): unknown =>
		projectDelivery(designWithAcceptance, [{ deliverable }], recordDestinations).deliverables[
			'DEL-01'
		];

	expect(project(next)).toMatchObject({ state: 'ready-for-acceptance' });
	expect(project(unfinished)).toMatchObject({ state: 'implementation-in-progress' });
	expect(project(failed)).toMatchObject({
		state: 'verification-failed',
		verification: { checks: [{ id: 'CHECK-01', outcome: { observation: 'Job began.' } }] },
	});
});

it('rejects invalid dependency graphs and role results with a deliverable ID in the error', (): void => {
	expect(() =>
		projectDelivery(
			design,
			[{ deliverable: candidate, blockedBy: ['DEL-99'] }],
			recordDestinations,
		),
	).toThrow(/DEL-01.*unknown blocking deliverable.*DEL-99/i);
	expect(() =>
		projectDelivery(
			design,
			[...entries.slice(0, 1), { deliverable: candidate, blockedBy: ['DEL-02'] }],
			recordDestinations,
		),
	).toThrow(/cycle.*DEL-02.*DEL-01.*DEL-02/i);
	expect(() =>
		projectDelivery(
			design,
			[{ deliverable: candidate }, { deliverable: candidate }],
			recordDestinations,
		),
	).toThrow(/DEL-01.*duplicate/i);
	expect(() =>
		projectDelivery(
			design,
			[
				{
					deliverable: { ...candidate, implementation: { closedIssues: ['TASK-99'] } },
				},
			],
			recordDestinations,
		),
	).toThrow(/DEL-01.*unknown issue.*TASK-99/i);
	expect(() =>
		parseDeliveryEntries([{ deliverable: { ...candidate, builder: 'old' }, dependsOn: [] }]),
	).toThrow(/DEL-01.*builder is now implementer.*dependsOn is now blockedBy/);
	expect(() =>
		projectDelivery(
			design,
			[
				{
					deliverable: {
						...candidate,
						verification: {
							build: 'build-a',

							outcomes: { 'CHECK-99': candidate.verification.outcomes['CHECK-01'] },
						},
					},
				},
			],
			recordDestinations,
		),
	).toThrow(/DEL-01.*unknown verification check.*CHECK-99/i);
});
