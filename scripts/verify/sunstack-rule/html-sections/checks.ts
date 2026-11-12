/** Compare emitted raw sections and field spans against a separately pinned outline. */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';

import {
	arrayValue,
	objectValue,
	numberValue,
	textValue,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';
import type { Section } from './corpus.ts';

function requireTextField(options: {
	field: JsonObject;
	raw: Buffer;
	start: number;
	end: number;
	source: JsonObject;
}): void {
	assert.equal(options.field['state'], 'text');
	assert.equal(
		options.field['value'],
		options.raw.subarray(options.start, options.end).toString('utf8'),
	);
	const spans = arrayValue(options.field['spans']);
	assert.equal(spans.length, 1);
	const span = objectValue(spans[0]);
	assert.equal(span['byte_start'], options.start);
	assert.equal(span['byte_end'], options.end);
	assert.deepEqual(span['source'], options.source, 'Field span belongs to another source');
}
export function requireSections(options: {
	pieces: readonly JsonObject[];
	sections: readonly Section[];
	raw: Buffer;
}): void {
	assert.equal(options.pieces.length, options.sections.length, 'Outline count differs');
	const ordered = options.pieces.toSorted(
		(left, right) =>
			numberValue(objectValue(objectValue(left['where'])['span'])['byte_start']) -
			numberValue(objectValue(objectValue(right['where'])['span'])['byte_start']),
	);
	for (const [index, expected] of options.sections.entries()) {
		const piece = ordered[index];
		assert.ok(piece);
		const location = objectValue(piece['where']);
		const span = objectValue(location['span']);
		const source = objectValue(span['source']);
		assert.equal(source['path'], location['file']);
		assert.equal(
			source['content_identity'],
			createHash('sha256').update(options.raw).digest('hex'),
		);
		assert.equal(span['byte_start'], expected.start, 'Section begins at the wrong heading');
		assert.equal(
			span['byte_end'],
			expected.end,
			'Section leaks across its next same-or-higher heading',
		);
		assert.equal(
			piece['text'],
			options.raw.subarray(expected.start, expected.end).toString('utf8'),
		);
		const fields = objectValue(piece['fields']);
		requireTextField({
			field: objectValue(fields['full']),
			raw: options.raw,
			start: expected.start,
			end: expected.end,
			source,
		});
		requireTextField({
			field: objectValue(fields['own']),
			raw: options.raw,
			start: expected.start,
			end: expected.own_end,
			source,
		});
		if (expected.parent !== null) {
			const parent = options.sections[expected.parent];
			assert.ok(parent);
			requireTextField({
				field: objectValue(fields['parent']),
				raw: options.raw,
				start: parent.start,
				end: parent.own_end,
				source,
			});
		} else
			assert.deepEqual(
				fields['parent'],
				{ state: 'absent' },
				'Top-level enclosing text is normal absence, not unavailable',
			);
	}
}
export function requireCut(options: { piece: JsonObject; state: JsonObject; field: string }): void {
	const view = objectValue(options.piece['model_view']);
	assert.equal(view['state'], 'bounded');
	assert.ok(
		arrayValue(view['cut']).includes(options.field),
		'Truncated field has no model-view cut indication',
	);
	assert.ok(
		arrayValue(options.state['cut-fields']).includes(options.field),
		'Request lost the field cut indication',
	);
	const raw = textValue(objectValue(objectValue(options.piece['fields'])[options.field])['value']);
	assert.ok(
		textValue(options.state[options.field]).length < raw.length,
		'Cut control did not truncate its field',
	);
}
