/** Observe released CLI byte binding with an owned synthetic loopback, without an SDK. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';

import { objectValue, writeJson } from '../rulebook/rulebook-fixture-support.ts';
import { declaredTarget, responseBody, RulebookProcess } from '../rulebook/rulebook-process.ts';
import { observeByteCliJourney } from './byte-cli-journey.ts';

async function main(): Promise<void> {
	const { values } = parseArgs({
		strict: true,
		options: {
			source: { type: 'string' },
			commit: { type: 'string' },
			binary: { type: 'string' },
			sha256: { type: 'string' },
			evidence: { type: 'string' },
			help: { type: 'boolean' },
		},
	});
	if (values.help) {
		process.stdout.write(
			'CLI-only exact byte observation: --source <absolute clean source> --commit <full source SHA> --binary <absolute CLI> --sha256 <binary SHA256> --evidence <new external directory>. Runs real adjacent prepare, scored selection and saved readback through controlled loopback; no SDK. Requires an explicitly declared target and grant.\n',
		);
		return;
	}
	assert.ok(values.source && path.isAbsolute(values.source));
	assert.ok(values.binary && path.isAbsolute(values.binary));
	assert.ok(values.commit && /^[a-f0-9]{40}$/u.test(values.commit));
	assert.ok(values.sha256 && /^[a-f0-9]{64}$/u.test(values.sha256));
	assert.ok(values.evidence && path.isAbsolute(values.evidence));
	const relative = path.relative(values.source, values.evidence);
	assert.ok(
		relative === '..' || relative.startsWith(`..${path.sep}`) || path.isAbsolute(relative),
		'Evidence must be outside source',
	);
	const target = declaredTarget({
		source: values.source,
		commit: values.commit,
		binary: values.binary,
		digest: values.sha256,
	});
	filesystem.mkdirSync(values.evidence, { recursive: false });
	writeJson(path.join(values.evidence, 'target.json'), target);
	const observer = new RulebookProcess(target, path.join(values.evidence, 'cli'));
	let started = false;
	try {
		await observer.start();
		started = true;
		observer.respond = async ({ body }) => {
			const text = objectValue(body['state'])['body'];
			assert.ok(text === 'café yes' || text === 'no', 'Unexpected fixture request');
			return { status: 200, body: responseBody(body, text === 'café yes' ? 0.91 : 0.09) };
		};
		await observeByteCliJourney(observer, values.evidence);
		writeJson(path.join(values.evidence, 'result.json'), {
			status: 'observed',
			scope: 'adjacent CLI prepare, score, individual selection, request parity and readback',
			gaps: [
				'native child/read-attempt and git2 source joins',
				'SDK, other platforms, hosted quality and released channel identity',
			],
		});
	} catch (error) {
		writeJson(path.join(values.evidence, 'failure.json'), {
			message: error instanceof Error ? error.message : String(error),
		});
		throw error;
	} finally {
		if (started) await observer.close();
	}
}
await main();
