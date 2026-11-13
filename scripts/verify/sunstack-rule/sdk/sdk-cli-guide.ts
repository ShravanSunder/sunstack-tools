/** Copy the complete source guide example and run its ordinary CLI commands. */
import { spawnSync } from 'node:child_process';
import filesystem from 'node:fs';
import path from 'node:path';

import { z } from 'zod';

import { requireSdk, saveSdkEvidence, type SdkTarget } from './sdk-target.ts';

export function observeSdkCliGuide(options: {
	readonly target: SdkTarget;
	readonly evidence: string;
}): string[] {
	const { target, evidence } = options;
	const cli = target.cli;
	requireSdk(cli, 'CLI guide requires a pinned CLI');
	const guide = filesystem.readFileSync(
		path.join(target.sourceRoot, 'crates', 'sunstack', 'README.md'),
		'utf8',
	);
	const sectionStart = guide.indexOf('### Complete keyless example');
	const sectionEnd = guide.indexOf('\nEvidence fields can use', sectionStart);
	requireSdk(sectionStart >= 0 && sectionEnd > sectionStart, 'Complete guide section missing');
	const section = guide.slice(sectionStart, sectionEnd);
	const root = path.join(evidence, 'copied-cli-guide');
	filesystem.mkdirSync(root);
	for (const name of [
		'sunstack.lint.json',
		'rulebook.md',
		'rulebook.rules.json',
		'evidence/sgconfig.yml',
		'evidence/rules/skill.yml',
		'docs/one.md',
	]) {
		const label = section.indexOf(`\`${name}\``);
		const fence = section.indexOf('```', label);
		const bodyStart = section.indexOf('\n', fence);
		const bodyEnd = section.indexOf('\n```', bodyStart);
		requireSdk(
			label >= 0 && fence > label && bodyStart > fence && bodyEnd > bodyStart,
			`Guide example missing ${name}`,
		);
		const destination = path.join(root, name);
		filesystem.mkdirSync(path.dirname(destination), { recursive: true });
		filesystem.writeFileSync(destination, `${section.slice(bodyStart + 1, bodyEnd)}\n`);
	}
	const env: NodeJS.ProcessEnv = { ...process.env, SUNSTACK_CREDENTIAL_STORE: 'off' };
	delete env['OPENROUTER_API_KEY'];
	const execute = (name: string, argumentsList: readonly string[], expected: number): string => {
		const result = spawnSync(cli.binary, ['lint', ...argumentsList], {
			cwd: root,
			env,
			encoding: 'utf8',
			timeout: 30000,
		});
		saveSdkEvidence(path.join(evidence, `guide-${name}.json`), {
			status: result.status,
			signal: result.signal,
			stdout: result.stdout,
			stderr: result.stderr,
			error: result.error?.message,
		});
		requireSdk(!result.error && result.status === expected, `Guide ${name}: unexpected exit`);
		return result.stdout;
	};
	const help = execute('help', ['--help'], 0);
	for (const phrase of [
		'--check',
		'--evidence',
		'--prepare',
		'--report',
		'--init',
		'not absence of violations',
		'no rule evaluation',
	])
		requireSdk(help.includes(phrase), `CLI help omitted ${phrase}`);
	execute('check', ['--check'], 0);
	const listing = execute('evidence', ['--all', '--evidence', 'skill'], 0);
	const value = z
		.object({
			status: z.object({ kind: z.literal('clean') }),
			pieces: z.array(z.object({ text: z.string() })).min(1),
		})
		.parse(JSON.parse(listing));
	requireSdk(
		value.pieces.some((piece) => piece.text.includes('Use when checking a complete example.')),
		'Guide evidence omitted its original text',
	);
	execute('run-clean', ['--all', '--format', 'json'], 0);
	execute('prepare', ['--all', '--prepare', 'prepared.json'], 6);
	const checkedFile = path.join(root, 'docs', 'one.md');
	const original = filesystem.readFileSync(checkedFile, 'utf8');
	const changed = original.replace('Use when checking a complete example.', 'Wrong opening');
	requireSdk(changed !== original, 'Guide wrong-opening control did not change its input');
	filesystem.writeFileSync(checkedFile, changed);
	const finding = execute('run-finding', ['--all', '--format', 'json'], 1);
	const findingReport = z
		.object({
			status: z.object({ kind: z.literal('findings') }),
			findings: z.array(z.unknown()).min(1),
		})
		.parse(JSON.parse(finding));
	requireSdk(
		findingReport.findings.length > 0,
		'Guide wrong-opening control did not find a violation',
	);
	return [
		'CLI help distinguishes admission, evidence, preparation and reports',
		'complete guide files copied from the pinned source and admitted by the CLI',
		'guide evidence, clean run, prepared exit and wrong-opening finding',
	];
}
