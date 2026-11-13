/** Literal standard/configured outcomes, independent of generated candidate output. */
import assert from 'node:assert/strict';

import {
	arrayValue,
	objectValue,
	textValue,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';
import { askingHtml, askingTexts, outlineHtml } from './convention-fixtures.ts';

export function requireOutline(
	report: JsonObject,
	mode: 'restored' | 'standard' | 'generic',
): void {
	const headings =
		mode === 'restored'
			? ['Root', 'Generic', 'Visible']
			: mode === 'generic'
				? ['Root', 'Island', 'Visible']
				: ['Root', 'Island', 'Generic', 'Visible'];
	const pieces = arrayValue(report['pieces']).map(objectValue);
	assert.equal(pieces.length, headings.length);
	const starts = pieces.map(
		(piece) => objectValue(objectValue(piece['where'])['span'])['byte_start'],
	);
	assert.deepEqual(
		starts,
		headings.map((heading) =>
			Buffer.byteLength(
				outlineHtml.slice(0, outlineHtml.indexOf(`<h${heading === 'Root' ? 1 : 2}>${heading}`)),
			),
		),
	);
	assert.equal(objectValue(report['usage'])['model_requests'], 0);
}

export function requireAsking(report: JsonObject, mode: 'restored' | 'standard' | 'generic'): void {
	const requests = arrayValue(report['requests']).map(objectValue);
	assert.equal(requests.length, 1);
	const request = requests[0];
	assert.ok(request);
	assert.equal(request['status'], 'prepared');
	assert.equal(objectValue(objectValue(request['request'])['state'])['full'], askingTexts[mode]);
	const piece = objectValue(objectValue(objectValue(report['piece_evidence'])['1'])['piece']);
	assert.equal(piece['text'], askingHtml);
	assert.equal(objectValue(objectValue(piece['fields'])['full'])['value'], askingHtml);
	assert.deepEqual(objectValue(piece['model_view'])['cut'], []);
	assert.equal(objectValue(report['usage'])['model_requests'], 0);
}

export function requireAnchors(report: JsonObject, scheme: boolean, released = false): void {
	const results = arrayValue(report['results']).map(objectValue);
	assert.equal(results.length, 7);
	const pieces = Object.values(objectValue(report['piece_evidence'])).map((value) =>
		objectValue(objectValue(value)['piece']),
	);
	assert.equal(pieces.length, 7);
	const seen = new Set<string>();
	for (const piece of pieces) {
		const file = textValue(objectValue(piece['where'])['file']);
		const name = file.split('/').at(-1)?.replace('.html', '');
		assert.ok(name);
		assert.ok(!seen.has(name));
		seen.add(name);
		const targets = arrayValue(objectValue(objectValue(piece['fields'])['links'])['values']).map(
			objectValue,
		);
		assert.equal(targets.length, 1);
		const target = targets[0];
		assert.ok(target);
		assert.equal(target['exists'], true);
		const expected =
			name === 'unsupported' || (!scheme && ['heading', 'missing-heading'].includes(name))
				? 'unsupported'
				: ['missing-heading', 'missing-html'].includes(name)
					? 'missing'
					: 'found';
		if (name === 'file-only') assert.equal(target['anchor'] ?? null, null);
		else {
			const anchor = objectValue(target['anchor']);
			assert.equal(anchor['status'], expected, name);
			if (expected === 'unsupported' && !released)
				assert.equal(
					anchor['reason'],
					name === 'unsupported' ? 'unsupported_document' : 'heading_scheme_not_configured',
				);
		}
		const matches = results.filter((result) => result['file'] === file);
		assert.equal(matches.length, 1);
		const result = matches[0];
		assert.ok(result);
		assert.equal(
			result['outcome'],
			expected === 'unsupported' ? 'incomplete' : expected === 'missing' ? 'finding' : 'pass',
			name,
		);
		assert.equal(
			result['gap'],
			expected === 'unsupported' ? 'evidence_unavailable' : undefined,
			name,
		);
	}
	assert.equal(objectValue(report['usage'])['model_requests'], 0);
	assert.equal(arrayValue(report['requests']).length, 0);
	assert.equal(objectValue(report['status'])['kind'], 'incomplete');
}
