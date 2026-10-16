import { execFileSync } from 'node:child_process';
import { appendFile, mkdtemp, readFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { z } from 'zod';

import { compareVersions, versionSchema } from './release-schema.ts';

const base = process.env['DEMISTIFY_BASE_REF'] ?? 'origin/main';
if (base !== 'origin/main' && !/^[a-f0-9]{40}$/.test(base))
	throw new Error('Base must be origin/main or a complete commit SHA.');
const changed = execFileSync('git', ['diff', '--name-only', `${base}...HEAD`], { encoding: 'utf8' })
	.trim()
	.split('\n');
const publicSourceChanges = changed.some(
	(path): boolean =>
		/^packages\/demistify\/(?:src|skill|scripts)\//.test(path) && !path.endsWith('.test.ts'),
);
const packageSchema = z.object({ version: versionSchema }).passthrough();
const before = packageSchema.parse(
	JSON.parse(
		execFileSync('git', ['show', `${base}:packages/demistify/package.json`], { encoding: 'utf8' }),
	),
);
const after = packageSchema.parse(
	JSON.parse(await readFile('packages/demistify/package.json', 'utf8')),
);
const metadataWithoutVersion = (metadata: Record<string, unknown>): string =>
	JSON.stringify(
		Object.fromEntries(
			Object.entries(metadata)
				.filter(([name]): boolean => name !== 'version')
				.toSorted(([left], [right]): number => left.localeCompare(right)),
		),
	);
const publicChanges =
	publicSourceChanges || metadataWithoutVersion(before) !== metadataWithoutVersion(after);
if (publicChanges && after.version !== before.version)
	throw new Error('Keep public source changes and the consumed version-only PR separate.');
if (compareVersions(after.version, before.version) < 0)
	throw new Error('A package version cannot go backwards.');
const versionPullRequest = after.version !== before.version && !publicChanges;
if (versionPullRequest) {
	process.stdout.write(
		`Version PR: ${before.version} → ${after.version}; no public source changes.\n`,
	);
} else if (publicChanges) {
	const directory = await mkdtemp(join(tmpdir(), 'demistify-release-intent-'));
	const output = join(directory, 'plan.json');
	const cli = join(
		dirname(fileURLToPath(import.meta.resolve('@changesets/cli/package.json'))),
		'bin.js',
	);
	execFileSync(process.execPath, [cli, 'status', '--since', base, '--output', output], {
		stdio: 'inherit',
	});
	const plan = z
		.object({
			releases: z.array(
				z.object({
					name: z.string(),
					type: z.string(),
					newVersion: versionSchema,
				}),
			),
		})
		.parse(JSON.parse(await readFile(output, 'utf8')));
	const release = plan.releases.find((entry): boolean => entry.name === '@sunstack/demistify');
	if (!release || !['major', 'minor', 'patch'].includes(release.type))
		throw new Error(
			'Public library/guide changes need a Changeset declaring @sunstack/demistify patch, minor or major.',
		);
	process.stdout.write(
		`Public library declaration: ${release.type} → ${release.newVersion}; reviewers must confirm semantic compatibility.\n`,
	);
} else process.stdout.write('No public library/guide change; no release declaration required.\n');
const output = process.env['GITHUB_OUTPUT'];
if (output) await appendFile(output, `version-pr=${versionPullRequest}\n`);
