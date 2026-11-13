/** Compile and run keyless examples copied from the pinned installed package. */
import { spawnSync } from 'node:child_process';
import filesystem from 'node:fs';
import path from 'node:path';

import { z } from 'zod';

import { observeSdkCliGuide } from './sdk-cli-guide.ts';
import { observeSdkExamplesWithoutAddon } from './sdk-examples-unavailable.ts';
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
	const badRoot = path.join(evidence, 'examples-finding');
	const oversizedRoot = path.join(evidence, 'examples-incomplete');
	const project = path.join(evidence, 'examples-load-project');
	filesystem.mkdirSync(path.join(goodRoot, 'docs'), { recursive: true });
	filesystem.mkdirSync(path.join(goodRoot, 'src'), { recursive: true });
	filesystem.mkdirSync(path.join(badRoot, 'docs'), { recursive: true });
	filesystem.mkdirSync(path.join(oversizedRoot, 'docs'), { recursive: true });
	filesystem.mkdirSync(path.join(oversizedRoot, 'src'), { recursive: true });
	filesystem.mkdirSync(path.join(project, 'evidence', 'rules'), { recursive: true });
	filesystem.mkdirSync(emptyRoot);
	filesystem.writeFileSync(
		path.join(goodRoot, 'docs', 'sample.md'),
		'---\ndescription: Use when testing.\n---\n\n# Heading\n\nBody.\n',
	);
	filesystem.writeFileSync(path.join(goodRoot, 'src', 'calls.ts'), 'console.log("example");\n');
	filesystem.writeFileSync(
		path.join(badRoot, 'docs', 'sample.md'),
		'---\ndescription: Wrong opening.\n---\n\n# Heading\n',
	);
	filesystem.writeFileSync(
		path.join(oversizedRoot, 'docs', 'large.md'),
		`---\ndescription: Use when large.\n---\n# Heading\n${'x'.repeat(1_048_576)}\n`,
	);
	filesystem.writeFileSync(
		path.join(oversizedRoot, 'src', 'large.ts'),
		`console.log("oversized");\n${' '.repeat(1_048_576)}\n`,
	);
	filesystem.writeFileSync(path.join(project, 'evidence', 'sgconfig.yml'), 'ruleDirs: [rules]\n');
	filesystem.writeFileSync(
		path.join(project, 'evidence', 'rules', 'skill.yml'),
		'id: skill\nlanguage: Markdown\nrule: {matches: frontmatter}\n',
	);
	filesystem.writeFileSync(
		path.join(project, 'rulebook.md'),
		'Descriptions begin with Use when. <!-- K001 -->\n',
	);
	filesystem.writeFileSync(
		path.join(project, 'rules.json'),
		JSON.stringify({
			K001: {
				name: 'description',
				evidence: 'skill',
				severity: 'error',
				decision: {
					start: 'starts',
					steps: {
						starts: {
							mechanical_check: { tool: 'regex', field: 'description', pattern: '^Use when' },
							yes: 'pass',
							['no']: { finding: 'Wrong description.' },
						},
					},
				},
			},
		}),
	);
	filesystem.writeFileSync(
		path.join(project, 'sunstack.lint.json'),
		JSON.stringify({
			marker_prefix: 'K',
			check: { files: ['docs/*.md'] },
			decision_model: { model: 'jev-latest' },
			rulebook: [{ markdown: 'rulebook.md', rules: 'rules.json' }],
			evidence: {
				ast_grep: 'evidence/sgconfig.yml',
				kinds: {
					skill: { rule: 'skill', fields: { description: { from: 'yaml', key: 'description' } } },
				},
			},
		}),
	);
	const env: NodeJS.ProcessEnv = { ...process.env, SUNSTACK_CREDENTIAL_STORE: 'off' };
	delete env['OPENROUTER_API_KEY'];
	const readme = filesystem.readFileSync(path.join(target.packageRoot, 'README.md'), 'utf8');
	const advertised = new Set(
		[...readme.matchAll(/examples\/([a-z0-9-]+)\.ts/gu)].map((match) => match[1]),
	);
	for (const required of ['mechanical-run', 'heading-fields', 'inspect-and-save', 'load-and-edit'])
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
				'--ignoreConfig',
				'--noEmit',
				'--strict',
				'--target',
				'ES2024',
				'--module',
				'NodeNext',
				'--moduleResolution',
				'NodeNext',
				'--types',
				'node',
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
		if (
			name !== 'mechanical-run' &&
			name !== 'heading-fields' &&
			name !== 'inspect-and-save' &&
			name !== 'load-and-edit'
		)
			continue;
		for (const [label, root, expected] of [
			['valid', goodRoot, 0],
			['empty', emptyRoot, 1],
		] as const) {
			const destination = path.join(evidence, `example-${name}-${label}.sdk.json`);
			const exampleArguments =
				name === 'inspect-and-save'
					? [copied, root, destination]
					: name === 'load-and-edit'
						? [copied, project, root]
						: [copied, root];
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
			} else if (name === 'load-and-edit') {
				requireSdk(
					result.stdout.includes('check: valid') &&
						result.stdout.includes('run: completed') &&
						result.stdout.includes(
							label === 'valid' ? 'status: clean' : 'status: nothing_selected',
						),
					`${name} ${label}: unexpected admission or run status`,
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
		if (name === 'mechanical-run' || name === 'load-and-edit') {
			const args = name === 'load-and-edit' ? [copied, project, badRoot] : [copied, badRoot];
			const finding = spawnSync(process.execPath, args, {
				cwd: target.consumerRoot,
				env,
				encoding: 'utf8',
				timeout: 30000,
			});
			saveSdkEvidence(path.join(evidence, `${name}-finding.json`), {
				status: finding.status,
				signal: finding.signal,
				stdout: finding.stdout,
				stderr: finding.stderr,
				error: finding.error?.message,
			});
			requireSdk(
				!finding.error && finding.status === 1 && finding.stdout.includes('findings'),
				`${name}: finding did not produce nonzero exit`,
			);
		}
		if (name === 'load-and-edit') {
			const missing = spawnSync(
				process.execPath,
				[copied, path.join(evidence, 'missing-project'), goodRoot],
				{ cwd: target.consumerRoot, env, encoding: 'utf8', timeout: 30000 },
			);
			saveSdkEvidence(path.join(evidence, `${name}-unavailable.json`), {
				status: missing.status,
				signal: missing.signal,
				stdout: missing.stdout,
				stderr: missing.stderr,
				error: missing.error?.message,
			});
			requireSdk(
				!missing.error && missing.status === 1 && missing.stdout.includes('load:'),
				'Missing project did not produce a visible failure',
			);
		}
		const incompleteDestination = path.join(evidence, `example-${name}-incomplete.sdk.json`);
		const incompleteArguments =
			name === 'inspect-and-save'
				? [copied, oversizedRoot, incompleteDestination]
				: name === 'load-and-edit'
					? [copied, project, oversizedRoot]
					: [copied, oversizedRoot];
		const incomplete = spawnSync(process.execPath, incompleteArguments, {
			cwd: target.consumerRoot,
			env,
			encoding: 'utf8',
			timeout: 30000,
		});
		saveSdkEvidence(path.join(evidence, `${name}-incomplete.json`), {
			status: incomplete.status,
			signal: incomplete.signal,
			stdout: incomplete.stdout,
			stderr: incomplete.stderr,
			error: incomplete.error?.message,
		});
		requireSdk(
			!incomplete.error &&
				incomplete.status === 1 &&
				`${incomplete.stdout}\n${incomplete.stderr}`.includes('incomplete'),
			`${name}: oversized input did not expose an incomplete failure`,
		);
		if (name === 'inspect-and-save')
			requireSdk(
				!filesystem.existsSync(incompleteDestination),
				'Incomplete inspection wrote a successful output',
			);
	}
	const unavailableObservations = observeSdkExamplesWithoutAddon({
		target,
		evidence,
		exampleNames: z.array(z.string()).parse([...advertised]),
		checkedRoot: goodRoot,
		project,
	});
	const guideObservations = target.cli ? observeSdkCliGuide({ target, evidence }) : [];
	return {
		observed: [
			...unavailableObservations,
			...guideObservations,
			'all README-advertised example files resolve in the installed package and compile strictly',
			'installed keyless mechanical, load/edit, heading and inspect/save examples run on valid and empty input',
			'mechanical and load/edit examples exit nonzero for findings; missing project fails',
			'all four keyless examples exit nonzero and expose an incomplete acquisition',
		],
		gaps: [
			'asking example successful model-backed runtime and other after-dispatch failure endings',
			...(!target.cli ? ['complete CLI guide example: no CLI supplied'] : []),
		],
	};
}
