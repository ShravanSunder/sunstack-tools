/** C4: only the root version and one proven config snapshot may change. */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';

import {
	arrayValue,
	objectValue,
	textValue,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';

export const restoreSettings = {
	html: { skip_tags: ['ex-markdown'], text_attributes: { 'ex-mermaid': ['label'] } },
	markdown: { heading_anchors: 'github' },
} as const;

export function configuredBytes(original: Buffer): Buffer {
	const config = objectValue(JSON.parse(original.toString('utf8')));
	const evidence = objectValue(config['evidence']);
	assert.equal(evidence['html'], undefined);
	assert.equal(evidence['markdown'], undefined);
	const settings = objectValue(JSON.parse(JSON.stringify(restoreSettings)));
	evidence['html'] = structuredClone(objectValue(settings['html']));
	evidence['markdown'] = structuredClone(objectValue(settings['markdown']));
	return Buffer.from(`${JSON.stringify(config, null, 2)}\n`);
}

export function requireConfigDelta(original: Buffer, configured: Buffer): void {
	assert.ok(
		configured.equals(configuredBytes(original)),
		'Config differs beyond the exact adopted additions',
	);
}

export function requireConfiguredParity(options: {
	before: Buffer;
	after: Buffer;
	originalConfig: Buffer;
	configuredConfig: Buffer;
	configPath: string;
}): void {
	requireConfigDelta(options.originalConfig, options.configuredConfig);
	const expected = objectValue(JSON.parse(options.before.toString('utf8')));
	const actual = objectValue(JSON.parse(options.after.toString('utf8')));
	assert.equal(expected['version'], '0.14.1');
	assert.equal(actual['version'], '0.15.0');
	const suffix = options.before.toString('utf8').endsWith('\n') ? '\n' : '';
	assert.equal(JSON.stringify(expected, null, 2) + suffix, options.before.toString('utf8'));
	assert.equal(expected['config'], options.configPath);
	assert.equal(actual['config'], options.configPath);
	const configs = arrayValue(expected['rulebook_files'])
		.map(objectValue)
		.filter((file) => file['path'] === options.configPath);
	assert.equal(configs.length, 1);
	const snapshot = configs[0];
	assert.ok(snapshot);
	assert.equal(snapshot['content'], options.originalConfig.toString('utf8'));
	assert.equal(
		snapshot['sha256'],
		createHash('sha256').update(options.originalConfig).digest('hex'),
	);
	snapshot['content'] = options.configuredConfig.toString('utf8');
	snapshot['sha256'] = createHash('sha256').update(options.configuredConfig).digest('hex');
	expected['version'] = '0.15.0';
	assert.equal(
		JSON.stringify(expected, null, 2) + suffix,
		options.after.toString('utf8'),
		'Report changed outside C4 config/version boundary',
	);
}

export function requireVersionOnly(before: Buffer, after: Buffer): void {
	const expected = objectValue(JSON.parse(before.toString('utf8')));
	const actual = objectValue(JSON.parse(after.toString('utf8')));
	assert.equal(expected['version'], '0.14.1');
	assert.equal(actual['version'], '0.15.0');
	const needle = '"version": "0.15.0"';
	assert.equal(after.toString('utf8').split(needle).length, 2);
	assert.equal(
		after.toString('utf8').replace(needle, '"version": "0.14.1"'),
		before.toString('utf8'),
	);
}

export function configSnapshot(report: JsonObject): JsonObject {
	const path = textValue(report['config']);
	const records = arrayValue(report['rulebook_files'])
		.map(objectValue)
		.filter((file) => file['path'] === path);
	assert.equal(records.length, 1);
	const record = records[0];
	assert.ok(record);
	return record;
}
