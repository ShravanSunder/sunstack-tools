/** Draft installed-SDK observations; actual target execution is separately commissioned. */
import assert from 'node:assert/strict';
import path from 'node:path';
import { isDeepStrictEqual } from 'node:util';

import { z } from 'zod';

import { objectValue, readObject, writeJson } from '../rulebook/rulebook-fixture-support.ts';
import { boundedSdkWait, SdkControlledBackend } from '../sdk/sdk-controlled-backend.ts';
import { requireSdkKind } from '../sdk/sdk-journeys.ts';
import { requireSdk, saveSdkEvidence, type SdkExports } from '../sdk/sdk-target.ts';
import { byteFixtureCases, byteFixturePath, createByteFixture } from './byte-fixture.ts';

const recordSchema = z.record(z.string(), z.unknown());

function selectionRow(report: unknown): z.infer<typeof recordSchema> {
	const record = recordSchema.parse(report);
	const selection = recordSchema.parse(record['case_selection']);
	const rows = z.array(recordSchema).parse(selection['bindings']);
	assert.equal(rows.length, 1);
	return recordSchema.parse(rows[0]);
}

export async function observeInstalledByteSdk(options: {
	readonly sdk: SdkExports;
	readonly evidence: string;
}): Promise<void> {
	const backend = new SdkControlledBackend();
	const previousKey = process.env['OPENROUTER_API_KEY'];
	const previousStore = process.env['SUNSTACK_CREDENTIAL_STORE'];
	requireSdk(!previousKey, 'Controlled SDK probe requires no inherited provider key');
	process.env['OPENROUTER_API_KEY'] = 'SYNTHETIC_SDK_LOOPBACK_ONLY';
	process.env['SUNSTACK_CREDENTIAL_STORE'] = 'off';
	let started = false;
	try {
		await backend.start();
		started = true;
		const root = path.join(options.evidence, 'sdk-fixture');
		createByteFixture(root, backend.endpoint);
		const loaded = requireSdkKind(
			await boundedSdkWait(options.sdk.load({ directory: root }), 'byte SDK load'),
			'loaded',
		);
		const first = byteFixtureCases[0];
		const binding = {
			id: 'selected',
			kind: 'rule',
			rule: 'approval',
			file: byteFixturePath,
			lines: [1n, 1n],
			bytes: [0n, 16n],
			text_sha256: first.text_sha256,
		};
		const run = async (
			name: string,
			selected: object,
		): Promise<{ outcome: z.infer<typeof recordSchema>; captures: number }> => {
			const before = backend.captures.length;
			const cancellation = new AbortController();
			const pending = options.sdk.run(loaded['value'], {
				root,
				scope: { kind: 'all' },
				case_selection: [selected],
				signal: cancellation.signal,
			});
			let outcome: z.infer<typeof recordSchema>;
			try {
				outcome = recordSchema.parse(await boundedSdkWait(pending, name));
			} finally {
				cancellation.abort();
				await boundedSdkWait(Promise.allSettled([pending]), `${name} settlement`);
			}
			const captures = backend.captures.length - before;
			saveSdkEvidence(path.join(options.evidence, `sdk-${name}.json`), { outcome, captures });
			return { outcome, captures };
		};
		const positive = await run('selected', binding);
		assert.equal(positive.outcome['kind'], 'completed');
		assert.equal(positive.captures, 1, 'Positive did not witness controlled SDK request');
		const row = selectionRow(positive.outcome['report']);
		assert.deepEqual(recordSchema.parse(row['binding'])['bytes'], binding.bytes);
		assert.equal(recordSchema.parse(row['outcome'])['kind'], 'selected');
		const requireInvalid = async (name: string, bytes: readonly bigint[]): Promise<void> => {
			const invalid = await run(name, { ...binding, id: name, bytes });
			assert.equal(invalid.captures, 0);
			assert.equal(invalid.outcome['kind'], 'invalid');
			const diagnostics = z.array(recordSchema).parse(invalid.outcome['diagnostics']);
			assert.ok(diagnostics.some((item) => item['code'] === 'unsupported_inputs'));
		};
		await requireInvalid('reversed', [16n, 0n]);
		await requireInvalid('out-of-file', [0n, 9999n]);
		const unsafe = await run('unsafe-number', {
			...binding,
			id: 'unsafe',
			bytes: [0, 9007199254740992],
		});
		assert.equal(unsafe.captures, 0);
		assert.equal(unsafe.outcome['kind'], 'failed');
		const failure = recordSchema.parse(unsafe.outcome['failure']);
		assert.equal(failure['phase'], 'before_dispatch');
		assert.equal(failure['effects'], 'none');
		assert.equal(failure['reason'], 'encoding_rejected');
		const stale = await run('stale', { ...binding, id: 'stale', text_sha256: '0'.repeat(64) });
		assert.equal(stale.captures, 0);
		assert.equal(stale.outcome['kind'], 'completed');
		assert.equal(
			recordSchema.parse(selectionRow(stale.outcome['report'])['outcome'])['kind'],
			'stale',
		);
		const huge = [9007199254740993n, 9007199254740994n];
		const wide = await run('unacquired-wide', {
			...binding,
			id: 'wide',
			file: 'docs/unacquired.html',
			bytes: huge,
		});
		assert.equal(wide.captures, 0);
		assert.ok(
			['completed', 'incomplete'].includes(String(wide.outcome['kind'])),
			'Unacquired source did not return a report',
		);
		const wideRow = selectionRow(wide.outcome['report']);
		assert.deepEqual(recordSchema.parse(wideRow['binding'])['bytes'], huge);
		assert.notEqual(recordSchema.parse(wideRow['outcome'])['kind'], 'selected');
		requireSdk(options.sdk.serialize && options.sdk.deserialize, 'Installed codec unavailable');
		const encoded = recordSchema.parse(
			options.sdk.serialize({ kind: 'report', value: wide.outcome['report'] }),
		);
		assert.equal(encoded['kind'], 'serialized');
		assert.equal(typeof encoded['json'], 'string');
		const json = z.string().parse(encoded['json']);
		const decoded = recordSchema.parse(
			await boundedSdkWait(options.sdk.deserialize(json), 'byte report decode'),
		);
		assert.equal(decoded['kind'], 'decoded');
		assert.ok(
			isDeepStrictEqual(decoded['value'], { kind: 'report', value: wide.outcome['report'] }),
			'Wide binding changed in actual codec roundtrip',
		);
		saveSdkEvidence(path.join(options.evidence, 'sdk-wide-codec.json'), { json, decoded });
		const selectionRoot = path.join(options.evidence, 'sdk-selection-fixture');
		createByteFixture(selectionRoot, backend.endpoint);
		const selectionConfigPath = path.join(selectionRoot, 'sunstack.lint.json');
		const selectionConfig = readObject(selectionConfigPath);
		const evidence = objectValue(selectionConfig['evidence']);
		const paragraph = objectValue(objectValue(evidence['kinds'])['paragraph']);
		paragraph['select'] = {
			question: { type: 'noul', instructions: { check: 'body', task: 'Select this paragraph.' } },
			yes_at_or_above: 0.8,
			no_at_or_below: 0.2,
		};
		writeJson(selectionConfigPath, selectionConfig);
		const selectionLoaded = requireSdkKind(
			await boundedSdkWait(
				options.sdk.load({ directory: selectionRoot }),
				'model-selection SDK load',
			),
			'loaded',
		);
		const selectionRun = async (
			name: string,
			selected: object,
		): Promise<{ outcome: z.infer<typeof recordSchema>; captures: number }> => {
			const before = backend.captures.length;
			const cancellation = new AbortController();
			const pending = options.sdk.run(selectionLoaded['value'], {
				root: selectionRoot,
				scope: { kind: 'all' },
				case_selection: [selected],
				signal: cancellation.signal,
			});
			let outcome: z.infer<typeof recordSchema>;
			try {
				outcome = recordSchema.parse(await boundedSdkWait(pending, name));
			} finally {
				cancellation.abort();
				await boundedSdkWait(Promise.allSettled([pending]), `${name} settlement`);
			}
			const captures = backend.captures.length - before;
			saveSdkEvidence(path.join(options.evidence, `sdk-selection-${name}.json`), {
				outcome,
				captures,
			});
			return { outcome, captures };
		};
		const selectionPositive = await selectionRun('positive', binding);
		assert.equal(selectionPositive.outcome['kind'], 'completed');
		assert.ok(
			selectionPositive.captures >= 2,
			'Positive did not witness selection and rule requests',
		);
		const positiveReport = recordSchema.parse(selectionPositive.outcome['report']);
		const owners = new Set(
			z
				.array(recordSchema)
				.parse(positiveReport['requests'])
				.map((item) => recordSchema.parse(item['owner'])['kind']),
		);
		assert.ok(owners.has('selection') && owners.has('rule'));
		const selectionStale = await selectionRun('stale', {
			...binding,
			id: 'stale-selection',
			text_sha256: '0'.repeat(64),
		});
		assert.equal(
			selectionStale.captures,
			0,
			'Stale direct binding spent selection or rule requests',
		);
		assert.equal(selectionStale.outcome['kind'], 'completed');
		assert.equal(
			recordSchema.parse(selectionRow(selectionStale.outcome['report'])['outcome'])['kind'],
			'stale',
		);
		assert.equal(backend.errors.length, 0);
	} finally {
		if (started) await backend.close();
		saveSdkEvidence(path.join(options.evidence, 'sdk-byte-http.json'), {
			captures: backend.captures,
			errors: backend.errors,
		});
		if (previousKey === undefined) delete process.env['OPENROUTER_API_KEY'];
		else process.env['OPENROUTER_API_KEY'] = previousKey;
		if (previousStore === undefined) delete process.env['SUNSTACK_CREDENTIAL_STORE'];
		else process.env['SUNSTACK_CREDENTIAL_STORE'] = previousStore;
	}
}
