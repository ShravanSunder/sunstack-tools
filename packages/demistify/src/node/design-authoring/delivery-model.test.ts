import typescript from 'typescript';
import { expect, it } from 'vitest';

import { parseDeliverablesView } from '../../core/deliverables-view.ts';
import { projectDelivery } from './delivery-model.ts';
import type { DeliveryProjection } from './delivery-model.ts';
import { deliverySdkSource } from './delivery-sdk-source.ts';
import { designSchema } from './schema.ts';

const source = { href: 'sources.md', excerpt: 'The agreed cancellation behavior.' };
const design = designSchema.parse({
	pages: [{ id: 'design', title: 'Design', href: 'design.html', icon: 'file' }],
	mapGroups: [],
	records: [
		{
			id: 'NEED-01',
			kind: 'need',
			title: 'Cancel queued work',
			statement: 'Operators can cancel queued work.',
			destination: { page: 'design.html', fragment: 'NEED-01' },
			sources: [source],
		},
		{
			id: 'RULE-01',
			kind: 'rule',
			title: 'Respect cancellation',
			statement: 'Cancelled work stays cancelled.',
			destination: { page: 'design.html', fragment: 'RULE-01' },
			sources: [source],
			relationships: [{ relation: 'serves', targetId: 'NEED-01' }],
		},
		{
			id: 'CHECK-01',
			kind: 'planned-check',
			title: 'Try cancelled work',
			status: 'planned',
			destination: { page: 'design.html', fragment: 'CHECK-01' },
			sources: [source],
			relationships: [{ relation: 'checks', targetId: 'RULE-01' }],
			cases: [
				{ given: 'Cancelled before claim', expect: 'Job is skipped' },
				{ given: 'Cancelled during claim', expect: 'Claim is rejected' },
				{ given: 'No cancellation', expect: 'Job begins' },
			],
		},
	],
});

it('type-checks scoped SDK results and included prerequisites against literal IDs', (): void => {
	const file = 'design-sdk.generated.ts';
	const authoringSource = `${deliverySdkSource}
const scopeRevision = '${'a'.repeat(64)}';
const context = {
	title: 'Skip cancelled work',
	destination: { page: 'plan.html', fragment: 'DEL-01' },
	scope: { implementationTasks: ['TASK-01'], verificationChecks: ['CHECK-01'] },
} as const;
const first = defineDeliverable({
	...context,
	id: 'DEL-01',
	implementation: { completedTasks: ['TASK-01'] },
	verification: { build: 'build-a', scopeRevision, outcomes: { 'CHECK-01': { kind: 'blocked', reason: 'Unavailable' } } },
	verificationHistory: [{ build: 'build-old', scopeRevision, outcomes: { 'CHECK-OLD': { kind: 'passed', observation: 'Previous check passed.', evidence: 'old.txt' } } }],
});
const firstId: 'DEL-01' = first.id;
const second = defineDeliverable({ ...context, id: 'DEL-02' });
const draft = defineDeliverable({ ...context, id: 'DEL-03', scope: {}, implementation: {} });
defineDelivery({}, [{ deliverable: second, dependsOn: [firstId] }, { deliverable: first }, { deliverable: draft }]);

// @ts-expect-error implementation task outside scope
defineDeliverable({ ...context, id: 'DEL-01', implementation: { completedTasks: ['TASK-99'] } });
// @ts-expect-error verification check outside scope
defineDeliverable({ ...context, id: 'DEL-01', verification: { build: 'build-a', scopeRevision, outcomes: { 'CHECK-99': { kind: 'blocked', reason: 'Unavailable' } } } });
// @ts-expect-error a passed outcome requires an observation and evidence
defineDeliverable({ ...context, id: 'DEL-01', verification: { build: 'build-a', scopeRevision, outcomes: { 'CHECK-01': { kind: 'passed' } } } });
// @ts-expect-error verification requires a scope revision
defineDeliverable({ ...context, id: 'DEL-01', verification: { build: 'build-a' } });
// @ts-expect-error prerequisite omitted from this delivery
defineDelivery({}, [{ deliverable: second, dependsOn: [firstId] }]);
`;
	const options: typescript.CompilerOptions = {
		strict: true,
		exactOptionalPropertyTypes: true,
		skipLibCheck: true,
		noEmit: true,
		types: [],
	};
	const host = typescript.createCompilerHost(options);
	const virtualHost: typescript.CompilerHost = {
		...host,
		getSourceFile(
			fileName,
			target,
			onError,
			shouldCreateNewSourceFile,
		): typescript.SourceFile | undefined {
			return fileName === file
				? typescript.createSourceFile(fileName, authoringSource, target, true)
				: host.getSourceFile(fileName, target, onError, shouldCreateNewSourceFile);
		},
	};
	const program = typescript.createProgram([file], options, virtualHost);

	expect(
		typescript
			.getPreEmitDiagnostics(program)
			.map((diagnostic): string =>
				typescript.flattenDiagnosticMessageText(diagnostic.messageText, '\n'),
			),
	).toEqual([]);
});
const recordRevisions = new Map([
	['NEED-01', 'b'.repeat(64)],
	['RULE-01', 'c'.repeat(64)],
	['CHECK-01', 'd'.repeat(64)],
]);
const candidateDraft = {
	id: 'DEL-01',
	title: 'Skip cancelled work',
	destination: { page: 'plan.html', fragment: 'DEL-01' },
	scope: {
		rules: ['RULE-01'],
		implementationTasks: ['TASK-01'],
		verificationChecks: ['CHECK-01'],
	},
	implementation: { completedTasks: ['TASK-01'], candidate: 'build-a', evidence: ['change.md'] },
};
const dependentDraft = { ...candidateDraft, id: 'DEL-02', title: 'Report already started work' };
const draft = projectDelivery(
	design,
	[{ deliverable: dependentDraft, dependsOn: ['DEL-01'] }, { deliverable: candidateDraft }],
	recordRevisions,
);

