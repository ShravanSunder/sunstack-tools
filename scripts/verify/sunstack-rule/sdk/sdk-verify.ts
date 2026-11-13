import { spawnSync } from 'node:child_process';
/** Run a single lasting installed-SDK journey; --help describes inputs and remaining proof joins. */
import filesystem from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import { parseArgs } from 'node:util';

import { z } from 'zod';

import { observeSdkCodecRoots } from './sdk-codec-roots.ts';
import { observeSdkCodecRefusals } from './sdk-codec.ts';
import { SdkControlledBackend } from './sdk-controlled-backend.ts';
import { observeSdkPackagedExamples } from './sdk-examples.ts';
import { observeSdkInspection } from './sdk-inspection.ts';
import {
	observeSdkSnapshot,
	observeSdkInline,
	observeSdkIsolation,
	inspectSdkInstall,
	observeSdkProvider,
	type JourneyResult,
} from './sdk-journeys.ts';
import {
	sdkTargetSchema,
	confirmSdkTarget,
	importInstalledSdk,
	inspectInstalledSdkContract,
	requireSdk,
	containedPath,
	fileDigest,
	saveSdkEvidence,
} from './sdk-target.ts';

const help = `Installed Sunstack SDK observations (draft until driven on a declared candidate).
From monorepo root:
  node scripts/verify/sunstack-rule/sdk/sdk-verify.ts --target <absolute-target.json> --journey snapshot|inline|isolation|install|codec|inspection|examples|provider --evidence <new-absolute-directory>
Target JSON uses sdkTargetSchema in sdk-target.ts: sourceRoot/full commit, fresh
consumerRoot/packageRoot/resolved entry/addon, tarball/hash, installed files/hashes,
release/contractRevision and actual platform/arch/libc. Optional cli binary/hash
enables inline CLI parity. Include package.json, entry, addon and all declarations/
runtime/platform-package files in files. No source/workspace links are permitted.
Provider additionally requires --allow-provider and an explicit separate commission.
Controlled journeys require no inherited key; they set one synthetic loopback key
and disable the credential store before native import, restoring both on exit.
Install runs where cargo/rustc and Zod are absent. Compiler is the repository's
installed TypeScript CLI, overridable with --compiler <absolute-existing-tsc>.
Each receipt lists observed portions and open native/CLI/platform proof joins.
Exit 0 means the portion ran; gaps yield exit 3, setup/assertion failures exit 1.
Never interpret an apparatus-unit pass or a draft/gapped receipt as SDK acceptance.
`;

