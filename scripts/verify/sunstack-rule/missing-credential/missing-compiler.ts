/** The real installed compiler must accept narrowing and reject independently bad shapes. */
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import filesystem from 'node:fs';
import path from 'node:path';

import { fileDigest, saveSdkEvidence, type SdkTarget } from '../sdk/sdk-target.ts';

const positive = `import type { RulePieceResult, RulePieceEnding, UnavailableCounts } from '@sunstack/sdk';
declare const result: RulePieceResult;
if (result.outcome === 'unavailable') { const reason: string = result.reason.kind; console.log(reason); }
const ending: RulePieceEnding = { outcome: 'unavailable', reason: { kind: 'credential_unavailable' } };
declare const counts: UnavailableCounts;
const fullWidth: bigint = counts.rule_pieces;
void ending; void fullWidth;
`;
const negative = `import type { RulePieceEnding, UnavailableCounts, WorkUnavailableReason } from '@sunstack/sdk';
// @ts-expect-error mandatory reason cannot be omitted
const omitted: RulePieceEnding = { outcome: 'unavailable' };
// @ts-expect-error filter reason requires filter and folder
const malformed: WorkUnavailableReason = { kind: 'filter_unavailable' };
declare const counts: UnavailableCounts;
// @ts-expect-error full-width counts are bigint
const rounded: number = counts.rule_pieces;
// @ts-expect-error public generated counts are readonly
counts.rule_pieces = 0n;
// @ts-expect-error new counts cannot be constructed from Number
const wrong: UnavailableCounts = { rule_pieces: 0, filter_folders: 0n, selection_candidates: 0n, evidence_scopes: 0n };
void omitted; void malformed; void rounded; void wrong;
`;
export function compileMissingCredential(options: {
	readonly target: SdkTarget;
	readonly compiler: string;
	readonly evidence: string;
}): void {
	assert.ok(path.isAbsolute(options.compiler));
	for (const [label, source] of [
		['positive', positive],
		['negative', negative],
	] as const) {
		const caller = path.join(
			options.target.consumerRoot,
			`missing-${path.basename(options.evidence)}-${label}.ts`,
		);
		filesystem.writeFileSync(caller, source, { flag: 'wx' });
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
		saveSdkEvidence(path.join(options.evidence, `compiler-${label}.json`), {
			compiler: options.compiler,
			compilerSha256: fileDigest(options.compiler),
			node: process.version,
			caller,
			source,
			exit: result.status,
			signal: result.signal,
			stdout: result.stdout,
			stderr: result.stderr,
			error: result.error?.message,
		});
		assert.equal(result.error, undefined, 'Compiler failed to run');
		assert.equal(result.status, 0, `${label}: installed generated contract failed`);
		assert.equal(result.stderr, '');
	}
}