function scopeRevisionOf(projection: DeliveryProjection, id: string): string {
	const view = projection.deliverables[id];
	if (!view) throw new Error(`Expected a projected deliverable ${id}.`);
	return view.scopeRevision;
}

const scopeRevision = scopeRevisionOf(draft, 'DEL-01');
const verification = {
	build: 'build-a',
	scopeRevision,
	outcomes: {
		'CHECK-01': { kind: 'passed', observation: 'Job was skipped.', evidence: 'receipt.md' },
	},
};
const candidate = { ...candidateDraft, verification };
const dependent = {
	...dependentDraft,
	verification: { ...verification, scopeRevision: scopeRevisionOf(draft, 'DEL-02') },
};
const acceptance = {
	deliverableId: 'DEL-01',
	build: 'build-a',
	scopeRevision,
	evidence: 'acceptance.md',
};
const entries = [{ deliverable: dependent, dependsOn: ['DEL-01'] }, { deliverable: candidate }];

it('rejects an unrelated check and blocks a matching acceptance for uncovered rules', (): void => {
	const rule = design.records.find((record) => record.kind === 'rule');
	const check = design.records.find((record) => record.kind === 'planned-check');
	if (!rule || !check) throw new Error('Expected a rule and a planned check in the fixture.');
	const otherRule = {
		...rule,
		id: 'RULE-02',
		destination: { ...rule.destination, fragment: 'RULE-02' },
	};
	const otherCheck = {
		...check,
		relationships: [{ relation: 'checks' as const, targetId: otherRule.id }],
	};
	const revisions = new Map(recordRevisions).set(otherRule.id, '0'.repeat(64));
	const twoRules = { ...design, records: [...design.records, otherRule] };
	const unrelatedDesign = {
		...twoRules,
		records: twoRules.records.map((record) => (record.id === check.id ? otherCheck : record)),
	};

	expect(() =>
		projectDelivery(unrelatedDesign, [{ deliverable: candidateDraft }], revisions),
	).toThrow(/DEL-01.*CHECK-01.*scoped rule/i);

	const expanded = {
		...candidateDraft,
		scope: { ...candidateDraft.scope, rules: ['RULE-01', 'RULE-02'] },
	};
	const expandedRevision = scopeRevisionOf(
		projectDelivery(twoRules, [{ deliverable: expanded }], revisions),
		'DEL-01',
	);
	const completed = {
		...expanded,
		verification: { ...verification, scopeRevision: expandedRevision },
	};
	const receipt = { ...acceptance, scopeRevision: expandedRevision };
	const uncovered = projectDelivery(
		{ ...twoRules, acceptances: [receipt] },
		[{ deliverable: completed }],
		revisions,
	);
	expect(uncovered.deliverables['DEL-01']?.state).toBe('awaiting-verification');
	expect(uncovered.deliverables['DEL-01']?.reasons.join(' ')).toContain('RULE-02');

	const sharedCheck = {
		...check,
		relationships: [...check.relationships, ...otherCheck.relationships],
	};
	const sharedDesign = {
		...twoRules,
		records: twoRules.records.map((record) => (record.id === check.id ? sharedCheck : record)),
	};
	const sharedRevisions = new Map(revisions).set(check.id, '1'.repeat(64));
	const sharedRevision = scopeRevisionOf(
		projectDelivery(sharedDesign, [{ deliverable: expanded }], sharedRevisions),
		'DEL-01',
	);
	const covered = projectDelivery(
		{ ...sharedDesign, acceptances: [{ ...receipt, scopeRevision: sharedRevision }] },
		[
			{
				deliverable: {
					...completed,
					verification: { ...verification, scopeRevision: sharedRevision },
				},
			},
		],
		sharedRevisions,
	);
	expect(covered.deliverables['DEL-01']?.state).toBe('accepted');
});

