/** Bounded FIFO read trap for an out-of-scope direct binding; baseline line contract only. */
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import filesystem from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';

import {
	arrayValue,
	objectValue,
	parseObject,
	readObject,
	writeJson,
} from '../rulebook/rulebook-fixture-support.ts';
import { declaredTarget, RulebookProcess } from '../rulebook/rulebook-process.ts';
import { requireSdk, saveSdkEvidence } from '../sdk/sdk-target.ts';
import { createByteFixture } from './byte-fixture.ts';

async function main(): Promise<void> {
	const { values } = parseArgs({
		strict: true,
		options: {
			binary: { type: 'string' },
			source: { type: 'string' },
			commit: { type: 'string' },
			sha256: { type: 'string' },
			evidence: { type: 'string' },
			help: { type: 'boolean' },
		},
	});
	if (values.help) {
		process.stdout.write(
			'Baseline no-read capability probe: --binary <absolute CLI> --source <absolute source> --commit <full SHA> --sha256 <binary hash> --evidence <new absolute directory>. A no-writer FIFO must block a positive read control. This exercises baseline line bindings only, not E5.\n',
		);
		return;
	}
	requireSdk(
		values.binary &&
			path.isAbsolute(values.binary) &&
			values.source &&
			path.isAbsolute(values.source),
		'Absolute binary and source required',
	);
	requireSdk(
		values.commit && values.sha256 && values.evidence && path.isAbsolute(values.evidence),
		'Commit, digest and new evidence required',
	);
	const target = declaredTarget({
		binary: values.binary,
		source: values.source,
		commit: values.commit,
		digest: values.sha256,
	});
	filesystem.mkdirSync(values.evidence, { recursive: false });
	const observer = new RulebookProcess(target, path.join(values.evidence, 'cli'));
	let started = false;
	try {
		await observer.start();
		started = true;
		const root = path.join(values.evidence, 'fixture');
		createByteFixture(root, observer.endpoint);
		const configPath = path.join(root, 'sunstack.lint.json');
		const config = readObject(configPath);
		config['check'] = { files: ['docs/pieces.html'] };
		writeJson(configPath, config);
		const fifoPath = path.join(root, 'docs', 'unacquired.html');
		const made = spawnSync('/usr/bin/mkfifo', [fifoPath], { encoding: 'utf8' });
		assert.equal(made.status, 0, `mkfifo failed: ${made.stderr}`);
		const control = spawnSync(
			process.execPath,
			[
				'-e',
				"process.stdout.write('ready\\n'); require('node:fs').readFileSync(process.argv[1]);",
				fifoPath,
			],
			{ encoding: 'utf8', timeout: 1000 },
		);
		saveSdkEvidence(path.join(values.evidence, 'fifo-control.json'), {
			stdout: control.stdout,
			status: control.status,
			signal: control.signal,
			error: control.error?.message,
		});
		assert.equal(control.stdout, 'ready\n', 'Positive control did not reach read');
		assert.ok(control.error && 'code' in control.error, 'FIFO control did not time out');
		assert.equal(control.error.code, 'ETIMEDOUT', 'FIFO did not block the intentional reader');
		const selectionPath = path.join(values.evidence, 'unacquired-selection.json');
		writeJson(selectionPath, [
			{
				id: 'unacquired',
				kind: 'rule',
				rule: 'approval',
				file: 'docs/unacquired.html',
				lines: [1, 1],
				text_sha256: '0'.repeat(64),
			},
		]);
		const run = await observer.run({
			root,
			name: 'unacquired',
			arguments: [
				'lint',
				'--config',
				'sunstack.lint.json',
				'--all',
				'--format',
				'json',
				'--case-selection',
				selectionPath,
			],
			expectedExit: 0,
			timeoutMilliseconds: 5000,
		});
		assert.equal(run.captures.length, 0);
		const report = parseObject(run.stdout.toString('utf8'));
		const rows = arrayValue(objectValue(report['case_selection'])['bindings']).map(objectValue);
		assert.equal(rows.length, 1);
		assert.equal(objectValue(rows[0]?.['binding'])['id'], 'unacquired');
		saveSdkEvidence(path.join(values.evidence, 'unacquired-disposition.json'), rows[0]);
	} finally {
		if (started) await observer.close();
	}
}
await main();
