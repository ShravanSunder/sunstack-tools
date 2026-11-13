import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import operatingSystem from 'node:os';
import path from 'node:path';

import { test } from 'vitest';

import { arrayValue, objectValue, type JsonObject } from '../rulebook/rulebook-fixture-support.ts';
import { requireDefaultAnchors } from './default-anchors.ts';

function encoded(value: JsonObject): Buffer {
	return Buffer.from(JSON.stringify(value, null, 2));
}
function pair(): { before: JsonObject; after: JsonObject } {
	const originalResult = {
		id: 1,
		name: 'original',
		file: 'links.md',
		outcome: 'finding',
		step: 'check',
		path: ['check'],
		basis: 'mechanical',
	};
	const target = {
		exists: true,
		path: 'target.md',
		anchor: { status: 'found', fragment: 'heading' },
	};
	const piece = { fields: { links: { values: [target, { exists: false, path: 'missing.md' }] } } };
	const before = {
		version: '0.14.1',
		incomplete: [],
		requests: [],
		piece_evidence: { '1': { piece } },
		results: [originalResult],
		findings: [{ id: 1, severity: 'error' }],
		counts: { error: 1, warn: 0 },
		status: { kind: 'findings' },
	};
	const after = {
		version: '0.15.0',
		incomplete: [
			{
				field: 'links',
				rule: 'original',
				file: 'links.md',
				reason: { kind: 'evidence_unavailable' },
			},
		],
		requests: [],
		piece_evidence: {
			'1': {
				piece: {
					fields: {
						links: {
							values: [
								{
									exists: true,
									path: 'target.md',
									anchor: {
										status: 'unsupported',
										fragment: 'heading',
										reason: 'heading_scheme_not_configured',
									},
								},
								{ exists: false, path: 'missing.md' },
							],
						},
					},
				},
			},
		},
		results: [
			{
				id: 1,
				gap: 'evidence_unavailable',
				name: 'original',
				file: 'links.md',
				outcome: 'incomplete',
				step: 'check',
				path: ['check'],
			},
		],
		findings: [],
		counts: { error: 0, warn: 0 },
		status: { kind: 'incomplete' },
	};
	return { before, after };
}
test('unsupported dominates a missing file in the same owner and suppresses only its finding', () => {
	const root = filesystem.mkdtempSync(
		path.join(operatingSystem.tmpdir(), 'sunstack-config-anchor-'),
	);
	try {
		filesystem.writeFileSync(path.join(root, 'target.md'), '# Heading\n');
		const { before, after } = pair();
		assert.deepEqual(requireDefaultAnchors(encoded(before), encoded(after), root), {
			heading_targets: 1,
			target_documents: 1,
			incomplete_owners: 1,
			preserved_findings: 0,
		});
		const wrong = structuredClone(after);
		objectValue(
			arrayValue(
				objectValue(
					objectValue(
						objectValue(objectValue(objectValue(wrong['piece_evidence'])['1'])['piece'])['fields'],
					)['links'],
				)['values'],
			)[0],
		)['anchor'] = { status: 'found', fragment: 'heading' };
		assert.throws(() => requireDefaultAnchors(encoded(before), encoded(wrong), root));
		filesystem.writeFileSync(path.join(root, 'target.md'), '<div id="heading">Standard</div>');
		assert.throws(() => requireDefaultAnchors(encoded(before), encoded(after), root));
	} finally {
		filesystem.unlinkSync(path.join(root, 'target.md'));
		filesystem.rmdirSync(root);
	}
});
