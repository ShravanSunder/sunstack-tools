/** W6: one-item containment lists preserve 0.3.0 pieces/outcomes/counts; a planted piece change must fail. */
import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import filesystem from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import { parseArgs } from 'node:util';

import { z } from 'zod';

import {
	arrayValue,
	configName,
	configureEndpoint,
	equal,
	errorMessage,
	numberValue,
	objectValue,
	parseObject,
	readObject,
	requireCondition,
	runSequence,
	writeJson,
	type JsonObject,
} from './rulebook-fixture-support.ts';
import { binaryDigest, confirmTarget, declaredTarget } from './rulebook-process.ts';
import { copySelectionFixture } from './selection-fixture.ts';
import { scriptedSelectionReply } from './selection-reply.ts';

/** Compare every byte of the three W6 projections; do not normalize or drop piece fields. */
export function requireWithinEquivalence(options: {
	reference: JsonObject;
	current: JsonObject;
}): void {
	for (const field of ['pieces', 'selections', 'file_counts']) {
		requireCondition(
			Array.isArray(options.reference[field]) && Array.isArray(options.current[field]),
			'W6 requires complete ' + field,
		);
		equal(
			JSON.stringify(options.current[field]),
			JSON.stringify(options.reference[field]),
			'W6 differs in ' + field,
		);
	}
}

const argumentsSchema = z.object({
	baseline: z.string(),
	'baseline-sha256': z.string().regex(/^[0-9a-f]{64}$/),
	binary: z.string(),
	sha256: z.string().regex(/^[0-9a-f]{64}$/),
	'source-root': z.string(),
	commit: z.string().regex(/^[0-9a-f]{40}$/),
	evidence: z.string(),
});
type Arguments = z.infer<typeof argumentsSchema>;
interface Listing {
	report: JsonObject;
	requests: Buffer[];
}

export async function main(): Promise<void> {
	const parsed = parseArgs({
		options: {
			baseline: { type: 'string' },
			'baseline-sha256': { type: 'string' },
			binary: { type: 'string' },
			sha256: { type: 'string' },
			'source-root': { type: 'string' },
			commit: { type: 'string' },
			evidence: { type: 'string' },
			help: { type: 'boolean' },
		},
	});
	if (parsed.values.help) {
		process.stdout.write(
			'W6 one-item equivalence: --baseline V0_3_0_BINARY --baseline-sha256 HASH --binary DECLARED_BINARY --sha256 HASH --source-root CHECKOUT --commit FULL_SHA --evidence NEW_DIR. Synthetic loopback only; same fixture path, old scalar vs new one-item list; selected/no/uncertain plus planted-piece rejection.\n',
		);
		return;
	}
	const options = argumentsSchema.parse(parsed.values);
	await observeEquivalence(options);
}

