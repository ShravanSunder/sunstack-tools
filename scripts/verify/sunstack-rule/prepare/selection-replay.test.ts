import { createHash } from 'node:crypto';

import { expect, test } from 'vitest';

import { preparedInventory } from './prepare-checks.ts';

function replayFixture(): { source: string; prepared: string } {
	const request = {
		state: { candidates: ['literal ☃'] },
		questions: { candidate: { type: 'noul' } },
	};
	const recorded = {
		id: 17,
		state: 'run',
		owner: { kind: 'selection', selection_kind: 'workflow', file: 'one.md' },
		request,
		input_sha256: createHash('sha256').update(JSON.stringify(request)).digest('hex'),
		resolved_model: 'controlled/model-v1',
		outcome: {
			kind: 'completed',
			answers: { candidate: { type: 'noul', noul: 0.95 } },
			receipt: { resolved_model: 'controlled/model-v1' },
		},
		selection_evidence: { candidates: [{ model_view: 'literal ☃', cuts: [] }], shared: {} },
	};
	return {
		source: JSON.stringify({ requests: [recorded] }),
		prepared: JSON.stringify({
			mode: 'prepare',
			requests: [
				{ ...recorded, id: 8, state: 'from_report', status: 'from_report', source_request_id: 17 },
			],
		}),
	};
}
test('replay accepts only source-linked recorded selection bytes', () => {
	const fixture = replayFixture();
	expect(preparedInventory(fixture.prepared, fixture.source).size).toBe(1);
	expect(() => preparedInventory(fixture.prepared)).toThrow('Unexpected replayed selection');
});
test.each(['source-id', 'status', 'outcome', 'evidence', 'model', 'owner'])(
	'replay rejects altered %s',
	(changed) => {
		const fixture = replayFixture();
		let raw = fixture.prepared;
		if (changed === 'source-id')
			raw = raw.replace('"source_request_id":17', '"source_request_id":18');
		else if (changed === 'status')
			raw = raw.replace('"status":"from_report"', '"status":"prepared"');
		else if (changed === 'outcome') raw = raw.replace('"noul":0.95', '"noul":0.01');
		else if (changed === 'evidence')
			raw = raw.replace('"model_view":"literal ☃"', '"model_view":"invented"');
		else if (changed === 'model')
			raw = raw.replace('"resolved_model":"controlled/model-v1"', '"resolved_model":"other"');
		else raw = raw.replace('"file":"one.md"', '"file":"other.md"');
		expect(raw).not.toBe(fixture.prepared);
		expect(() => preparedInventory(raw, fixture.source)).toThrow();
	},
);
