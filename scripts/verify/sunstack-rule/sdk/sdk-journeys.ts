/** Installed load/check/run observations. Open joins remain gaps, never inferred passes. */
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import path from 'node:path';
import { isDeepStrictEqual } from 'node:util';

import { z } from 'zod';

import { SdkControlledBackend, boundedSdkWait } from './sdk-controlled-backend.ts';
import {
	createCheckedSdkInputs,
	createSdkFileDefinitions,
	sdkInlineConfiguration,
} from './sdk-fixtures.ts';
import { requireSdk, saveSdkEvidence, type SdkExports, type SdkTarget } from './sdk-target.ts';

export const sdkRecordSchema = z.record(z.string(), z.unknown());
const completedStatusSchema = z.object({
	kind: z.enum(['clean', 'nothing_selected', 'findings', 'uncertain']),
});
const sourcesSchema = z
	.array(
		z
			.object({
				origin: z.object({ kind: z.enum(['file', 'inline', 'edited']) }).passthrough(),
				content: z.string(),
				sha256: z.string().regex(/^[a-f0-9]{64}$/),
				representation: z.enum(['original_text', 'normalized_value']),
				acquisition: z.enum(['read', 'supplied_snapshot', 'inline']),
			})
			.passthrough(),
	)
	.min(1);
export interface JourneyContext {
	readonly sdk: SdkExports;
	readonly target: SdkTarget;
	readonly evidence: string;
	readonly backend: SdkControlledBackend;
}
export interface JourneyResult {
	readonly observed: string[];
	readonly gaps: string[];
}
function record(value: unknown): z.infer<typeof sdkRecordSchema> {
	return sdkRecordSchema.parse(value);
}
export function requireSdkKind(value: unknown, kind: string): z.infer<typeof sdkRecordSchema> {
	const outcome = record(value);
	requireSdk(outcome['kind'] === kind, `Expected ${kind}, received ${String(outcome['kind'])}`);
	return outcome;
}
export function requireCompletedSdkReport(value: unknown): z.infer<typeof sdkRecordSchema> {
	const report = record(requireSdkKind(value, 'completed')['report']);
	requireSdk(
		completedStatusSchema.safeParse(report['status']).success,
		'Completed outcome requires a tagged completed status',
	);
	return report;
}
export function requireSdkSources(value: unknown, kind: 'file' | 'inline' | 'edited'): void {
	const sources = sourcesSchema.parse(value);
	requireSdk(
		sources.some((source) => source.origin.kind === kind),
		`No ${kind} provenance`,
	);
	for (const source of sources)
		requireSdk(
			createHash('sha256').update(source.content).digest('hex') === source.sha256,
			'Returned source content/digest disagree',
		);
}
function requireRequestSentinel(
	backend: SdkControlledBackend,
	since: number,
	present: string,
	absent: string,
): void {
	const requests = backend.captures.slice(since);
	requireSdk(requests.length > 0, 'Inert run: no actual inference requests');
	const text = JSON.stringify(requests);
	requireSdk(
		text.includes(present) && !text.includes(absent),
		`Requests did not preserve ${present} versus ${absent}`,
	);
}
async function runConfigured(
	context: JourneyContext,
	input: unknown,
	root: string,
): Promise<z.infer<typeof sdkRecordSchema>> {
	const cancellation = new AbortController();
	const pending = context.sdk.run(input, {
		root,
		scope: { kind: 'all' },
		signal: cancellation.signal,
	});
	try {
		return requireCompletedSdkReport(await boundedSdkWait(pending, 'installed SDK run'));
	} finally {
		cancellation.abort();
		await boundedSdkWait(Promise.allSettled([pending]), 'owned installed run settlement');
	}
}

