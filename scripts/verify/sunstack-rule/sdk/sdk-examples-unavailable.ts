/** Exercise each packed example against an isolated wrapper with its native addon absent. */
import { spawnSync } from 'node:child_process';
import filesystem from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';

import { z } from 'zod';

import {
	containedPath,
	fileDigest,
	requireSdk,
	saveSdkEvidence,
	type SdkTarget,
} from './sdk-target.ts';

/** A nonzero process alone does not establish the intended typed failure. */
export function requireSdkMissingAddonExampleResult(options: {
	readonly name: string;
	readonly status: number | null;
	readonly stdout: string;
	readonly stderr: string;
	readonly error: Error | undefined;
}): void {
	requireSdk(
		!options.error && options.status === 1,
		`${options.name}: missing addon did not produce exit 1`,
	);
	const output = `${options.stdout}\n${options.stderr}`;
	requireSdk(
		output.includes('addon_missing') && output.includes('before_dispatch'),
		`${options.name}: typed missing-addon phase/reason not visible`,
	);
}

export function observeSdkExamplesWithoutAddon(options: {
	readonly target: SdkTarget;
	readonly evidence: string;
	readonly exampleNames: readonly string[];
	readonly checkedRoot: string;
	readonly project: string;
}): string[] {
	const { target, evidence, exampleNames, checkedRoot, project } = options;
	const consumer = path.join(evidence, 'consumer-without-addon');
	const scope = path.join(consumer, 'node_modules', '@sunstack');
	filesystem.mkdirSync(scope, { recursive: true });
	filesystem.writeFileSync(
		path.join(consumer, 'package.json'),
		'{"private":true,"type":"module"}\n',
	);
	const extraction = spawnSync('tar', ['-xzf', target.tarball, '-C', scope], {
		encoding: 'utf8',
		timeout: 30000,
	});
	saveSdkEvidence(path.join(evidence, 'missing-addon-extraction.json'), {
		status: extraction.status,
		signal: extraction.signal,
		stdout: extraction.stdout,
		stderr: extraction.stderr,
		error: extraction.error?.message,
	});
	requireSdk(
		!extraction.error && extraction.status === 0,
		'Could not extract the pinned SDK tarball',
	);
	const packageRoot = path.join(scope, 'sdk');
	filesystem.renameSync(path.join(scope, 'package'), packageRoot);
	const pinned = target.files.filter((file) => containedPath(target.packageRoot, file.path));
	requireSdk(pinned.length > 0, 'No wrapper files were pinned');
	const expectedPaths = new Set(pinned.map((file) => path.relative(target.packageRoot, file.path)));
	const extractedPaths: string[] = [];
	const visit = (directory: string): void => {
		for (const entry of filesystem.readdirSync(directory, { withFileTypes: true })) {
			const file = path.join(directory, entry.name);
			requireSdk(!entry.isSymbolicLink(), `Extracted SDK contains a link: ${file}`);
			if (entry.isDirectory()) visit(file);
			else {
				requireSdk(entry.isFile(), `Extracted SDK contains a non-file: ${file}`);
				extractedPaths.push(path.relative(packageRoot, file));
			}
		}
	};
	visit(packageRoot);
	requireSdk(
		extractedPaths.length === expectedPaths.size &&
			extractedPaths.every((name) => expectedPaths.has(name)),
		'Extracted SDK file inventory differs from pinned installation',
	);
	for (const file of pinned) {
		const copy = path.join(packageRoot, path.relative(target.packageRoot, file.path));
		requireSdk(fileDigest(copy) === file.sha256, `Extracted SDK differs: ${copy}`);
	}
	const resolver = createRequire(path.join(packageRoot, 'package.json'));
	const cell = `${target.platform}-${target.arch}${target.libc === 'none' ? '' : `-${target.libc === 'glibc' ? 'gnu' : 'musl'}`}`;
	let addonResolved = false;
	try {
		resolver.resolve(`@sunstack/sdk-${cell}`);
		addonResolved = true;
	} catch {
		/* The addon must be absent. */
	}
	requireSdk(!addonResolved, 'Isolated consumer unexpectedly resolves the addon');
	const env: NodeJS.ProcessEnv = { ...process.env, SUNSTACK_CREDENTIAL_STORE: 'off' };
	delete env['OPENROUTER_API_KEY'];
	const probe = spawnSync(
		process.execPath,
		[
			'--input-type=module',
			'-e',
			"import {contractInfo} from '@sunstack/sdk'; console.log(JSON.stringify(await contractInfo()));",
		],
		{ cwd: consumer, env, encoding: 'utf8', timeout: 30000 },
	);
	saveSdkEvidence(path.join(evidence, 'missing-addon-contract.json'), {
		status: probe.status,
		signal: probe.signal,
		stdout: probe.stdout,
		stderr: probe.stderr,
		error: probe.error?.message,
	});
	requireSdk(!probe.error && probe.status === 0, 'Missing-addon contract probe failed to run');
	const contract = z
		.object({
			kind: z.literal('failed'),
			failure: z.object({
				phase: z.literal('before_dispatch'),
				effects: z.literal('none'),
				reason: z.literal('addon_missing'),
			}),
		})
		.parse(JSON.parse(probe.stdout));
	requireSdk(
		contract.failure.reason === 'addon_missing',
		'Missing-addon probe returned another failure',
	);
	for (const name of exampleNames) {
		const allowed = [
			'mechanical-run',
			'heading-fields',
			'inspect-and-save',
			'load-and-edit',
			'inline-configuration',
		];
		requireSdk(allowed.includes(name), `No missing-addon arguments for advertised example ${name}`);
		const source = path.join(packageRoot, 'examples', `${name}.ts`);
		const copied = path.join(consumer, `${name}.ts`);
		filesystem.copyFileSync(source, copied, filesystem.constants.COPYFILE_EXCL);
		requireSdk(
			fileDigest(copied) === fileDigest(path.join(target.packageRoot, 'examples', `${name}.ts`)),
			`Copied example differs: ${name}`,
		);
		const destination = path.join(evidence, `missing-addon-${name}.sdk.json`);
		const argumentsList =
			name === 'inspect-and-save'
				? [copied, checkedRoot, destination]
				: name === 'load-and-edit'
					? [copied, project, checkedRoot]
					: [copied, checkedRoot];
		const result = spawnSync(process.execPath, argumentsList, {
			cwd: consumer,
			env,
			encoding: 'utf8',
			timeout: 30000,
		});
		saveSdkEvidence(path.join(evidence, `missing-addon-${name}.json`), {
			status: result.status,
			signal: result.signal,
			stdout: result.stdout,
			stderr: result.stderr,
			error: result.error?.message,
		});
		requireSdkMissingAddonExampleResult({
			name,
			status: result.status,
			stdout: result.stdout,
			stderr: result.stderr,
			error: result.error,
		});
		if (name === 'inspect-and-save')
			requireSdk(!filesystem.existsSync(destination), 'Missing addon wrote a successful document');
	}
	return [
		'each advertised packed example exits nonzero and exposes the missing-addon before-dispatch failure in an isolated consumer',
	];
}
