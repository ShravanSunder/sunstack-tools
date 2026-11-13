import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';

import { test } from 'vitest';

import { objectValue, type JsonObject } from '../rulebook/rulebook-fixture-support.ts';
import {
	configuredBytes,
	requireConfiguredParity,
	requireConfigDelta,
} from './configured-parity.ts';

function encoded(value: JsonObject): Buffer {
	return Buffer.from(JSON.stringify(value, null, 2));
}
function report(version: string, config: Buffer): JsonObject {
	return {
		version,
		config: '/project/config.json',
		rulebook_files: [
			{
				path: '/project/config.json',
				content: config.toString('utf8'),
				sha256: createHash('sha256').update(config).digest('hex'),
			},
		],
		requests: [{ input_sha256: 'original-hash', state: 'original' }],
		fields: { raw: 'original' },
	};
}
const originalConfig = Buffer.from('{"evidence":{"kinds":{}},"check":{"files":["*.html"]}}');
test('C4 permits only adopted config bytes and their computed snapshot digest plus root version', () => {
	const configuredConfig = configuredBytes(originalConfig);
	requireConfiguredParity({
		before: encoded(report('0.14.1', originalConfig)),
		after: encoded(report('0.15.0', configuredConfig)),
		originalConfig,
		configuredConfig,
		configPath: '/project/config.json',
	});
});
test.each(['requests', 'fields', 'rulebook_files'])(
	'C4 rejects unrelated %s mutations',
	(field) => {
		const configuredConfig = configuredBytes(originalConfig);
		const candidate = report('0.15.0', configuredConfig);
		candidate[field] = [];
		assert.throws(() =>
			requireConfiguredParity({
				before: encoded(report('0.14.1', originalConfig)),
				after: encoded(candidate),
				originalConfig,
				configuredConfig,
				configPath: '/project/config.json',
			}),
		);
	},
);
test('C4 rejects an extra config setting even with a correct snapshot hash', () => {
	const candidate = objectValue(JSON.parse(configuredBytes(originalConfig).toString('utf8')));
	candidate['max_files'] = 4;
	assert.throws(() => requireConfigDelta(originalConfig, encoded(candidate)));
});