export async function observeSdkSnapshot(context: JourneyContext): Promise<JourneyResult> {
	const root = path.join(context.evidence, 'snapshot-root');
	createCheckedSdkInputs(root);
	createSdkFileDefinitions(root, context.backend.endpoint);
	const loaded = requireSdkKind(
		await context.sdk.load({ directory: path.join(root, 'skills', 'portable') }),
		'loaded',
	);
	const held = record(loaded['value']);
	requireSdkSources(held['sources'], 'file');
	requireSdk(
		Object.isFrozen(loaded['value']) && Object.isFrozen(held['configuration']),
		'Loaded configuration is not frozen',
	);
	filesystem.writeFileSync(
		path.join(root, 'rulebook.md'),
		'ACTOR_B: Changed disk standard. <!-- K001 -->\n',
	);
	let before = context.backend.captures.length;
	const original = await runConfigured(context, held, root);
	requireRequestSentinel(context.backend, before, 'ACTOR_A', 'ACTOR_B');
	requireSdkSources(original['sources'], 'file');
	const configuration = record(structuredClone(held['configuration']));
	const rulebooks = z.array(sdkRecordSchema).parse(configuration['rulebooks']);
	const first = rulebooks[0];
	requireSdk(first, 'No captured rulebook');
	const markdown = record(first['markdown']);
	configuration['rulebooks'] = [
		{
			...first,
			markdown: { ...markdown, value: 'ACTOR_EDIT: Edited typed standard. <!-- K001 -->\n' },
		},
		...rulebooks.slice(1),
	];
	before = context.backend.captures.length;
	const edited = await runConfigured(context, configuration, root);
	requireRequestSentinel(context.backend, before, 'ACTOR_EDIT', 'ACTOR_B');
	requireSdkSources(edited['sources'], 'edited');
	const reloaded = requireSdkKind(await context.sdk.load({ directory: root }), 'loaded');
	before = context.backend.captures.length;
	const refreshed = await runConfigured(context, reloaded['value'], root);
	requireRequestSentinel(context.backend, before, 'ACTOR_B', 'ACTOR_A');
	const missing = await context.sdk.load({ directory: root, configPath: 'absent.json' });
	requireSdkKind(missing, 'unavailable');
	filesystem.writeFileSync(path.join(root, 'broken.json'), '{');
	const malformed = await context.sdk.load({ directory: root, configPath: 'broken.json' });
	requireSdkKind(malformed, 'invalid');
	saveSdkEvidence(path.join(context.evidence, 'snapshot-outcomes.json'), {
		loaded,
		original,
		edited,
		refreshed,
		missing,
		malformed,
	});
	return {
		observed: [
			'held A, edited typed A, explicit reload B with distinct real requests',
			'file/edited content hashes',
			'missing/malformed config distinction',
		],
		gaps: [
			'unreadable dependency, malformed captured metadata and receipt-only override controls',
			'deep runtime freezing beyond the checked root values',
			'no-recapture syscall/counter join and copied filter mutation',
		],
	};
}

