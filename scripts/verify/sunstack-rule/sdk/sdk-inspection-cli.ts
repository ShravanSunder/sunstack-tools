/** Compare real CLI evidence and rule execution with an installed SDK inspection. */
import { spawnSync } from 'node:child_process';
import filesystem from 'node:fs';
import path from 'node:path';
import { isDeepStrictEqual } from 'node:util';

import { z } from 'zod';

import { requireSdkFileCountParity } from './sdk-counter-parity.ts';
import { requireSdk, saveSdkEvidence, type SdkTarget } from './sdk-target.ts';

const recordSchema = z.record(z.string(), z.unknown());

export function observeSdkInspectionCli(options: {
	readonly target: SdkTarget;
	readonly evidence: string;
	readonly root: string;
	readonly sdkReport: Record<string, unknown>;
}): { readonly observed: string[]; readonly incomplete: Record<string, unknown> } {
	const { target, evidence, root, sdkReport } = options;
	const cliCandidate = target.cli;
	requireSdk(cliCandidate, 'CLI parity requires a pinned CLI');
	const definitions = path.join(evidence, 'inspection-cli-definitions');
	const rules = path.join(definitions, 'evidence', 'rules');
	filesystem.mkdirSync(rules, { recursive: true });
	filesystem.writeFileSync(
		path.join(definitions, 'evidence', 'sgconfig.yml'),
		'ruleDirs: [rules]\n',
	);
	filesystem.writeFileSync(
		path.join(rules, 'typescript.yml'),
		"id: typescript_call\nlanguage: TypeScript\nrule: {pattern: 'console.log($A)'}\n",
	);
	filesystem.writeFileSync(
		path.join(rules, 'rust.yml'),
		"id: rust_call\nlanguage: Rust\nrule: {pattern: 'send($A)'}\n",
	);
	filesystem.writeFileSync(
		path.join(definitions, 'rulebook.md'),
		'The call must contain a sentinel. <!-- K001 -->\n',
	);
	filesystem.writeFileSync(
		path.join(definitions, 'rules.json'),
		JSON.stringify({
			K001: {
				name: 'sentinel',
				evidence: 'typescript_call',
				severity: 'error',
				decision: {
					start: 'check',
					steps: {
						check: {
							mechanical_check: { tool: 'contains', field: 'body', text: 'ABSENT_SENTINEL' },
							yes: 'pass',
							['no']: { finding: 'Sentinel absent.' },
						},
					},
				},
			},
		}),
	);
	const config = {
		marker_prefix: 'K',
		check: { files: ['src/**'], ignore: ['src/ignored.ts'] },
		decision_model: { model: 'jev-latest' },
		rulebook: [{ markdown: 'rulebook.md', rules: 'rules.json' }],
		evidence: {
			ast_grep: 'evidence/sgconfig.yml',
			kinds: {
				typescript_call: { rule: 'typescript_call', fields: { body: { from: 'remainder' } } },
				rust_call: { rule: 'rust_call', fields: { body: { from: 'remainder' } } },
			},
		},
	};
	const configPath = path.join(definitions, 'sunstack.lint.json');
	filesystem.writeFileSync(configPath, JSON.stringify(config));
	const env: NodeJS.ProcessEnv = { ...process.env, SUNSTACK_CREDENTIAL_STORE: 'off' };
	delete env['OPENROUTER_API_KEY'];
	const execute = (name: string, extra: readonly string[]): ReturnType<typeof spawnSync> => {
		const result = spawnSync(
			cliCandidate.binary,
			['lint', '--config', configPath, '--root', root, '--all', '--format', 'json', ...extra],
			{ cwd: root, env, encoding: 'utf8', timeout: 30000 },
		);
		saveSdkEvidence(path.join(evidence, `inspection-cli-${name}.json`), {
			status: result.status,
			signal: result.signal,
			stdout: result.stdout,
			stderr: result.stderr,
			error: result.error?.message,
		});
		return result;
	};
	const listing = execute('evidence', ['--evidence']);
	requireSdk(
		!listing.error && listing.status === 0 && typeof listing.stdout === 'string',
		'CLI evidence did not finish cleanly',
	);
	const cli = recordSchema.parse(JSON.parse(listing.stdout));
	requireSdk(recordSchema.parse(cli['status'])['kind'] === 'clean', 'CLI evidence status differs');
	requireSdk(
		isDeepStrictEqual(cli['exclusions'], sdkReport['exclusions']),
		'CLI/SDK exclusions differ',
	);
	const cliLimits = recordSchema.parse(cli['limits']);
	const sdkLimits = recordSchema.parse(sdkReport['limits']);
	requireSdk(
		isDeepStrictEqual(Object.keys(cliLimits), Object.keys(sdkLimits)),
		'CLI/SDK limit fields differ',
	);
	for (const [key, value] of Object.entries(cliLimits)) {
		const sdkValue = sdkLimits[key];
		requireSdk(
			typeof sdkValue === 'bigint'
				? typeof value === 'number' && Number.isSafeInteger(value) && BigInt(value) === sdkValue
				: Object.is(value, sdkValue),
			`CLI/SDK limit ${key} differs`,
		);
	}
	for (const key of ['diagnostics', 'incomplete', 'selections']) {
		requireSdk(
			z.array(z.unknown()).parse(cli[key]).length === 0 &&
				z.array(z.unknown()).parse(sdkReport[key]).length === 0,
			`CLI/SDK clean ${key} is not empty`,
		);
	}
	requireSdk(
		recordSchema.parse(cli['usage'])['model_requests'] === 0 &&
			recordSchema.parse(sdkReport['usage'])['model_requests'] === 0,
		'Static listing recorded model requests',
	);
	requireSdkFileCountParity(listing.stdout, sdkReport['file_counts']);
	const cliPieces = z.array(recordSchema).parse(cli['pieces']);
	const sdkPieces = z.array(recordSchema).parse(sdkReport['pieces']);
	requireSdk(cliPieces.length === sdkPieces.length, 'CLI/SDK piece count differs');
	for (const piece of sdkPieces) {
		const other = cliPieces.find((candidate) => candidate['kind'] === piece['kind']);
		requireSdk(other && other['text'] === piece['text'], 'CLI/SDK piece text differs');
		const sdkLocation = recordSchema.parse(piece['where']);
		const cliLocation = recordSchema.parse(other['where']);
		const sdkSpan = recordSchema.parse(sdkLocation['span']);
		const cliSpan = recordSchema.parse(cliLocation['span']);
		requireSdk(cliLocation['file'] === sdkLocation['file'], 'CLI/SDK source file differs');
		for (const field of [
			'byte_start',
			'byte_end',
			'start_line',
			'start_column',
			'end_line',
			'end_column',
		])
			requireSdk(
				typeof cliSpan[field] === 'number' &&
					typeof sdkSpan[field] === 'bigint' &&
					BigInt(cliSpan[field]) === sdkSpan[field],
				`CLI/SDK ${field} differs`,
			);
		requireSdk(
			recordSchema.parse(cliSpan['source'])['content_identity'] ===
				recordSchema.parse(sdkSpan['source'])['content_identity'],
			'CLI/SDK source identity differs',
		);
	}
	const evaluated = execute('rule-witness', []);
	requireSdk(
		!evaluated.error && evaluated.status === 1 && typeof evaluated.stdout === 'string',
		'The same source did not produce the expected rule finding',
	);
	const report = recordSchema.parse(JSON.parse(evaluated.stdout));
	requireSdk(
		recordSchema.parse(report['status'])['kind'] === 'findings' &&
			z.array(z.unknown()).parse(report['findings']).length > 0,
		'Rule witness did not report a finding',
	);
	const emptyConfig = path.join(definitions, 'empty-rulebook.json');
	filesystem.writeFileSync(emptyConfig, JSON.stringify({ ...config, rulebook: [] }));
	const admission = spawnSync(cliCandidate.binary, ['lint', '--config', emptyConfig, '--check'], {
		cwd: root,
		env,
		encoding: 'utf8',
		timeout: 30000,
	});
	saveSdkEvidence(path.join(evidence, 'inspection-cli-empty-admission.json'), {
		status: admission.status,
		signal: admission.signal,
		stdout: admission.stdout,
		stderr: admission.stderr,
		error: admission.error?.message,
	});
	requireSdk(!admission.error && admission.status === 2, 'CLI admitted an empty rulebook');
	requireSdk(
		`${admission.stdout}\n${admission.stderr}`.toLowerCase().includes('rulebook'),
		'Empty-rulebook refusal did not identify its cause',
	);
	const limitedConfig = path.join(definitions, 'limited.json');
	filesystem.writeFileSync(
		limitedConfig,
		JSON.stringify({ ...config, limits: { max_file_bytes: 1 } }),
	);
	const limited = spawnSync(
		cliCandidate.binary,
		['lint', '--config', limitedConfig, '--root', root, '--all', '--format', 'json', '--evidence'],
		{ cwd: root, env, encoding: 'utf8', timeout: 30000 },
	);
	saveSdkEvidence(path.join(evidence, 'inspection-cli-limited.json'), {
		status: limited.status,
		signal: limited.signal,
		stdout: limited.stdout,
		stderr: limited.stderr,
		error: limited.error?.message,
	});
	requireSdk(
		!limited.error && limited.status === 3,
		'CLI limited inspection did not exit incomplete',
	);
	const incomplete = recordSchema.parse(JSON.parse(limited.stdout));
	requireSdk(
		recordSchema.parse(incomplete['status'])['kind'] === 'incomplete',
		'CLI limited listing lost incomplete status',
	);
	requireSdk(
		recordSchema.parse(incomplete['limits'])['max_file_bytes'] === 1,
		'CLI limited listing lost byte limit',
	);
	return {
		observed: [
			'CLI/SDK evidence text, source identity, spans, limits, exclusions and file counts',
			'clean listing on source with an actual mechanical finding',
			'CLI retains empty-rulebook admission refusal',
		],
		incomplete,
	};
}
