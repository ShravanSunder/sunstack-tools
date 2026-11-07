import { createHash } from 'node:crypto';

import { expect, test } from 'vitest';

import {
	preparedInventory,
	requireRequestParity,
	requireSelectionWaits,
} from './prepare-checks.ts';

const payload = {
	state: { body: 'Literal evidence', values: ['one', 'two'] },
	questions: { verdict: { type: 'noul' } },
};
const owner = {
	kind: 'rule',
	rule: 'observed',
	piece: { file: 'one.md', first_line: 1, last_line: 1 },
};
const preparedRecord = {
	id: 1,
	state: 'prepared',
	status: 'prepared',
	input_sha256: createHash('sha256').update(JSON.stringify(payload)).digest('hex'),
	owner,
	request: payload,
};
function reports(): {
	prepared: string;
	ordinary: string;
	captures: string[];
	expectedPrepared: number;
} {
	return {
		prepared: JSON.stringify({
			mode: 'prepare',
			requests: [preparedRecord],
		}),
		ordinary: JSON.stringify({ requests: [{ id: 9, owner, request: payload }] }),
		captures: [JSON.stringify({ model: 'scripted/pin', ...payload })],
		expectedPrepared: 1,
	};
}
test('prepared/report/actual capture comparison ignores unrelated result IDs', () => {
	expect(() => requireRequestParity(reports())).not.toThrow();
});
test.each([
	'missing',
	'duplicate',
	'value',
	'capture',
	'ordinary-mode',
	'hash',
	'outcome',
	'state',
])('parity refuses %s even when another boundary agrees', (changed) => {
	const input = reports();
	if (changed === 'missing') input.prepared = '{"mode":"prepare","requests":[]}';
	else if (changed === 'duplicate')
		input.prepared = JSON.stringify({
			mode: 'prepare',
			requests: [1, 2].map((id) => Object.assign({}, preparedRecord, { id })),
		});
	else if (changed === 'value')
		input.prepared = input.prepared.replace('Literal evidence', 'Wrong prepared evidence');
	else if (changed === 'capture')
		input.captures = [
			input.captures[0]?.replace('Literal evidence', 'Wrong transmitted evidence') ?? '',
		];
	else if (changed === 'hash')
		input.prepared = input.prepared.replace(preparedRecord.input_sha256, '0'.repeat(64));
	else if (changed === 'outcome')
		input.prepared = input.prepared.replace(
			'"status":"prepared"',
			'"status":"prepared","outcome":{}',
		);
	else if (changed === 'state')
		input.prepared = input.prepared.replace('"state":"prepared"', '"state":"run"');
	else input.ordinary = input.ordinary.replace('{"requests"', '{"mode":"prepare","requests"');
	expect(() => requireRequestParity(input)).toThrow();
});
test('prepared inventory refuses answers rather than trusting its status marker', () => {
	const raw = JSON.stringify({
		mode: 'prepare',
		requests: [{ ...preparedRecord, answers: {} }],
	});
	expect(() => preparedInventory(raw)).toThrow('invented answers');
});

function selectionReport(changed?: string): string {
	const file = 'skills/practices-research/SKILL.md';
	const selectionOwner = { kind: 'selection', selection_kind: 'workflow_section', file };
	if (changed === 'kind') selectionOwner.selection_kind = 'unrelated';
	if (changed === 'file') selectionOwner.file = 'other.md';
	return JSON.stringify({
		mode: 'prepare',
		requests: [{ ...preparedRecord, owner: changed === 'owner' ? owner : selectionOwner }],
		needs_answer: ['workflow_section', 'step'].map((kind) => ({
			kind,
			file,
			waits_on: { selection_kind: 'workflow_section', request_id: preparedRecord.id },
		})),
	});
}
test('selection dependencies identify the matching saved selection', () => {
	expect(() => requireSelectionWaits(selectionReport())).not.toThrow();
});
test.each(['owner', 'kind', 'file'])('selection dependency rejects mismatched %s', (changed) => {
	expect(() => requireSelectionWaits(selectionReport(changed))).toThrow(
		'Dangling selection dependency',
	);
});