export async function observeSdkInline(context: JourneyContext): Promise<JourneyResult> {
	const root = path.join(context.evidence, 'inline-root');
	createCheckedSdkInputs(root);
	const base = path.join(context.evidence, 'separate-definition-base');
	filesystem.mkdirSync(base);
	const configuration = sdkInlineConfiguration(base, context.backend.endpoint);
	const before = context.backend.captures.length;
	const report = await runConfigured(context, configuration, root);
	requireSdkSources(report['sources'], 'inline');
	const requests = context.backend.captures.slice(before).map((capture) => capture.body);
	const questionTypes = new Set(
		requests.flatMap((request) =>
			Object.values(record(request['questions'])).map((question) => record(question)['type']),
		),
	);
	requireSdk(
		questionTypes.has('choice') && questionTypes.has('noul'),
		'Filter or asking rule was not reached',
	);
	requireSdk(
		JSON.stringify(requests).includes('CHECKED_A'),
		'Checked content never reached Rust requests',
	);
	const entries = filesystem.readdirSync(root);
	requireSdk(
		isDeepStrictEqual(entries, ['skills']),
		'SDK wrote a definition project in checked root',
	);
	saveSdkEvidence(path.join(context.evidence, 'inline-report.json'), report);
	if (!context.target.cli)
		return {
			observed: [
				'inline filter and asking execution with referenced utility and separate definition base',
			],
			gaps: [
				'CLI parity: no pinned CLI candidate supplied',
				'blocked-filter branch, per-source ID inventory, temporary writes outside checked root and deletion-before-return',
			],
		};
	const cliRoot = path.join(context.evidence, 'cli-root');
	createCheckedSdkInputs(cliRoot);
	createSdkFileDefinitions(cliRoot, context.backend.endpoint);
	const cliStart = context.backend.captures.length;
	// spawnSync would block this process's loopback responder. Keep CLI execution asynchronous.
	const { spawn } = await import('node:child_process');
	const child = spawn(
		context.target.cli.binary,
		['lint', '--config', path.join(cliRoot, 'sunstack.lint.json'), '--all', '--format', 'json'],
		{
			cwd: cliRoot,
			env: {
				PATH: process.env['PATH'],
				HOME: context.evidence,
				OPENROUTER_API_KEY: 'SYNTHETIC_SDK_LOOPBACK_ONLY',
				SUNSTACK_CREDENTIAL_STORE: 'off',
			},
		},
	);
	const stdout: Buffer[] = [];
	const stderr: Buffer[] = [];
	child.stdout.on('data', (chunk: Buffer) => stdout.push(chunk));
	child.stderr.on('data', (chunk: Buffer) => stderr.push(chunk));
	let timer: ReturnType<typeof setTimeout> | undefined;
	try {
		timer = setTimeout(() => child.kill('SIGKILL'), 15000);
		const exit = await new Promise<number | null>((resolve, reject) => {
			child.once('error', reject);
			child.once('close', resolve);
		});
		filesystem.writeFileSync(path.join(context.evidence, 'cli-stdout.json'), Buffer.concat(stdout));
		filesystem.writeFileSync(path.join(context.evidence, 'cli-stderr.log'), Buffer.concat(stderr));
		requireSdk(exit === 0, 'CLI parity counterpart did not complete cleanly');
		const cliValue: unknown = JSON.parse(Buffer.concat(stdout).toString('utf8'));
		const cli = requireCompletedSdkReport({
			kind: 'completed',
			report: cliValue,
		});
		for (const field of ['status', 'counts', 'file_counts', 'filter_skips'])
			requireSdk(isDeepStrictEqual(cli[field], report[field]), `CLI/SDK ${field} differs`);
		requireSdk(
			isDeepStrictEqual(
				context.backend.captures.slice(cliStart).map((capture) => capture.body),
				requests,
			),
			'CLI/SDK full encoded request bodies differ; inspect before introducing any normalization',
		);
	} finally {
		clearTimeout(timer);
		if (child.exitCode === null && child.signalCode === null) {
			child.kill('SIGKILL');
			await new Promise<void>((resolve) => child.once('close', () => resolve()));
		}
	}
	return {
		observed: ['inline/native filter/rule execution and exact CLI encoded request equality'],
		gaps: [
			'full semantic report comparison and all CLI root/scope/report/ContextCapture joins',
			'blocked-filter branch, complete source ID inventory and temporary-write observation',
		],
	};
}

