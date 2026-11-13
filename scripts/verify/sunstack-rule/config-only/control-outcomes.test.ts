import assert from 'node:assert/strict';

import { test } from 'vitest';

import { arrayValue, objectValue, type JsonObject } from '../rulebook/rulebook-fixture-support.ts';
import { requireAnchors } from './control-outcomes.ts';

function report(): JsonObject {
	const pieces: JsonObject = {};
	const results: JsonObject[] = [];
	for (const [index, name] of [
		'heading',
		'explicit',
		'missing-heading',
		'html',
		'missing-html',
		'file-only',
		'unsupported',
	].entries()) {
		const file = `links/${name}.html`;
		const status =
			name === 'unsupported' || name === 'heading' || name === 'missing-heading'
				? 'unsupported'
				: name === 'missing-html'
					? 'missing'
					: 'found';
		const anchor =
			name === 'file-only'
				? {}
				: {
						anchor: {
							status,
							fragment: 'fragment',
							...(status === 'unsupported'
								? {
										reason:
											name === 'unsupported'
												? 'unsupported_document'
												: 'heading_scheme_not_configured',
									}
								: {}),
						},
					};
		pieces[String(index + 1)] = {
			piece: { where: { file }, fields: { links: { values: [{ exists: true, ...anchor }] } } },
		};
		results.push({
			file,
			outcome: status === 'unsupported' ? 'incomplete' : status === 'missing' ? 'finding' : 'pass',
			...(status === 'unsupported' ? { gap: 'evidence_unavailable' } : {}),
		});
	}
	return {
		piece_evidence: pieces,
		results,
		usage: { model_requests: 0 },
		requests: [],
		status: { kind: 'incomplete' },
	};
}
test('each standard positive must pass despite another owner keeping the report incomplete', () => {
	const value = report();
	requireAnchors(value, false);
	const result = arrayValue(value['results'])
		.map(objectValue)
		.find((entry) => entry['file'] === 'links/explicit.html');
	assert.ok(result);
	result['outcome'] = 'incomplete';
	assert.throws(() => requireAnchors(value, false));
});
