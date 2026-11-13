/** No-scheme transition for the fixed original corpus, which has no explicit HTML IDs. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';

import {
	arrayValue,
	numberValue,
	objectValue,
	textValue,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';

export function requireDefaultAnchors(before: Buffer, after: Buffer, root: string): JsonObject {
	const expected = objectValue(JSON.parse(before.toString('utf8')));
	const actual = objectValue(JSON.parse(after.toString('utf8')));
	assert.equal(expected['version'], '0.14.1');
	assert.equal(actual['version'], '0.15.0');
	assert.deepEqual(expected['incomplete'], []);
	assert.deepEqual(expected['requests'], []);
	const changed = new Set<number>();
	const inspected = new Set<string>();
	let targets = 0;
	for (const [owner, value] of Object.entries(objectValue(expected['piece_evidence']))) {
		const piece = objectValue(objectValue(value)['piece']);
		for (const item of arrayValue(objectValue(objectValue(piece['fields'])['links'])['values'])) {
			const target = objectValue(item);
			if (target['exists'] !== true || target['anchor'] === undefined) continue;
			const targetPath = textValue(target['path']);
			assert.ok(targetPath.endsWith('.md'));
			const absolute = path.resolve(root, targetPath);
			assert.ok(absolute.startsWith(`${path.resolve(root)}/`));
			if (!inspected.has(targetPath)) {
				// Conservative source guard: even an ID lookalike in code refuses this corpus-specific oracle.
				assert.doesNotMatch(
					filesystem.readFileSync(absolute, 'utf8'),
					/<[^>]*(?:\bid\s*=|\bname\s*=)[^>]*>/iu,
					`Explicit HTML identifier in ${targetPath}`,
				);
				inspected.add(targetPath);
			}
			const anchor = objectValue(target['anchor']);
			assert.ok(['found', 'missing'].includes(textValue(anchor['status'])));
			target['anchor'] = {
				status: 'unsupported',
				fragment: textValue(anchor['fragment']),
				reason: 'heading_scheme_not_configured',
			};
			changed.add(Number(owner));
			targets++;
		}
	}
	assert.ok(targets > 0, 'No heading targets reached');
	const incomplete: JsonObject[] = [];
	expected['results'] = arrayValue(expected['results']).map((value) => {
		const result = objectValue(value);
		if (!changed.has(numberValue(result['id']))) return result;
		assert.equal(result['basis'], 'mechanical');
		assert.equal(result['step'], 'check');
		assert.deepEqual(result['path'], ['check']);
		const output: JsonObject = {};
		for (const [key, field] of Object.entries(result)) {
			if (key === 'name') output['gap'] = 'evidence_unavailable';
			if (key !== 'basis') output[key] = key === 'outcome' ? 'incomplete' : field;
		}
		incomplete.push({
			field: 'links',
			rule: result['name'] ?? null,
			file: result['file'] ?? null,
			reason: { kind: 'evidence_unavailable' },
		});
		return output;
	});
	expected['incomplete'] = incomplete;
	const findings = arrayValue(expected['findings'])
		.map(objectValue)
		.filter((finding) => !changed.has(numberValue(finding['id'])));
	expected['findings'] = findings;
	expected['counts'] = {
		error: findings.filter((value) => value['severity'] === 'error').length,
		warn: findings.filter((value) => value['severity'] === 'warn').length,
	};
	expected['status'] = { kind: 'incomplete' };
	expected['version'] = '0.15.0';
	const suffix = before.toString('utf8').endsWith('\n') ? '\n' : '';
	assert.equal(
		JSON.stringify(expected, null, 2) + suffix,
		after.toString('utf8'),
		'Default Markdown changed beyond typed no-scheme transitions',
	);
	return {
		heading_targets: targets,
		target_documents: inspected.size,
		incomplete_owners: changed.size,
		preserved_findings: findings.length,
	};
}
