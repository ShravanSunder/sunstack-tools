/** Compile positive and independently invalid callers against the pinned installed package. */
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import filesystem from 'node:fs';
import path from 'node:path';

import { fileDigest, saveSdkEvidence, type SdkTarget } from '../sdk/sdk-target.ts';

const consumerSource = `import { run, type CaseBinding, type Configuration, type RunOptions } from '@sunstack/sdk';
declare const config: Configuration;
const binding: CaseBinding = { id: 'rule', kind: 'rule', rule: 'approval', file: 'docs/a.md', lines: [1n, 1n], text_sha256: '0'.repeat(64) };
const filter: CaseBinding = { id: 'filter', kind: 'filter', filter: 'purpose', folder: 'docs', reads_sha256: '0'.repeat(64) };
const options: RunOptions = { root: '/tmp/example', scope: { kind: 'all' }, case_selection: [binding, filter] };
void run(config, options);
void run(config, { root: '/tmp/example', scope: { kind: 'all' }, case_selection: [] });
// @ts-expect-error selection is a sequence, not a string
const invalidSelection: RunOptions = { root: '/tmp/example', scope: { kind: 'all' }, case_selection: 'rule' };
// @ts-expect-error line widths are bigint, not number
const invalidLines: CaseBinding = { id: 'x', kind: 'rule', rule: 'approval', file: 'docs/a.md', lines: [1, 1], text_sha256: '0' };
// @ts-expect-error execution bindings do not contain expected labels
const leakedLabel: CaseBinding = { id: 'x', kind: 'rule', rule: 'approval', file: 'docs/a.md', lines: [1n, 1n], text_sha256: '0', expected: 'pass' };
// @ts-expect-error filter binding requires its read digest
const missingDigest: CaseBinding = { id: 'x', kind: 'filter', filter: 'purpose', folder: 'docs' };
void invalidSelection; void invalidLines; void leakedLabel; void missingDigest;
`;

export function observeInstalledSelectionCompiler(options: {
	readonly target: SdkTarget;
	readonly evidence: string;
	readonly compiler: string;
}): void {
	assert.ok(path.isAbsolute(options.compiler), 'Compiler must have an absolute path');
	const caller = path.join(
		options.target.consumerRoot,
		`case-dispatch-${path.basename(options.evidence)}.ts`,
	);
	filesystem.writeFileSync(caller, consumerSource, { flag: 'wx' });
	const result = spawnSync(
		process.execPath,
		[
			options.compiler,
			'--ignoreConfig',
			'--noEmit',
			'--strict',
			'--target',
			'ES2023',
			'--module',
			'NodeNext',
			'--moduleResolution',
			'NodeNext',
			caller,
		],
		{ cwd: options.target.consumerRoot, encoding: 'utf8', timeout: 30000 },
	);
	saveSdkEvidence(path.join(options.evidence, 'case-dispatch-compiler.json'), {
		compiler: options.compiler,
		compilerSha256: fileDigest(options.compiler),
		node: process.version,
		caller,
		source: consumerSource,
		exit: result.status,
		signal: result.signal,
		stdout: result.stdout,
		stderr: result.stderr,
		error: result.error?.message,
	});
	assert.equal(result.error, undefined, 'Compiler did not run');
	assert.equal(result.status, 0, 'Installed positive or negative selection caller failed');
}
