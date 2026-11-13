/** Compile and run keyless examples copied from the pinned installed package. */
import { spawnSync } from 'node:child_process';
import filesystem from 'node:fs';
import path from 'node:path';

import { observeSdkCliGuide } from './sdk-cli-guide.ts';
import type { JourneyResult } from './sdk-journeys.ts';
import { requireSdk, saveSdkEvidence, type SdkTarget } from './sdk-target.ts';

export function observeSdkPackagedExamples(options: {
	readonly target: SdkTarget;
	readonly evidence: string;
	readonly compiler: string;
}): JourneyResult {
	const { target, evidence, compiler } = options;
	const goodRoot = path.join(evidence, 'examples-good');
	const emptyRoot = path.join(evidence, 'examples-empty');
	filesystem.mkdirSync(path.join(goodRoot, 'docs'), { recursive: true });
	filesystem.mkdirSync(path.join(goodRoot, 'src'), { recursive: true });
	filesystem.mkdirSync(emptyRoot);
	filesystem.writeFileSync(
		path.join(goodRoot, 'docs', 'sample.md'),
		'---\ndescription: Use when testing.\n---\n\n# Heading\n\nBody.\n',
	);
	filesystem.writeFileSync(path.join(goodRoot, 'src', 'calls.ts'), 'console.log("example");\n');
	const env: NodeJS.ProcessEnv = { ...process.env, SUNSTACK_CREDENTIAL_STORE: 'off' };
	delete env['OPENROUTER_API_KEY'];
	const readme = filesystem.readFileSync(path.join(target.packageRoot, 'README.md'), 'utf8');
	const advertised = new Set(
		[...readme.matchAll(/examples\/([a-z0-9-]+)\.ts/gu)].map((match) => match[1]),
	);
	for (const required of ['mechanical-run', 'heading-fields', 'inspect-and-save'])
		requireSdk(advertised.has(required), `Installed README does not advertise ${required}`);
	for (const name of advertised) {
		requireSdk(name, 'Advertised example name missing');
		const installed = path.join(target.packageRoot, 'examples', `${name}.ts`);
		const copied = path.join(
			target.consumerRoot,
			`sdk-example-${path.basename(evidence)}-${name}.ts`,
		);
		filesystem.copyFileSync(installed, copied, filesystem.constants.COPYFILE_EXCL);
		const compiled = spawnSync(
			process.execPath,
			[
				compiler,
				'--noEmit',
				'--strict',
				'--target',
				'ES2024',
				'--module',
				'NodeNext',
				'--moduleResolution',
				'NodeNext',
				copied,
			],
			{ cwd: target.consumerRoot, env, encoding: 'utf8', timeout: 30000 },
		);
		saveSdkEvidence(path.join(evidence, `${name}-compiler.json`), {
			status: compiled.status,
			signal: compiled.signal,
			stdout: compiled.stdout,
			stderr: compiled.stderr,
			error: compiled.error?.message,
		});
		requireSdk(compiled.status === 0 && !compiled.error, `${name}: packed example did not compile`);
		if (name !== 'mechanical-run' && name !== 'heading-fields' && name !== 'inspect-and-save')
			continue;
		for (const [label, root, expected] of [
			['valid', goodRoot, 0],
			['empty', emptyRoot, 1],
		] as const) {
			const destination = path.join(evidence, `example-${name}-${label}.sdk.json`);
			const exampleArguments =
				name === 'inspect-and-save' ? [copied, root, destination] : [copied, root];
			const result = spawnSync(process.execPath, exampleArguments, {
				cwd: target.consumerRoot,
				env,
				encoding: 'utf8',
				timeout: 30000,
			});
			saveSdkEvidence(path.join(evidence, `${name}-${label}.json`), {
				status: result.status,
				signal: result.signal,
				stdout: result.stdout,
				stderr: result.stderr,
				error: result.error?.message,
			});
			requireSdk(
				!result.error && result.status === expected,
				`${name} ${label}: unexpected process exit`,
			);
			if (name === 'inspect-and-save') {
				requireSdk(
					result.stdout.includes('inspection: clean; rules were not evaluated'),
					`${name} ${label}: listing status or explanation missing`,
				);
				if (label === 'valid') {
					requireSdk(
						result.stdout.includes('restored exact typed outcome') &&
							result.stdout.includes('revision refused:'),
						'Inspection example did not read back and refuse another revision',
					);
					requireSdk(
						filesystem.existsSync(destination),
						'Inspection example did not write its document',
					);
				} else
					requireSdk(
						!filesystem.existsSync(destination),
						'Empty inspection incorrectly wrote a successful output',
					);
			} else {
				requireSdk(
					result.stdout.includes('check: valid'),
					`${name} ${label}: admission was not observed`,
				);
				requireSdk(
					result.stdout.includes(label === 'valid' ? 'run: clean' : 'run: nothing_selected'),
					`${name} ${label}: unexpected run status`,
				);
			}
		}
	}
	const guideObservations = target.cli ? observeSdkCliGuide({ target, evidence }) : [];
	return {
		observed: [
			...guideObservations,
			'all README-advertised example files resolve in the installed package and compile strictly',
			'installed mechanical, heading and inspect/save examples run on valid and empty input',
		],
		gaps: [
			'other advertised example runtime paths and other declared failure endings',
			...(!target.cli ? ['complete CLI guide example: no CLI supplied'] : []),
		],
	};
}
