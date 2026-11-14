// Install packed candidate tarballs into a fresh consumer outside the repository,
// type-check the package examples there, and run real load/check/run.
//
//   node scripts/installed-consumer-check.ts --tarballs <absolute-directory> [--evidence <absolute-directory>]
//   node scripts/installed-consumer-check.ts --print-contract --evidence <consumer> --expect-cell <key>
//
// The consumer gets only the tarballs plus TypeScript and Node types from the
// registry: no workspace links, no Rust toolchain use and no Zod.
import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import {
	copyFileSync,
	cpSync,
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

/** Run one step, keeping its command, output and exit in `<label>.log` even when it fails. */
function step({
	label,
	command,
	args,
	expectedExit = 0,
}: {
	readonly label: string;
	readonly command: string;
	readonly args: readonly string[];
	readonly expectedExit?: number;
}): string {
	process.stdout.write(`\n$ ${command} ${args.join(' ')}\n`);
	const result = spawnSync(command, args, {
		cwd: consumer,
		encoding: 'utf8',
		env: keyless,
		stdio: ['ignore', 'pipe', 'pipe'],
	});
	writeFileSync(
		join(consumer, `${label}.log`),
		`$ ${command} ${args.join(' ')}\n--- stdout\n${result.stdout}\n--- stderr\n${result.stderr}\n--- exit ${String(result.status)} signal ${String(result.signal)} error ${String(result.error)}\n`,
	);
	assert.equal(result.error, undefined, `${label}: process could not start`);
	assert.equal(result.signal, null, `${label}: process ended by signal`);
	assert.equal(result.status, expectedExit, `${label}: ${result.stderr}\n${result.stdout}`);
	process.stdout.write(result.stdout);
	return result.stdout;
}

writeFileSync(
	join(consumer, 'package.json'),
	`${JSON.stringify({ name: 'sdk-consumer', private: true, type: 'module', overrides: platformOverrides }, null, 2)}\n`,
);
step({
	label: 'install',
	command: 'npm',
	args: [
		'install',
		'--no-audit',
		'--no-fund',
		'--save-exact',
		join(tarballDirectory, sdkTarball),
		'typescript@7.0.2',
		'@types/node@26.6.5',
	],
});

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

const exampleFiles = [
	'inline-configuration.ts',
	'load-and-edit.ts',
	'mechanical-run.ts',
	'heading-fields.ts',
	'inspect-and-save.ts',
] as const;
for (const example of exampleFiles) {
	copyFileSync(
		join(consumer, 'node_modules', '@sunstack', 'sdk', 'examples', example),
		join(consumer, example),
	);
}
copyFileSync(
	join(packageDirectory, 'tests', 'installed-cases', 'markdown-islands.ts'),
	join(consumer, 'markdown-islands.ts'),
);
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
step({
	label: 'typecheck',
	command: join(consumer, 'node_modules', '.bin', 'tsc'),
	args: ['-p', 'tsconfig.json'],
});

// Exercise the shipped standalone instruction with this consumer's tsconfig
// present. Project-only compilation cannot catch a broken README command.
const installedReadme = readFileSync(
	join(consumer, 'node_modules', '@sunstack', 'sdk', 'README.md'),
	'utf8',
);
const documentedCompilerArguments = /^npx tsc (.+)$/mu.exec(installedReadme)?.[1];
assert.ok(
	documentedCompilerArguments !== undefined,
	'README advertises its standalone compile command',
);
assert.deepEqual(
	new Set(
		[...installedReadme.matchAll(/examples\/([a-z-]+\.ts)/gu)].flatMap((match) =>
			match[1] === undefined ? [] : [match[1]],
		),
	),
	new Set(exampleFiles),
	'every advertised example is exercised from packed bytes',
);
step({
	label: 'readme-typecheck',
	command: join(consumer, 'node_modules', '.bin', 'tsc'),
	args: documentedCompilerArguments.split(/\s+/u),
});
step({
	label: 'configured-markdown-islands',
	command: process.execPath,
	args: ['markdown-islands.ts'],
});

// Heading pages: a non-ASCII setext heading and an ATX child keep their Markdown source; HTML
// entities are decoded exactly once.
const descriptionPage = '---\ndescription: Wrong opening\n---\n\n# One\n';
const headingGuide = 'Café *guide* &amp;\n===\n\nIntro.\n\n## Install `npm` ##\n\nSteps.\n';
const headingPage =
	'猫<h2>Review &lt;Widget&gt; &amp;lt;</h2><h3>1. <em>Read</em> addressed source</h3><p>Body</p>';

/** `start-end`: the UTF-8 byte range of `fragment` in `page`. */
function headingBytes(page: string, fragment: string): string {
	const start = Buffer.from(page).indexOf(Buffer.from(fragment));
	assert.ok(start >= 0, fragment);
	return `${start}-${start + Buffer.byteLength(fragment)}`;
}

// A synthetic checked repository and an existing CLI-style configuration project.
const checked = join(consumer, 'checked');
const project = join(consumer, 'project');
for (const [path, content] of Object.entries({
	[join(checked, 'docs', 'one.md')]: descriptionPage,
	[join(checked, 'docs', 'guide.md')]: headingGuide,
	[join(checked, 'docs', 'page.html')]: headingPage,
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

const mechanical = step({
	label: 'mechanical-run-findings',
	command: process.execPath,
	args: ['mechanical-run.ts', checked],
	expectedExit: 1,
});
assert.match(mechanical, /contract: release \S+ revision \d+ target \S+/u);
assert.match(mechanical, /check: valid/u);
assert.match(mechanical, /run: findings 1/u);
const edited = step({
	label: 'load-and-edit-findings',
	command: process.execPath,
	args: ['load-and-edit.ts', project, checked],
	expectedExit: 1,
});
assert.match(edited, /check: valid/u);
assert.match(edited, /run: completed/u);
assert.match(edited, /source edited/u);
// Heading fields execute on real sections: exact own and enclosing headings, outermost first.
const headings = step({
	label: 'heading-fields',
	command: process.execPath,
	args: ['heading-fields.ts', checked],
});
assert.match(headings, /check: valid/u);
assert.match(headings, /run: clean/u);
const guideHeading = 'Café *guide* &amp;';
const pageHeading = 'Review <Widget> &lt;';
const headingRows = (file: string): string[] =>
	headings
		.split('\n')
		.filter((line) => line.startsWith(`${file}: `))
		.toSorted();
// Exactly these rows per file: an extra, missing or duplicated row fails.
assert.deepEqual(
	headingRows('docs/guide.md'),
	[
		`docs/guide.md: ${guideHeading} | ${guideHeading} @ ${headingBytes(headingGuide, 'Café *guide* &amp;\n===\n')}`,
		`docs/guide.md: Install \`npm\` | ${guideHeading} > Install \`npm\` @ ${headingBytes(headingGuide, '## Install `npm` ##\n')}`,
	].toSorted(),
);
assert.deepEqual(
	headingRows('docs/page.html'),
	[
		`docs/page.html: ${pageHeading} | ${pageHeading} @ ${headingBytes(headingPage, '<h2>Review &lt;Widget&gt; &amp;lt;</h2>')}`,
		`docs/page.html: 1. Read addressed source | ${pageHeading} > 1. Read addressed source @ ${headingBytes(headingPage, '<h3>1. <em>Read</em> addressed source</h3>')}`,
	].toSorted(),
);
// The description page's own section is reported too, separately.
assert.deepEqual(headingRows('docs/one.md'), [
	`docs/one.md: One | One @ ${headingBytes(descriptionPage, '# One\n')}`,
]);
const inline = step({
	label: 'inline-configuration-keyless-refusal',
	command: process.execPath,
	args: ['inline-configuration.ts', checked],
	expectedExit: 1,
});
assert.match(inline, /check: valid/u);
// Supply the mechanical recipe's favorable input, then prove empty/refused input
// cannot look like a successful example process. These run only packed sources.
writeFileSync(
	join(checked, 'docs', 'one.md'),
	descriptionPage.replace('Wrong opening', 'Use when checking documents'),
);
step({
	label: 'mechanical-run-clean',
	command: process.execPath,
	args: ['mechanical-run.ts', checked],
});
step({
	label: 'load-and-edit-clean',
	command: process.execPath,
	args: ['load-and-edit.ts', project, checked],
});
const empty = join(consumer, 'empty');
mkdirSync(empty);
step({
	label: 'mechanical-run-empty',
	command: process.execPath,
	args: ['mechanical-run.ts', empty],
	expectedExit: 1,
});
step({
	label: 'heading-fields-empty',
	command: process.execPath,
	args: ['heading-fields.ts', empty],
	expectedExit: 1,
});
mkdirSync(join(checked, 'src'));
writeFileSync(join(checked, 'src', 'one.ts'), "// café\nconsole.log('one');\n");
const saved = join(consumer, 'inspection.sdk.json');
const inspection = step({
	label: 'inspect-and-save',
	command: process.execPath,
	args: ['inspect-and-save.ts', checked, saved],
});
assert.match(inspection, /rules were not evaluated/u);
assert.match(inspection, /restored exact typed outcome/u);
assert.match(inspection, /revision refused/u);
assert.ok(readFileSync(saved, 'utf8').includes('"contract_revision"'));
step({
	label: 'inspect-and-save-empty',
	command: process.execPath,
	args: ['inspect-and-save.ts', empty, saved],
	expectedExit: 1,
});

// A sibling consumer cannot inherit the main consumer's optional native addon.
// Its wrapper and examples are byte-for-byte copies of the same installed tarball.
const missingAddonConsumer = `${consumer}-missing-addon`;
mkdirSync(missingAddonConsumer);
mkdirSync(join(missingAddonConsumer, 'node_modules', '@sunstack'), { recursive: true });
writeFileSync(join(missingAddonConsumer, 'package.json'), '{"private":true,"type":"module"}\n');
cpSync(
	join(consumer, 'node_modules', '@sunstack', 'sdk'),
	join(missingAddonConsumer, 'node_modules', '@sunstack', 'sdk'),
	{ recursive: true, force: false, errorOnExist: true },
);
const missingAddonRequire = createRequire(join(missingAddonConsumer, 'package.json'));
for (const cell of platformCells) {
	assert.throws(
		() => missingAddonRequire.resolve(`@sunstack/sdk-${cell.key}`),
		/Cannot find module/u,
	);
}
for (const example of exampleFiles) {
	const source = join(consumer, 'node_modules', '@sunstack', 'sdk', 'examples', example);
	const copied = join(missingAddonConsumer, example);
	copyFileSync(source, copied);
	assert.deepEqual(readFileSync(copied), readFileSync(source));
	const label = `missing-addon-${example.slice(0, -3)}`;
	const exampleArguments = example === 'load-and-edit.ts' ? [project, checked] : [checked];
	step({ label, command: process.execPath, args: [copied, ...exampleArguments], expectedExit: 1 });
	const receipt = readFileSync(join(consumer, `${label}.log`), 'utf8');
	assert.match(receipt, /addon_missing/u);
	assert.match(receipt, /before_dispatch/u);
	assert.match(receipt, /effects: 'none'/u);
	assert.doesNotMatch(receipt, /run: completed|run: clean|restored exact typed outcome/u);
}
process.stdout.write(`\nInstalled consumer passed in ${consumer}\n`);
