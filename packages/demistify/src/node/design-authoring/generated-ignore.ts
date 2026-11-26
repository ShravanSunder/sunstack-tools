import { execFile } from 'node:child_process';
import { copyFile, lstat, mkdir, mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { basename, dirname, join, relative, resolve } from 'node:path';
import { promisify } from 'node:util';

import { designRecordsFilename } from '../../core/design-record-view.ts';
import { containsPath } from '../local-site.ts';
import {
	gitOutputRoot,
	isTrackedOutput,
	readOutputFile,
	type GeneratedFile,
} from './design-output.ts';
import { findRegisteredInitiative } from './read-initiative.ts';

const patterns = [
	'*.generated.json',
	'design-sdk.generated.ts',
	'design-schema.d.ts',
	designRecordsFilename,
];
const designOutputs = [
	'design.generated.json',
	'navigation.generated.json',
	'deliverables.generated.json',
	...patterns.slice(1),
];
const run = promisify(execFile);

export function projectIgnoreDirectory(project: string): string {
	const parent = dirname(project);
	return basename(parent) === 'projects' ? parent : project;
}

export async function designIgnoreDirectory(directory: string): Promise<string> {
	const project = await findRegisteredInitiative(directory);
	return project ? projectIgnoreDirectory(project.directory) : directory;
}

export function generatedDesignPaths(
	designDirectories: readonly string[],
	additional: readonly string[] = [],
): string[] {
	return [
		...new Set([
			...designDirectories.flatMap((directory) =>
				designOutputs.map((name) => join(directory, name)),
			),
			...additional,
		]),
	];
}

interface IgnoreMatch {
	readonly file: string;
	readonly ignore: string;
	readonly excluded: boolean;
}

function ignoreMatches(root: string, paths: readonly string[]): Promise<IgnoreMatch[]> {
	return new Promise((done, reject) => {
		const git = execFile(
			'git',
			[
				'-c',
				'core.excludesFile=/dev/null',
				'check-ignore',
				'--no-index',
				'--verbose',
				'--stdin',
				'-z',
			],
			{ cwd: root },
			(error, stdout) => {
				if (error && error.code !== 1) return reject(error);
				const fields = stdout.split('\0');
				const matches: IgnoreMatch[] = [];
				for (let index = 0; index + 3 < fields.length; index += 4) {
					const source = fields[index];
					const pattern = fields[index + 2];
					const file = fields[index + 3];
					if (source && pattern && file)
						matches.push({
							file,
							ignore: resolve(root, source),
							excluded: !pattern.startsWith('!'),
						});
				}
				done(matches);
			},
		);
		git.stdin?.end(paths.map((file) => relative(root, file) + '\0').join(''));
	});
}

async function hasAncestorCoverage(
	root: string,
	directory: string,
	files: readonly string[],
): Promise<boolean> {
	// Git reports only the winning rule. Recheck ancestor rules alone when child rules mask them.
	const scratch = await mkdtemp(join(tmpdir(), 'demistify-ignore-'));
	const directories: string[] = [];
	for (let candidate = directory; ; candidate = dirname(candidate)) {
		directories.push(candidate);
		if (candidate === root) break;
	}
	try {
		await run('git', ['init', '--quiet', '--template='], { cwd: scratch });
		await mkdir(join(scratch, relative(root, directory)), { recursive: true });
		await Promise.all(
			directories.map(async (source) => {
				const ignore = join(source, '.gitignore');
				try {
					if ((await lstat(ignore)).isFile())
						await copyFile(ignore, join(scratch, relative(root, ignore)));
				} catch (error) {
					if (!(error instanceof Error && 'code' in error && error.code === 'ENOENT')) throw error;
				}
			}),
		);
		const mapped = files.map((file) => join(scratch, relative(root, file)));
		const matches = await ignoreMatches(scratch, mapped);
		return matches.length === files.length && matches.every(({ excluded }) => excluded);
	} finally {
		await rm(scratch, { force: true, recursive: true });
	}
}

export async function planDesignIgnore(
	directory: string,
	generated: readonly string[],
): Promise<GeneratedFile | undefined> {
	const file = join(directory, '.gitignore');
	const root = await gitOutputRoot(directory);
	if (generated.some((output) => !containsPath(directory, output)))
		throw new Error('Generated design files must be inside the ignore scope: ' + directory);
	const block = patterns.join('\n') + '\n';
	if (root) {
		const matches = await ignoreMatches(root, generated);
		const childRule = (ignore: string): boolean =>
			ignore !== file && basename(ignore) === '.gitignore' && containsPath(directory, ignore);
		for (const { excluded, ignore, file: output } of matches) {
			if (!excluded && childRule(ignore))
				throw new Error(
					`Generated output is unignored by ${ignore}: ${output}. Remove that exception before writing.`,
				);
		}
		if (matches.length === generated.length && matches.every(({ excluded }) => excluded)) {
			if (
				matches.every(
					({ ignore }) =>
						basename(ignore) === '.gitignore' &&
						containsPath(root, ignore) &&
						containsPath(dirname(ignore), directory),
				)
			)
				return undefined;
			if (
				matches.some(({ ignore }) => childRule(ignore)) &&
				(await hasAncestorCoverage(root, directory, generated))
			)
				return undefined;
		}
		if (directory === root)
			throw new Error(
				`Add the generated design ignore patterns to repository-root .gitignore at ${file} before writing: ${patterns.join(', ')}.`,
			);
		if (await isTrackedOutput(file))
			throw new Error(
				`Add the generated design ignore patterns to tracked ${file} before writing: ${patterns.join(', ')}.`,
			);
	}
	const previous = await readOutputFile(file, 'regular .gitignore file');
	if (!root && previous?.endsWith(block)) return undefined;
	const prefix = previous && !previous.endsWith('\n') ? previous + '\n' : (previous ?? '');
	return { file, previous, text: prefix + block };
}