it('preserves archived verification and Lead evidence on the versioned wire', (): void => {
	const check = design.records.find((record) => record.kind === 'planned-check');
	if (!check) throw new Error('Expected a planned check in the fixture.');
	const replacement = {
		...check,
		id: 'CHECK-02',
		destination: { ...check.destination, fragment: 'CHECK-02' },
	};
	const revisedDesign = {
		...design,
		records: [...design.records.filter(({ id }): boolean => id !== check.id), replacement],
	};
	const revisions = new Map(recordRevisions).set(replacement.id, '1'.repeat(64));
	const draftWithHistory = {
		...candidateDraft,
		scope: { ...candidateDraft.scope, verificationChecks: [replacement.id] },
		verificationHistory: [verification],
	};
	const view = projectDelivery(revisedDesign, [{ deliverable: draftWithHistory }], revisions);
	const newRevision = scopeRevisionOf(view, 'DEL-01');
	const currentVerification = {
		...verification,
		scopeRevision: newRevision,
		outcomes: { [replacement.id]: verification.outcomes['CHECK-01'] },
	};
	const currentAcceptance = { ...acceptance, scopeRevision: newRevision };
	const result = projectDelivery(
		{ ...revisedDesign, acceptances: [currentAcceptance] },
		[{ deliverable: { ...draftWithHistory, verification: currentVerification } }],
		revisions,
	);
	const snapshotRevision = '2'.repeat(64);
	const snapshot = parseDeliverablesView(
		JSON.stringify({ version: 1, snapshotRevision, deliverables: result.deliverables }),
	);

	expect(view.deliverables['DEL-01']).toMatchObject({
		state: 'awaiting-verification',
		verification: { checks: [{ id: replacement.id, state: 'missing' }], history: [verification] },
	});
	expect(view.evidence).toContain('receipt.md');
	expect(snapshot).toMatchObject({
		version: 1,
		snapshotRevision,
		deliverables: {
			'DEL-01': {
				state: 'accepted',
				progress: { acceptance: currentAcceptance },
				verification: { scopeRevision: newRevision, history: [verification] },
			},
		},
	});
	expect(() => parseDeliverablesView(JSON.stringify({ version: 1, deliverables: {} }))).toThrow(
		/snapshotRevision/,
	);
});

it('reopens an accepted candidate when its governing scope revision changes', (): void => {
	const designWithAcceptance = { ...design, acceptances: [acceptance] };
	const changedRevisions = new Map(recordRevisions).set('NEED-01', 'e'.repeat(64));
	const previouslyAccepted = projectDelivery(designWithAcceptance, entries, recordRevisions);
	const projected = projectDelivery(designWithAcceptance, entries, changedRevisions);
	const currentRevision = scopeRevisionOf(projected, 'DEL-01');
	const current = {
		...candidate,
		verification: { ...verification, scopeRevision: currentRevision },
	};
	const reverified = projectDelivery(
		designWithAcceptance,
		[{ deliverable: current }],
		changedRevisions,
	);
	const refreshedAcceptance = {
		...acceptance,
		scopeRevision: currentRevision,
		evidence: 'refresh.md',
	};
	const reaccepted = projectDelivery(
		{ ...design, acceptances: [acceptance, refreshedAcceptance] },
		[{ deliverable: current }],
		changedRevisions,
	);

	expect(previouslyAccepted.deliverables['DEL-01']).toMatchObject({
		state: 'accepted',
		scopeRevision,
	});
	expect(currentRevision).toMatch(/^[a-f0-9]{64}$/);
	expect(currentRevision).not.toEqual(scopeRevision);
	expect(projected.deliverables['DEL-01']).toMatchObject({
		state: 'awaiting-verification',
		verification: {
			build: 'build-a',
			scopeRevision,
			checks: [{ id: 'CHECK-01', state: 'stale', outcome: { kind: 'passed' } }],
		},
	});
	expect(projected.deliverables['DEL-01']?.reasons).toContain(
		'A recorded Lead acceptance covers a different scope revision.',
	);
	expect(projected.deliverables['DEL-02']).toMatchObject({ state: 'dependency-blocked' });
	expect(reverified.deliverables['DEL-01']).toMatchObject({
		state: 'ready-for-acceptance',
		scopeRevision: currentRevision,
	});
	expect(reaccepted.deliverables['DEL-01']).toMatchObject({
		state: 'accepted',
		progress: { acceptance: refreshedAcceptance },
	});
	expect(reaccepted.evidence).toEqual(expect.arrayContaining(['acceptance.md', 'refresh.md']));
	expect(reaccepted.deliverables['DEL-01']?.reasons).toEqual([
		'The Lead accepted candidate build-a.',
	]);
});