export async function observeSdkIsolation(context: JourneyContext): Promise<JourneyResult> {
	const rootA = path.join(context.evidence, 'isolation-a');
	const rootB = path.join(context.evidence, 'isolation-b');
	createCheckedSdkInputs(rootA, 'ISOLATION_A');
	createCheckedSdkInputs(rootB, 'ISOLATION_B');
	const inputA = sdkInlineConfiguration(rootA, context.backend.endpoint, 'STANDARD_A');
	const inputB = sdkInlineConfiguration(rootB, context.backend.endpoint, 'STANDARD_B');
	const cwd = process.cwd();
	const environment = { ...process.env };
	const beforeCheck = context.backend.captures.length;
	const admission = await boundedSdkWait(context.sdk.check(inputA), 'credential-free check');
	requireSdkKind(admission, 'valid');
	requireSdk(context.backend.captures.length === beforeCheck, 'Check dispatched inference');
	const preAbort = new AbortController();
	preAbort.abort();
	const cancelled = requireSdkKind(
		await context.sdk.run(
			{ invalid: () => undefined },
			{ root: rootA, scope: { kind: 'all' }, signal: preAbort.signal },
		),
		'cancelled',
	);
	requireSdk(
		cancelled['report'] === null && context.backend.captures.length === beforeCheck,
		'Pre-abort performed work or fabricated report',
	);
	// Independent listeners identify the two real in-process calls without timing guesses.
	const backendB = new SdkControlledBackend();
	await backendB.start();
	const controllerA = new AbortController();
	const controllerB = new AbortController();
	context.backend.hold = true;
	backendB.hold = true;
	const pendingA = context.sdk.run(inputA, {
		root: rootA,
		scope: { kind: 'all' },
		signal: controllerA.signal,
	});
	const pendingB = context.sdk.run(
		{ ...inputB, decision_model: { model: 'jev-latest', endpoint: backendB.endpoint } },
		{ root: rootB, scope: { kind: 'all' }, signal: controllerB.signal },
	);
	// Attach handlers now so a failure before the gates cannot become an unhandled rejection.
	const results = Promise.allSettled([pendingA, pendingB]);
	try {
		await Promise.all([
			context.backend.waitForRequests(beforeCheck + 1),
			backendB.waitForRequests(1),
		]);
		controllerA.abort();
		const resultA = requireSdkKind(await boundedSdkWait(pendingA, 'cancel A'), 'cancelled');
		requireSdk(resultA['report'] !== null, 'In-flight A lost its actual partial report');
		backendB.releaseHeld();
		const resultB = requireCompletedSdkReport(
			await boundedSdkWait(pendingB, 'complete unaffected B'),
		);
		requireSdk(
			JSON.stringify(backendB.captures).includes('ISOLATION_B') &&
				!JSON.stringify(backendB.captures).includes('ISOLATION_A'),
			'B used A context',
		);
		requireSdk(
			isDeepStrictEqual({ ...process.env }, environment) && process.cwd() === cwd,
			'SDK mutated process environment/CWD',
		);
		saveSdkEvidence(path.join(context.evidence, 'isolation-outcomes.json'), {
			admission,
			cancelled,
			resultA,
			resultB,
			bRequests: backendB.captures,
		});
	} finally {
		controllerA.abort();
		controllerB.abort();
		await backendB.close();
		await boundedSdkWait(results, 'both owned SDK calls settle');
		context.backend.hold = false;
	}
	return {
		observed: [
			'valid check with zero HTTP',
			'pre-abort beats invalid encoding',
			'same-process two-addon-call causal cancellation and unaffected B',
			'process CWD/environment preserved',
		],
		gaps: [
			'native task drain, zero credential/checked-file acquisition joins',
			'partial incomplete, recoverable reply fault, all exact outcomes and whole-run replay sensitivity',
		],
	};
}

export function inspectSdkInstall(): JourneyResult {
	const probes = ['rustc', 'cargo'].map((command) =>
		spawnSync(command, ['--version'], { encoding: 'utf8', timeout: 5000 }),
	);
	requireSdk(
		probes.every((probe) => probe.error && 'code' in probe.error && probe.error.code === 'ENOENT'),
		'Install proof requires consumer environment without cargo/rustc',
	);
	return {
		observed: [
			'declared installed artifact hashes, entry resolution, runtime OS/CPU/libc, no ambient Zod or Rust tools',
		],
		gaps: [
			'fresh package-manager install transcript and optional-package topology',
			'missing/skew/runtime/platform controls before effects',
			'other platform cells are separate observations',
		],
	};
}

export async function observeSdkProvider(context: JourneyContext): Promise<JourneyResult> {
	const root = path.join(context.evidence, 'provider-root');
	createCheckedSdkInputs(root, 'Synthetic provider verification instruction.');
	const inline = sdkInlineConfiguration(root, context.backend.endpoint);
	// Deliberately remove the controlled endpoint. The ordinary Rust host resolves credentials.
	const report = await runConfigured(
		context,
		{ ...inline, decision_model: { model: 'jev-latest' } },
		root,
	);
	const requests = z.array(sdkRecordSchema).min(1).parse(report['requests']);
	for (const request of requests) {
		const outcome = record(request['outcome']);
		requireSdk(
			outcome['state'] === 'run',
			'Provider request contains only prepared/reused evidence',
		);
		const decision = requireSdkKind(outcome['outcome'], 'completed');
		const receipt = record(decision['receipt']);
		requireSdk(
			typeof receipt['resolved_model'] === 'string' &&
				!receipt['resolved_model'].includes('controlled'),
			'Missing ordinary provider model receipt',
		);
	}
	requireSdk(context.backend.captures.length === 0, 'Hosted journey reached controlled backend');
	saveSdkEvidence(path.join(context.evidence, 'provider-report.json'), report);
	return {
		observed: ['installed SDK ordinary backend run with actual completed request receipts'],
		gaps: [
			'usage accounting and complete receipt schema must be joined to current product O3/native proof',
			'provider output is not inference-quality evaluation or six-cell distribution proof',
		],
	};
}
