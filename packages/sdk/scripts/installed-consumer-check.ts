// Install packed candidate tarballs into a fresh consumer outside the repository,
// type-check the package examples there, and run real load/check/run.
//
//   node scripts/installed-consumer-check.ts --tarballs <absolute-directory> [--evidence <absolute-directory>]
//   node scripts/installed-consumer-check.ts --print-contract --evidence <consumer> --expect-cell <key>
//
// The consumer gets only the tarballs plus TypeScript and Node types from the
// registry: no workspace links, no Rust toolchain use and no Zod.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import {
	copyFileSync,
	mkdirSync,
	mkdtempSync,
	readdirSync,
	readFileSync,
	realpathSync,
	writeFileSync,
} from 'node:fs';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import { isAbsolute, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';

import { CONTRACT_REVISION } from '../src/contracts/contract-revision.ts';
import { platformCells } from '../src/native-binding.ts';

const packageDirectory = fileURLToPath(new URL('../', import.meta.url));

function sdkVersion(): string {
	const manifest: unknown = JSON.parse(
		readFileSync(join(packageDirectory, 'package.json'), 'utf8'),
	);
	if (
		typeof manifest !== 'object' ||
		manifest === null ||
		!('version' in manifest) ||
		typeof manifest.version !== 'string'
	) {
		throw new Error('package.json has no version');
	}
	return manifest.version;
}
const { values } = parseArgs({
	options: {
		tarballs: { type: 'string' },
		evidence: { type: 'string' },
		'print-contract': { type: 'boolean' },
		'expect-cell': { type: 'string' },
	},
});

// Second mode: report the installed addon's contract and require this cell's target.
if (values['print-contract'] === true) {
	const consumerDirectory = values.evidence;
	const cellKey = values['expect-cell'];
	if (consumerDirectory === undefined || cellKey === undefined) {
		throw new Error('Pass --evidence <consumer> and --expect-cell <key>.');
	}
	const expected = platformCells.find((cell) => cell.key === cellKey);
	if (expected === undefined) throw new Error(`Unknown cell ${cellKey}.`);
	const probe = join(consumerDirectory, 'contract-probe.mjs');
	writeFileSync(
		probe,
		"import { contractInfo } from '@sunstack/sdk';\nconsole.log(JSON.stringify(await contractInfo()));\n",
	);
	const output = execFileSync(process.execPath, [probe], {
		cwd: consumerDirectory,
		encoding: 'utf8',
	});
	writeFileSync(join(consumerDirectory, 'contract.log'), output);
	process.stdout.write(output);
	const reported: unknown = JSON.parse(output);
	assert.deepEqual(reported, {
		kind: 'ready',
		contract: {
			sdk_release: sdkVersion(),
			contract_revision: CONTRACT_REVISION,
			target: expected.target,
		},
	});
	const installed = readdirSync(join(consumerDirectory, 'node_modules', '@sunstack')).toSorted();
	assert.deepEqual(
		installed,
		['sdk', `sdk-${cellKey}`],
		"exactly this cell's addon package is installed",
	);
	process.exit(0);
}
const tarballDirectory = values.tarballs;
if (tarballDirectory === undefined || !isAbsolute(tarballDirectory))
	throw new Error('Pass --tarballs <absolute-directory>.');

const consumer = values.evidence ?? mkdtempSync(join(tmpdir(), 'sunstack-sdk-consumer-'));
mkdirSync(consumer, { recursive: true });
const tarballs = readdirSync(tarballDirectory).filter((name) => name.endsWith('.tgz'));
const sdkTarball = tarballs.find((name) => /^sunstack-sdk-\d/u.test(name));
assert.ok(sdkTarball !== undefined, 'the @sunstack/sdk tarball is present');
// Each platform package resolves to its candidate tarball, so npm itself selects
// the cell exactly as it would from a registry; non-matching cells are skipped.
const platformOverrides = Object.fromEntries(
	platformCells.flatMap((cell) => {
		const tarball = tarballs.find((name) => name.startsWith(`sunstack-sdk-${cell.key}-`));
		return tarball === undefined
			? []
			: [[`@sunstack/sdk-${cell.key}`, `file:${join(tarballDirectory, tarball)}`]];
	}),
);

// Examples run keyless: no provider request can be made from this check.
const keyless: NodeJS.ProcessEnv = { ...process.env, SUNSTACK_CREDENTIAL_STORE: 'off' };
delete keyless['OPENROUTER_API_KEY'];

function step(label: string, command: string, args: readonly string[]): string {
	process.stdout.write(`\n$ ${command} ${args.join(' ')}\n`);
	const output = execFileSync(command, args, {
		cwd: consumer,
		encoding: 'utf8',
		env: keyless,
		stdio: ['ignore', 'pipe', 'inherit'],
	});
	process.stdout.write(output);
	writeFileSync(join(consumer, `${label}.log`), output);
	return output;
}

writeFileSync(
	join(consumer, 'package.json'),
	`${JSON.stringify({ name: 'sdk-consumer', private: true, type: 'module', overrides: platformOverrides }, null, 2)}\n`,
);
step('install', 'npm', [
	'install',
	'--no-audit',
	'--no-fund',
	'--save-exact',
	join(tarballDirectory, sdkTarball),
	'typescript@7.0.2',
	'@types/node@26.6.5',
]);

// No Zod and no workspace links: the installed tree is all there is.
const consumerRequire = createRequire(join(consumer, 'package.json'));
assert.throws(
	() => consumerRequire.resolve('zod'),
	/Cannot find module/u,
	'zod must not be installed',
);
const sdkEntry = consumerRequire.resolve('@sunstack/sdk');
assert.ok(
	realpathSync(sdkEntry).startsWith(join(realpathSync(consumer), 'node_modules')),
	`the SDK resolves inside the consumer: ${sdkEntry}`,
);

for (const example of ['inline-configuration.ts', 'load-and-edit.ts', 'mechanical-run.ts']) {
	copyFileSync(join(packageDirectory, 'examples', example), join(consumer, example));
}
writeFileSync(
	join(consumer, 'tsconfig.json'),
	`${JSON.stringify(
		{
			compilerOptions: {
				target: 'ES2024',
				module: 'NodeNext',
				moduleResolution: 'NodeNext',
				lib: ['ES2024'],
				types: ['node'],
				strict: true,
				exactOptionalPropertyTypes: true,
				noUncheckedIndexedAccess: true,
				noEmit: true,
				skipLibCheck: false,
			},
			include: ['*.ts'],
		},
		null,
		2,
	)}\n`,
);
step('typecheck', join(consumer, 'node_modules', '.bin', 'tsc'), ['-p', 'tsconfig.json']);

// A synthetic checked repository and an existing CLI-style configuration project.
const checked = join(consumer, 'checked');
const project = join(consumer, 'project');
for (const [path, content] of Object.entries({
	[join(checked, 'docs', 'one.md')]: '---\ndescription: Wrong opening\n---\n\n# One\n',
	[join(project, 'sunstack.lint.json')]: JSON.stringify({
		rulebook: [{ markdown: 'rulebook.md', rules: 'rules.json' }],
		marker_prefix: 'K',
		check: { files: ['docs/*.md'] },
		evidence: {
			ast_grep: 'evidence/sgconfig.yml',
			kinds: {
				skill: { rule: 'skill', fields: { description: { from: 'yaml', key: 'description' } } },
			},
		},
		decision_model: { model: 'jev-latest' },
	}),
	[join(project, 'rulebook.md')]: 'Description must begin with Use when. <!-- K001 -->\n',
	[join(project, 'rules.json')]: JSON.stringify({
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
						no: { finding: 'Description must begin with Use when.', at: 'description' },
					},
				},
			},
		},
	}),
	[join(project, 'evidence', 'sgconfig.yml')]: 'ruleDirs: [rules]\n',
	[join(project, 'evidence', 'rules', 'skill.yml')]:
		'id: skill\nlanguage: Markdown\nrule: {matches: frontmatter}\n',
})) {
	mkdirSync(join(path, '..'), { recursive: true });
	writeFileSync(path, content);
}

const mechanical = step('mechanical-run', process.execPath, ['mechanical-run.ts', checked]);
assert.match(mechanical, /contract: release \S+ revision \d+ target \S+/u);
assert.match(mechanical, /check: valid/u);
assert.match(mechanical, /run: findings 1/u);
const edited = step('load-and-edit', process.execPath, ['load-and-edit.ts', project, checked]);
assert.match(edited, /check: valid/u);
assert.match(edited, /run: completed/u);
assert.match(edited, /source edited/u);
const inline = step('inline-configuration', process.execPath, ['inline-configuration.ts', checked]);
assert.match(inline, /check: valid/u);
process.stdout.write(`\nInstalled consumer passed in ${consumer}\n`);
