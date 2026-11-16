/** Public CLI and installed SDK count successful checked-file acquisition with matched demand. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';

import { z } from 'zod';

import {
	arrayValue,
	objectValue,
	parseObject,
	readObject,
	writeJson,
} from '../rulebook/rulebook-fixture-support.ts';
import type { RulebookProcess } from '../rulebook/rulebook-process.ts';
import { boundedSdkWait, SdkControlledBackend } from '../sdk/sdk-controlled-backend.ts';
import { requireSdkKind } from '../sdk/sdk-journeys.ts';
import { requireSdk, saveSdkEvidence, type SdkExports } from '../sdk/sdk-target.ts';
import { createByteFixture } from './byte-fixture.ts';

const scope = {
	id: 'scope',
	kind: 'rule',
	rule: 'approval',
	file: 'docs/pieces.html',
	lines: [1, 1],
	bytes: [0, 22],
	text_sha256: 'bbe437a632571a316c4650f10171eddf59e7c670e2996c3c1aa1f25d2fe641b8',
};
const sentinel = {
	id: 'sentinel',
	kind: 'rule',
	rule: 'approval',
	file: 'docs/sentinel.html',
	lines: [1, 1],
	bytes: [0, 25],
	text_sha256: '7f0476997b7e5dfadb8fa96c4d47f27aa773a43747bf47b60cfbdab34fa577e3',
};

function prepareRoot(root: string, endpoint: string): string {
	createByteFixture(root, endpoint);
	filesystem.writeFileSync(path.join(root, 'docs', 'pieces.html'), '<p>Selected scope.</p>\n');
	filesystem.writeFileSync(path.join(root, 'docs', 'sentinel.html'), '<p>Readable sentinel.</p>\n');
	return path.join(root, 'sunstack.lint.json');
}
function selectFile(configPath: string, file: string): void {
	const config = readObject(configPath);
	config['check'] = { files: [file] };
	writeJson(configPath, config);
}

export async function observeByteCliAcquisition(options: {
	readonly observer: RulebookProcess;
	readonly evidence: string;
}): Promise<void> {
	const root = path.join(options.evidence, 'cli-acquisition-fixture');
	const configPath = prepareRoot(root, options.observer.endpoint);
	const run = async (
		name: string,
		bindings: readonly object[],
	): Promise<{ count: unknown; captures: string[]; outcomes: unknown[] }> => {
		const file = path.join(options.evidence, `cli-acquisition-${name}.json`);
		writeJson(file, bindings);
		const result = await options.observer.run({
			root,
			name: `cli-acquisition-${name}`,
			arguments: [
				'lint',
				'--config',
				'sunstack.lint.json',
				'--all',
				'--format',
				'json',
				'--case-selection',
				file,
			],
			credential: true,
			expectedExit: 0,
		});
		const report = parseObject(result.stdout.toString('utf8'));
		const outcomes = arrayValue(objectValue(report['case_selection'])['bindings']).map(
			(row) => objectValue(objectValue(row)['outcome'])['kind'],
		);
		return {
			count: objectValue(report['usage'])['checked_files_acquired'],
			captures: result.captures.map((capture) => capture.raw.toString('utf8')),
			outcomes,
		};
	};
	selectFile(configPath, 'docs/sentinel.html');
	const positive = await run('sentinel-positive', [sentinel]);
	assert.equal(positive.count, 1);
	assert.equal(positive.captures.length, 1);
	assert.deepEqual(positive.outcomes, ['selected']);
	selectFile(configPath, 'docs/pieces.html');
	const normal = await run('scope-positive', [scope]);
	const mixed = await run('scope-plus-excluded', [scope, sentinel]);
	assert.equal(normal.count, 1);
	assert.equal(normal.captures.length, 1);
	assert.equal(mixed.count, normal.count);
	assert.deepEqual(mixed.captures, normal.captures);
	assert.deepEqual(mixed.outcomes, ['selected', 'unmatched']);
	writeJson(path.join(options.evidence, 'cli-acquisition-summary.json'), {
		positive,
		normal,
		mixed,
		limit: 'counts successful distinct acquisitions, not failed attempts',
	});
}

const recordSchema = z.record(z.string(), z.unknown());
export async function observeByteSdkAcquisition(options: {
	readonly sdk: SdkExports;
	readonly evidence: string;
}): Promise<void> {
	const backend = new SdkControlledBackend();
	const priorKey = process.env['OPENROUTER_API_KEY'];
	const priorStore = process.env['SUNSTACK_CREDENTIAL_STORE'];
	requireSdk(!priorKey, 'Controlled SDK probe requires no inherited provider key');
	process.env['OPENROUTER_API_KEY'] = 'SYNTHETIC_SDK_LOOPBACK_ONLY';
	process.env['SUNSTACK_CREDENTIAL_STORE'] = 'off';
	let started = false;
	try {
		await backend.start();
		started = true;
		const root = path.join(options.evidence, 'sdk-acquisition-fixture');
		const configPath = prepareRoot(root, backend.endpoint);
		const sdkBinding = (binding: typeof scope): object => ({
			...binding,
			lines: [1n, 1n],
			bytes: binding.id === 'scope' ? [0n, 22n] : [0n, 25n],
		});
		const run = async (
			name: string,
			bindings: readonly object[],
		): Promise<{ count: unknown; captures: string[]; outcomes: unknown[] }> => {
			const loaded = requireSdkKind(
				await boundedSdkWait(options.sdk.load({ directory: root }), `${name} load`),
				'loaded',
			);
			const before = backend.captures.length;
			const cancellation = new AbortController();
			const pending = options.sdk.run(loaded['value'], {
				root,
				scope: { kind: 'all' },
				case_selection: bindings,
				signal: cancellation.signal,
			});
			let outcome: z.infer<typeof recordSchema>;
			try {
				outcome = recordSchema.parse(await boundedSdkWait(pending, name));
			} finally {
				cancellation.abort();
				await boundedSdkWait(Promise.allSettled([pending]), `${name} settlement`);
			}
			assert.equal(outcome['kind'], 'completed');
			const report = recordSchema.parse(outcome['report']);
			const rows = z
				.array(recordSchema)
				.parse(recordSchema.parse(report['case_selection'])['bindings']);
			const result = {
				count: recordSchema.parse(report['usage'])['checked_files_acquired'],
				captures: backend.captures.slice(before).map((capture) => capture.raw),
				outcomes: rows.map((row) => recordSchema.parse(row['outcome'])['kind']),
			};
			saveSdkEvidence(path.join(options.evidence, `sdk-acquisition-${name}.json`), {
				outcome,
				result,
			});
			return result;
		};
		selectFile(configPath, 'docs/sentinel.html');
		const positive = await run('sentinel-positive', [sdkBinding(sentinel)]);
		assert.equal(positive.count, 1n);
		assert.equal(positive.captures.length, 1);
		assert.deepEqual(positive.outcomes, ['selected']);
		selectFile(configPath, 'docs/pieces.html');
		const normal = await run('scope-positive', [sdkBinding(scope)]);
		const mixed = await run('scope-plus-excluded', [sdkBinding(scope), sdkBinding(sentinel)]);
		assert.equal(normal.count, 1n);
		assert.equal(normal.captures.length, 1);
		assert.equal(mixed.count, normal.count);
		assert.deepEqual(mixed.captures, normal.captures);
		assert.deepEqual(mixed.outcomes, ['selected', 'unmatched']);
		assert.equal(backend.errors.length, 0);
	} finally {
		if (started) await backend.close();
		if (priorKey === undefined) delete process.env['OPENROUTER_API_KEY'];
		else process.env['OPENROUTER_API_KEY'] = priorKey;
		if (priorStore === undefined) delete process.env['SUNSTACK_CREDENTIAL_STORE'];
		else process.env['SUNSTACK_CREDENTIAL_STORE'] = priorStore;
	}
}
