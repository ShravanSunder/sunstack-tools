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
function prepared(scenario: RecoveryCase = control): JsonObject {
	const raw = Buffer.from(scenario.raw);
	const sections = recoverySections(scenario);
	const source = {
		kind: 'document',
		path: `${scenario.name}.html`,
		repository: 'checked_repository',
		content_identity: createHash('sha256').update(raw).digest('hex'),
	};
	const field = (start: number, end: number): JsonObject => ({
		state: 'text',
		value: raw.subarray(start, end).toString(),
		spans: [{ source, byte_start: start, byte_end: end }],
	});
	const pieces = sections.map((section, index) => ({
		where: {
			file: `${scenario.name}.html`,
			span: { source, byte_start: section.start, byte_end: section.end },
		},
		text: raw.subarray(section.start, section.end).toString(),
		model_view: {
			state: 'bounded',
			cut: [],
			fields: {
				full:
					index === 0 && scenario.nested
						? `${scenario.titles[0]} ${scenario.titles[1]}`
						: (scenario.titles[index] ?? ''),
				own: scenario.titles[index] ?? '',
				parent: index === 1 && scenario.nested ? scenario.titles[0] : null,
			},
		},
		fields: {
			full: field(section.start, section.end),
			own: field(section.start, section.own_end),
			parent: section.parent === null ? { state: 'absent' } : field(0, sections[0]?.own_end ?? -1),
		},
	}));
	return {
		status: { kind: 'prepared' },
		usage: { model_requests: 0 },
		incomplete: [],
		requests: pieces.map((piece, index) => {
			const title = scenario.titles[index];
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

function namedHeadingReport(): { report: JsonObject; scenario: RecoveryCase } {
	const scenario: RecoveryCase = {
		...control,
		name: 'unclosed',
		raw: '<h2>Parent<h3>Child',
		allowIncomplete: true,
	};
	const report = prepared(scenario);
	const value = '[cut: html_projection_unrecoverable_markup]';
	const view = objectValue(
		objectValue(objectValue(objectValue(report['piece_evidence'])['1'])['piece'])['model_view'],
	);
	view['cut'] = ['full'];
	objectValue(view['fields'])['full'] = value;
	const requests = report['requests'];
	assert.ok(Array.isArray(requests));
	const state = objectValue(objectValue(objectValue(requests[0])['request'])['state']);
	state['full'] = value;
	state['cut-fields'] = ['full'];
	return { report, scenario };
}
test('accepts a named field cut on explicitly unclosed headings while preserving raw spans', () => {
	const { report, scenario } = namedHeadingReport();
	requireRecoveryPrepared(report, [scenario]);
});
test('rejects named heading cuts missing from the asking request', () => {
	const { report, scenario } = namedHeadingReport();
	const requests = report['requests'];
	assert.ok(Array.isArray(requests));
	objectValue(objectValue(objectValue(requests[0])['request'])['state'])['cut-fields'] = [];
	assert.throws(() => requireRecoveryPrepared(report, [scenario]));
});
test('rejects a false incomplete on proper closed headings', () => {
	const { report } = namedHeadingReport();
	assert.throws(() => requireRecoveryPrepared(report, [{ ...control, name: 'unclosed' }]));
});
