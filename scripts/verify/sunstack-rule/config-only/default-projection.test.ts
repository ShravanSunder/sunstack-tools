import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';

import { test } from 'vitest';

import { arrayValue, objectValue, type JsonObject } from '../rulebook/rulebook-fixture-support.ts';
import { requireDefaultProjection } from './default-projection.ts';

function encoded(value: JsonObject): Buffer {
	return Buffer.from(JSON.stringify(value, null, 2));
}
function report(version: string, text: string): JsonObject {
	const request = { state: { full: text }, questions: {} };
	return {
		version,
		piece_evidence: {
			'1': {
				piece: {
					where: { file: 'page.html', span: { byte_start: 0, byte_end: 90 } },
					text: 'original raw',
					fields: { full: { value: 'original raw' } },
					model_view: { fields: { full: text }, cut: [] },
				},
			},
		},
		requests: [
			{
				id: 1,
				owner: { rule: 'K001' },
				request,
				input_sha256: createHash('sha256').update(JSON.stringify(request)).digest('hex'),
			},
		],
	};
}
test('C5 allows a materially different budgeted text only from the independent released witness', () => {
	requireDefaultProjection(
		encoded(report('0.14.1', 'prefix [label: Flow]')),
		encoded(report('0.14.1', 'prefix tail beyond previous cut')),
		encoded(report('0.15.0', 'prefix tail beyond previous cut')),
	);
});
test.each(['raw', 'request', 'hash', 'owner'])(
	'C5 rejects %s drift beside correct witness text',
	(field) => {
		const candidate = report('0.15.0', 'prefix tail');
		const request = objectValue(arrayValue(candidate['requests'])[0]);
		if (field === 'raw')
			objectValue(objectValue(objectValue(candidate['piece_evidence'])['1'])['piece'])['text'] =
				'changed raw';
		else if (field === 'request') request['request'] = { state: { full: 'old prefix' } };
		else if (field === 'hash') request['input_sha256'] = 'arbitrary';
		else request['owner'] = { rule: 'another' };
		assert.throws(() =>
			requireDefaultProjection(
				encoded(report('0.14.1', 'prefix label')),
				encoded(report('0.14.1', 'prefix tail')),
				encoded(candidate),
			),
		);
	},
);