async function observeEquivalence(options: Arguments): Promise<void> {
	const target = declaredTarget({
		binary: options.binary,
		source: options['source-root'],
		commit: options.commit,
		digest: options.sha256,
	});
	equal(binaryDigest(options.baseline), options['baseline-sha256'], 'Baseline binary bytes differ');
	const version = spawnSync(options.baseline, ['--version'], { encoding: 'utf8', timeout: 10000 });
	equal(version.status, 0, 'Baseline version failed');
	equal(version.stdout.trim(), 'sunstack 0.3.0', 'W6 baseline must be v0.3.0');
	const evidence = path.resolve(options.evidence);
	requireCondition(!filesystem.existsSync(evidence), 'W6 evidence directory must be fresh');
	filesystem.mkdirSync(evidence);
	let probability = 0.93;
	let captures: Buffer[] = [];
	let errors: string[] = [];
	const server = http.createServer((request, response) => {
		void (async () => {
			const chunks: Buffer[] = [];
			for await (const chunk of request)
				chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk));
			const raw = Buffer.concat(chunks);
			const body = parseObject(raw.toString('utf8'));
			captures.push(raw);
			response.writeHead(200, { 'Content-Type': 'application/json' });
			response.end(
				JSON.stringify(scriptedSelectionReply({ probability: () => probability })(body)),
			);
		})().catch((error: unknown) => {
			errors.push(errorMessage(error));
			response.writeHead(500);
			response.end('{}');
		});
	});
	await new Promise<void>((resolve, reject) => {
		server.once('error', reject);
		server.listen(0, '127.0.0.1', resolve);
	});
	const address = server.address();
	requireCondition(address && typeof address === 'object', 'W6 loopback address missing');
	try {
		const root = copySelectionFixture({
			destination: path.join(evidence, 'fixture'),
			sourceRoot: options['source-root'],
		});
		configureEndpoint(root, `http://127.0.0.1:${address.port}/systemone`);
		const listConfig = readObject(path.join(root, configName));
		const scalarConfig = structuredClone(listConfig);
		let converted = 0;
		for (const value of Object.values(
			objectValue(objectValue(scalarConfig['evidence'])['kinds']),
		)) {
			const kind = objectValue(value);
			if (kind['within'] === undefined) continue;
			const containers = arrayValue(kind['within']);
			requireCondition(
				containers.length === 1 && typeof containers[0] === 'string',
				'W6 source must use one-item lists, not a compatibility alias',
			);
			kind['within'] = containers[0];
			converted++;
		}
		requireCondition(converted > 0, 'W6 fixture has no containment to compare');
		const confirm = (): void => {
			confirmTarget(target);
			equal(
				binaryDigest(options.baseline),
				options['baseline-sha256'],
				'Baseline changed during comparison',
			);
		};
		async function run(input: {
			name: string;
			binary: string;
			config: JsonObject;
			exit: number;
		}): Promise<Listing> {
			confirm();
			captures = [];
			errors = [];
			writeJson(path.join(root, configName), input.config);
			const directory = path.join(evidence, input.name);
			filesystem.mkdirSync(directory);
			writeJson(path.join(directory, 'config.json'), input.config);
			const home = path.join(directory, 'home');
			filesystem.mkdirSync(home);
			const args = ['lint', '--config', configName, '--evidence', '--all'];
			const child = spawn(input.binary, args, {
				cwd: root,
				env: {
					PATH: '/usr/bin:/bin',
					HOME: home,
					LANG: 'C.UTF-8',
					SUNSTACK_CREDENTIAL_STORE: 'off',
					OPENROUTER_API_KEY: 'SYNTHETIC_WITHIN_EQUIVALENCE_ONLY',
					GIT_CONFIG_NOSYSTEM: '1',
					GIT_CONFIG_GLOBAL: '/dev/null',
				},
				detached: true,
				stdio: ['pipe', 'pipe', 'pipe'],
			});
			const stdout: Buffer[] = [];
			const stderr: Buffer[] = [];
			child.stdout.on('data', (chunk: Buffer) => stdout.push(chunk));
			child.stderr.on('data', (chunk: Buffer) => stderr.push(chunk));
			let timedOut = false;
			const timer = setTimeout(() => {
				timedOut = true;
				if (child.pid) process.kill(-child.pid, 'SIGKILL');
			}, 30000);
			let exit: number | null;
			try {
				exit = await new Promise<number | null>((resolve, reject) => {
					child.once('error', reject);
					child.once('close', resolve);
				});
			} finally {
				clearTimeout(timer);
				child.stdin.destroy();
			}
			const raw = Buffer.concat(stdout);
			filesystem.writeFileSync(path.join(directory, 'stdout.json'), raw);
			filesystem.writeFileSync(path.join(directory, 'stderr.log'), Buffer.concat(stderr));
			captures.forEach((body, index) =>
				filesystem.writeFileSync(path.join(directory, `request-${index + 1}.raw.json`), body),
			);
			writeJson(path.join(directory, 'process.json'), {
				command: [input.binary, ...args],
				cwd: root,
				source_guard_commit: target.commit,
				binary_sha256: binaryDigest(input.binary),
				exit,
				timed_out: timedOut,
				requests: captures.length,
				harness_errors: errors,
				credential: 'synthetic',
				stdin_held_open: true,
			});
			equal(timedOut, false, 'W6 process timed out');
			equal(errors, [], 'W6 harness failed');
			equal(exit, input.exit, 'W6 child exit differs');
			confirm();
			return { report: parseObject(raw.toString('utf8')), requests: [...captures] };
		}
		const results: JsonObject[] = [];
		await runSequence(
			[
				{ name: 'selected', yes: 0.93, exit: 0 },
				{ name: 'not-selected', yes: 0.12, exit: 0 },
				{ name: 'uncertain', yes: 0.5, exit: 5 },
			],
			async (control) => {
				probability = control.yes;
				const reference = await run({
					name: control.name + '-v0.3.0',
					binary: options.baseline,
					config: scalarConfig,
					exit: control.exit,
				});
				const current = await run({
					name: control.name + '-new',
					binary: target.binary,
					config: listConfig,
					exit: control.exit,
				});
				requireWithinEquivalence({ reference: reference.report, current: current.report });
				equal(
					current.requests,
					reference.requests,
					'One-item cutover changed exact selection request bytes/order',
				);
				requireCondition(
					current.requests.length > 0 && arrayValue(current.report['selections']).length > 0,
					'W6 failed to reach selection',
				);
				if (control.name === 'selected') {
					const planted = structuredClone(current.report);
					const piece = objectValue(arrayValue(planted['pieces'])[0]);
					const where = objectValue(piece['where']);
					where['first_line'] = numberValue(where['first_line']) + 1;
					writeJson(path.join(evidence, 'planted-piece-report.json'), planted);
					assert.throws(
						() => requireWithinEquivalence({ reference: reference.report, current: planted }),
						/W6 differs in pieces/,
					);
					writeJson(path.join(evidence, 'planted-failure.json'), {
						changed: 'pieces[0].where.first_line + 1',
						rejected: true,
					});
				}
				results.push({
					case: control.name,
					exit: control.exit,
					pieces: arrayValue(current.report['pieces']).length,
					selections: arrayValue(current.report['selections']).length,
					requests: current.requests.length,
					exact_w6_fields: true,
					exact_request_bytes: true,
				});
			},
		);
		writeJson(path.join(evidence, 'pass.json'), {
			source_commit: target.commit,
			binary_sha256: target.digest,
			baseline_version: version.stdout.trim(),
			baseline_sha256: options['baseline-sha256'],
			fields: ['pieces', 'selections', 'file_counts'],
			normalization:
				'none; same fixture path/endpoint, only declared scalar vs one-item config change',
			cases: results,
			planted_piece_rejected: true,
		});
		process.stdout.write(
			'W6 one-item containment equivalence and planted-piece rejection passed\n',
		);
	} finally {
		await new Promise<void>((resolve, reject) => {
			server.close((error) => (error ? reject(error) : resolve()));
			server.closeAllConnections();
		});
	}
}

if (import.meta.main)
	try {
		await main();
	} catch (error) {
		process.stderr.write(`Within equivalence failed: ${errorMessage(error)}\n`);
		process.exitCode = 2;
	}