async function main(): Promise<void> {
	const { values } = parseArgs({
		options: {
			help: { type: 'boolean' },
			target: { type: 'string' },
			journey: { type: 'string' },
			evidence: { type: 'string' },
			compiler: { type: 'string' },
			'allow-provider': { type: 'boolean' },
		},
		strict: true,
	});
	if (values.help) {
		process.stdout.write(help);
		return;
	}
	const journey = z
		.enum([
			'snapshot',
			'inline',
			'isolation',
			'install',
			'codec',
			'inspection',
			'examples',
			'provider',
		])
		.parse(values.journey);
	if (journey === 'provider')
		requireSdk(
			values['allow-provider'],
			'Provider remains held: a separate authorized invocation is required',
		);
	const targetPath = z.string().refine(path.isAbsolute).parse(values.target);
	const target = sdkTargetSchema.parse(JSON.parse(filesystem.readFileSync(targetPath, 'utf8')));
	const evidence = z.string().refine(path.isAbsolute).parse(values.evidence);
	requireSdk(
		!containedPath(target.sourceRoot, evidence) && !containedPath(target.consumerRoot, evidence),
		'Evidence must be outside product and consumer',
	);
	filesystem.mkdirSync(evidence, { recursive: false });
	saveSdkEvidence(path.join(evidence, 'target.json'), target);
	const oldKey = process.env['OPENROUTER_API_KEY'];
	const oldStore = process.env['SUNSTACK_CREDENTIAL_STORE'];
	const backend = new SdkControlledBackend();
	let started = false;
	let outcome: JourneyResult | undefined;
	try {
		if (journey !== 'provider') {
			requireSdk(!oldKey, 'Controlled journey must start without an inherited real key');
			process.env['OPENROUTER_API_KEY'] = 'SYNTHETIC_SDK_LOOPBACK_ONLY';
			process.env['SUNSTACK_CREDENTIAL_STORE'] = 'off';
		}
		confirmSdkTarget(target);
		const compilerManifestPath = createRequire(import.meta.url).resolve('typescript/package.json');
		const compilerManifest = z
			.object({ bin: z.object({ tsc: z.string() }) })
			.parse(JSON.parse(filesystem.readFileSync(compilerManifestPath, 'utf8')));
		const compiler =
			values.compiler ?? path.resolve(path.dirname(compilerManifestPath), compilerManifest.bin.tsc);
		requireSdk(path.isAbsolute(compiler), 'Compiler path must be absolute');
		const consumer = path.join(
			target.consumerRoot,
			`sdk-verification-${path.basename(evidence)}.ts`,
		);
		filesystem.writeFileSync(
			consumer,
			`import {load, check, run, type Configuration} from '@sunstack/sdk';\ndeclare const configuration: Configuration;\nconst loaded = await load({directory: '/synthetic'});\nconst checked = await check(configuration);\nconst outcome = await run(configuration, {root: '/synthetic', scope: {kind: 'all'}});\nif (outcome.kind === 'completed') { outcome.report.findings; }\nvoid loaded; void checked;\n`,
			{ flag: 'wx' },
		);
		const compiled = spawnSync(
			process.execPath,
			[
				compiler,
				'--noEmit',
				'--strict',
				'--target',
				'ES2023',
				'--module',
				'NodeNext',
				'--moduleResolution',
				'NodeNext',
				consumer,
			],
			{ cwd: target.consumerRoot, encoding: 'utf8', timeout: 30000 },
		);
		saveSdkEvidence(path.join(evidence, 'consumer-compiler.json'), {
			compiler,
			compilerSha256: fileDigest(compiler),
			runtime: process.execPath,
			node: process.version,
			code: compiled.status,
			signal: compiled.signal,
			stdout: compiled.stdout,
			stderr: compiled.stderr,
			error: compiled.error?.message,
		});
		requireSdk(
			compiled.status === 0 && !compiled.error,
			'Installed positive caller did not compile',
		);
		await backend.start();
		started = true;
		const sdk = await importInstalledSdk(target);
		saveSdkEvidence(
			path.join(evidence, 'native-contract.json'),
			await inspectInstalledSdkContract(sdk, target),
		);
		const context = { sdk, target, evidence, backend };
		switch (journey) {
			case 'snapshot':
				outcome = await observeSdkSnapshot(context);
				break;
			case 'inline':
				outcome = await observeSdkInline(context);
				break;
			case 'isolation':
				outcome = await observeSdkIsolation(context);
				break;
			case 'codec': {
				const refusals = await observeSdkCodecRefusals(context);
				const roots = await observeSdkCodecRoots(context);
				outcome = {
					observed: [...refusals.observed, ...roots.observed],
					gaps: [...refusals.gaps, ...roots.gaps],
				};
				break;
			}
			case 'inspection':
				outcome = await observeSdkInspection(context);
				break;
			case 'examples':
				outcome = observeSdkPackagedExamples({ target, evidence, compiler });
				break;
			case 'install': {
				const install = inspectSdkInstall();
				const inline = await observeSdkInline(context);
				outcome = {
					observed: [...install.observed, ...inline.observed],
					gaps: [...install.gaps, ...inline.gaps],
				};
				break;
			}
			case 'provider':
				outcome = await observeSdkProvider(context);
				break;
		}
		confirmSdkTarget(target);
		requireSdk(backend.errors.length === 0, 'Controlled backend failed');
	} catch (error) {
		saveSdkEvidence(path.join(evidence, 'failure.json'), {
			journey,
			code: 'observation_failed',
			message: error instanceof Error ? error.message : 'Unknown failure',
		});
		throw error;
	} finally {
		try {
			if (started) await backend.close();
		} finally {
			if (oldKey === undefined) delete process.env['OPENROUTER_API_KEY'];
			else process.env['OPENROUTER_API_KEY'] = oldKey;
			if (oldStore === undefined) delete process.env['SUNSTACK_CREDENTIAL_STORE'];
			else process.env['SUNSTACK_CREDENTIAL_STORE'] = oldStore;
			saveSdkEvidence(path.join(evidence, 'controlled-http.json'), {
				requests: backend.captures,
				errors: backend.errors,
			});
		}
	}
	requireSdk(outcome, 'No journey executed');
	saveSdkEvidence(path.join(evidence, 'receipt.json'), {
		journey,
		sourceCommit: target.commit,
		node: process.version,
		platform: process.platform,
		arch: process.arch,
		...outcome,
		coverage: outcome.gaps.length ? 'partial' : 'observed',
	});
	process.stdout.write(
		`${journey}: ${outcome.observed.length} observed portions; ${outcome.gaps.length} required joins open; ${evidence}\n`,
	);
	if (outcome.gaps.length) process.exitCode = 3;
}

void main().catch((error: unknown) => {
	process.stderr.write(
		`SDK journey failed: ${error instanceof Error ? error.message : 'unknown error'}\n`,
	);
	process.exitCode = 1;
});