it('retains a verification scope when scoped task and prerequisite lists are reordered', (): void => {
	const work = {
		...candidateDraft,
		scope: { ...candidateDraft.scope, implementationTasks: ['TASK-01', 'TASK-02'] },
	};
	const prerequisites = [
		{ deliverable: { ...candidateDraft, id: 'DEL-02' } },
		{ deliverable: { ...candidateDraft, id: 'DEL-03' } },
	];
	const first = projectDelivery(
		design,
		[...prerequisites, { deliverable: work, dependsOn: ['DEL-02', 'DEL-03'] }],
		recordRevisions,
	);
	const reordered = {
		...work,
		scope: { ...work.scope, implementationTasks: ['TASK-02', 'TASK-01'] },
	};
	const second = projectDelivery(
		design,
		[...prerequisites, { deliverable: reordered, dependsOn: ['DEL-03', 'DEL-02'] }],
		recordRevisions,
	);
	expect(scopeRevisionOf(second, 'DEL-01')).toBe(scopeRevisionOf(first, 'DEL-01'));
});

it('reopens the same build when a completed task is added to its declared scope', (): void => {
	const changedScope = {
		...candidate,
		scope: { ...candidate.scope, implementationTasks: ['TASK-01', 'TASK-02'] },
		implementation: { ...candidate.implementation, completedTasks: ['TASK-01', 'TASK-02'] },
	};
	const projected = projectDelivery(
		{ ...design, acceptances: [acceptance] },
		[{ deliverable: changedScope }],
		recordRevisions,
	);

	expect(scopeRevisionOf(projected, 'DEL-01')).not.toBe(scopeRevision);
	expect(projected.deliverables['DEL-01']).toMatchObject({
		state: 'awaiting-verification',
		verification: { checks: [{ id: 'CHECK-01', state: 'stale', outcome: { kind: 'passed' } }] },
	});
});

it('requires a Lead receipt for the current build to clear a prerequisite', (): void => {
	const before = projectDelivery(design, entries, recordRevisions);
	const after = projectDelivery({ ...design, acceptances: [acceptance] }, entries, recordRevisions);

	expect(before.deliverables).toMatchObject({
		'DEL-01': { state: 'ready-for-acceptance' },
		'DEL-02': {
			state: 'dependency-blocked',
			dependsOn: ['DEL-01'],
			verification: { checks: [{ id: 'CHECK-01', state: 'passed' }] },
		},
	});
	expect(after.deliverables).toMatchObject({
		'DEL-01': { state: 'accepted' },
		'DEL-02': {
			state: 'ready-for-acceptance',
			links: {
				verificationChecks: [{ id: 'CHECK-01', page: 'design.html', fragment: 'CHECK-01' }],
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
			scopeRevision,
			outcomes: { 'CHECK-01': { kind: 'blocked', reason: 'Environment unavailable.' } },
		},
	};

	expect(
		projectDelivery(designWithAcceptance, [{ deliverable: next }], recordRevisions).deliverables,
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
		projectDelivery(designWithAcceptance, [{ deliverable: currentBlock }], recordRevisions)
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
		scope: { ...candidate.scope, implementationTasks: ['TASK-01', 'TASK-02'] },
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
		projectDelivery(designWithAcceptance, [{ deliverable }], recordRevisions).deliverables[
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
		projectDelivery(design, [{ deliverable: candidate, dependsOn: ['DEL-99'] }], recordRevisions),
	).toThrow(/DEL-01.*unknown prerequisite.*DEL-99/i);
	expect(() =>
		projectDelivery(
			design,
			[...entries.slice(0, 1), { deliverable: candidate, dependsOn: ['DEL-02'] }],
			recordRevisions,
		),
	).toThrow(/cycle.*DEL-02.*DEL-01.*DEL-02/i);
	expect(() =>
		projectDelivery(
			design,
			[{ deliverable: candidate }, { deliverable: candidate }],
			recordRevisions,
		),
	).toThrow(/DEL-01.*duplicate/i);
	expect(() =>
		projectDelivery(
			design,
			[
				{
					deliverable: { ...candidate, implementation: { completedTasks: ['TASK-99'] } },
				},
			],
			recordRevisions,
		),
	).toThrow(/DEL-01.*unknown implementation task.*TASK-99/i);
	expect(() =>
		projectDelivery(
			design,
			[
				{
					deliverable: {
						...candidate,
						verification: {
							build: 'build-a',
							scopeRevision,
							outcomes: { 'CHECK-99': candidate.verification.outcomes['CHECK-01'] },
						},
					},
				},
			],
			recordRevisions,
		),
	).toThrow(/DEL-01.*unknown verification check.*CHECK-99/i);
});
