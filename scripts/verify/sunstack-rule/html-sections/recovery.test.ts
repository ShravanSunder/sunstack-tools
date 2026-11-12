import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';

import { test } from 'vitest';

import { objectValue, type JsonObject } from '../rulebook/rulebook-fixture-support.ts';
import {
	recoveryCases,
	recoverySections,
	requireRecoveryPrepared,
	type RecoveryCase,
} from './recovery.ts';

const control: RecoveryCase = {
	name: 'proper',
	raw: '<h2>Parent</h2><h3>Child</h3>',
	markers: ['<h2>Parent', '<h3>Child'],
	titles: ['Parent', 'Child'],
	nested: true,
};
function prepared(): JsonObject {
	const raw = Buffer.from(control.raw);
	const sections = recoverySections(control);
	const source = {
		kind: 'document',
		path: 'proper.html',
		repository: 'checked_repository',
		content_identity: createHash('sha256').update(raw).digest('hex'),
	};
	const field = (start: number, end: number): JsonObject => ({
		state: 'text',
		value: raw.subarray(start, end).toString(),
		spans: [{ source, byte_start: start, byte_end: end }],
	});
	const pieces = sections.map((section) => ({
		where: {
			file: 'proper.html',
			span: { source, byte_start: section.start, byte_end: section.end },
		},
		text: raw.subarray(section.start, section.end).toString(),
		fields: {
			full: field(section.start, section.end),
			own: field(section.start, section.own_end),
			parent: section.parent === null ? { state: 'absent' } : field(0, 15),
		},
	}));
	return {
		status: { kind: 'prepared' },
		usage: { model_requests: 0 },
		incomplete: [],
		requests: pieces.map((piece, index) => {
			const title = control.titles[index];
			assert.ok(title);
			return {
				id: index + 1,
				status: 'prepared',
				owner: { kind: 'rule', piece: piece.where },
				request: {
					state: {
						full: index === 0 ? 'Parent Child' : title,
						own: title,
						parent: index === 0 ? null : 'Parent',
						'unavailable-fields': {},
					},
				},
			};
		}),
		piece_evidence: Object.fromEntries(
			pieces.map((piece, index) => [String(index + 1), { piece }]),
		),
	};
}
test('accepts two normally prepared recovered section owners', () => {
	requireRecoveryPrepared(prepared(), [control]);
});
test('rejects the old successful Prepared result with zero requests', () => {
	assert.throws(() =>
		requireRecoveryPrepared(
			{
				status: { kind: 'prepared' },
				usage: { model_requests: 0 },
				incomplete: [],
				requests: [],
				piece_evidence: {},
			},
			[control],
		),
	);
});
test('rejects recovered heading words glued in the prepared payload', () => {
	const report = prepared();
	const requests = report['requests'];
	assert.ok(Array.isArray(requests));
	objectValue(objectValue(objectValue(requests[0])['request'])['state'])['full'] = 'ParentChild';
	assert.throws(() => requireRecoveryPrepared(report, [control]));
});
test('rejects duplicated prepared owners despite the correct request total', () => {
	const report = prepared();
	const requests = report['requests'];
	assert.ok(Array.isArray(requests));
	const first = objectValue(requests[0]);
	objectValue(requests[1])['owner'] = objectValue(first['owner']);
	assert.throws(() => requireRecoveryPrepared(report, [control]));
});
test('keeps Unicode byte offsets and lexical sibling boundaries in the independent oracle', () => {
	const unicode = recoveryCases.find((item) => item.name === 'unicode');
	assert.ok(unicode);
	assert.deepEqual(
		recoverySections(unicode).map((section) => [section.start, section.end, section.parent]),
		[
			[3, 26, null],
			[17, 26, 0],
		],
	);
	const siblings = recoveryCases.find((item) => item.name === 'siblings');
	assert.ok(siblings);
	assert.deepEqual(
		recoverySections(siblings).map((section) => [section.start, section.end, section.parent]),
		[
			[0, 5, null],
			[5, 10, null],
		],
	);
});
