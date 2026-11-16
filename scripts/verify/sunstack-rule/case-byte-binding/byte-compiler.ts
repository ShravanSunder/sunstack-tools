/** Compile the public caller against the fresh installed SDK, never workspace declarations. */
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import filesystem from 'node:fs';
import path from 'node:path';

import { fileDigest, saveSdkEvidence, type SdkTarget } from '../sdk/sdk-target.ts';

const consumerSource = `import { type CaseBinding, type CaseExecutionOwner } from '@sunstack/sdk';
const selected: CaseBinding = { id: 'first', kind: 'rule', rule: 'approval', file: 'docs/pieces.html', lines: [1n, 1n], bytes: [0n, 16n], text_sha256: 'x' };
const legacy: CaseBinding = { id: 'legacy', kind: 'rule', rule: 'approval', file: 'docs/pieces.html', lines: [1n, 1n], text_sha256: 'x' };
const owner: CaseExecutionOwner = { kind: 'rule', rule: 'approval', file: 'docs/pieces.html', lines: [1n, 1n], bytes: [0n, 16n] };
// @ts-expect-error offsets must be a two-element tuple
const badLength: CaseBinding = { id: 'x', kind: 'rule', rule: 'approval', file: 'x', lines: [1n, 1n], bytes: [0n], text_sha256: 'x' };
// @ts-expect-error numbers are not full-width offsets
const badNumbers: CaseBinding = { id: 'x', kind: 'rule', rule: 'approval', file: 'x', lines: [1n, 1n], bytes: [0, 16], text_sha256: 'x' };
// @ts-expect-error strings are not offsets
const badStrings: CaseBinding = { id: 'x', kind: 'rule', rule: 'approval', file: 'x', lines: [1n, 1n], bytes: ['0', '16'], text_sha256: 'x' };
void selected; void legacy; void owner; void badLength; void badNumbers; void badStrings;
`;

export function observeInstalledByteCompiler(options: {
	readonly target: SdkTarget;
	readonly evidence: string;
	readonly compiler: string;
}): void {
	assert.ok(path.isAbsolute(options.compiler), 'Compiler must have an absolute path');
	const caller = path.join(
		options.target.consumerRoot,
		`byte-cases-${path.basename(options.evidence)}.ts`,
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
	saveSdkEvidence(path.join(options.evidence, 'byte-compiler.json'), {
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
	assert.equal(result.status, 0, 'Installed byte-binding caller failed');
}
