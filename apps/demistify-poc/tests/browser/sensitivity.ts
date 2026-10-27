import { spawn } from 'node:child_process';
import { cp, mkdtemp, readFile, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const appRoot = fileURLToPath(new URL('../../', import.meta.url));
const resultsRoot = await mkdtemp(join(tmpdir(), '2026-10-demistify-sensitivity-'));
const baseline = join(resultsRoot, 'baseline');
await cp(
	process.env['DEMISTIFY_BROWSER_ASSETS'] ??
		fileURLToPath(new URL('../../../../packages/demistify/dist', import.meta.url)),
	baseline,
	{ recursive: true },
);

interface SensitivityCase {
	readonly name: string;
	readonly testName: string;
	readonly css?: string;
	readonly section?: string;
	readonly pattern?: RegExp;
	readonly replacement?: string;
	readonly expectedFailures?: number;
}

const cases: readonly SensitivityCase[] = [
	{
		name: 'hidden-markdown',
		testName: 'external and embedded Markdown render',
		css: '#notes > h1, #notes table, #notes ex-mermaid { display:none !important; }',
	},
	{
		name: 'truncated-source',
		testName: 'Show Markdown source displays',
		section: 'src/islands/markdown.ts',
		pattern: /(\w+)\.textContent = (\w+)\.text(?=[,;])/g,
		replacement: '$1.textContent = "Source unexpectedly truncated"',
	},
	{
		name: 'multiple-presentations',
		testName: 'presentation menu shows',
		css: 'ex-presentation[hidden] { display:block !important; }',
	},
	{
		name: 'unrevealed-fragment',
		testName: 'initial fragments reveal',
		section: 'src/layout/document.ts',
		pattern: /let (\w+) = (\w+) \? document\.getElementById\(\2\) : null;/g,
		replacement: 'let $1 = null;',
	},
	{
		name: 'clipped-diagram-start',
		testName: 'wide diagrams scroll locally',
		css: 'ex-mermaid > svg { transform:translateX(-200px) !important; }',
	},
	{
		name: 'tree-label-under-icon',
		testName: 'tree multiline text stays',
		css: '.ex-tree-node { flex-wrap:wrap !important; } .ex-tree-label { min-width:100% !important; }',
	},
	{
		name: 'moved-sidebar-original',
		testName: 'replacing sidebar content keeps',
		section: 'src/islands/inspector.ts',
		pattern: /let (\w+) = (\w+) \? (\w+) : (\w+)\(\3\);/g,
		replacement: 'let $1 = $3;',
	},
	{
		name: 'disabled-modifier-zoom',
		testName: 'inspection modifier wheel zooms',
		section: 'src/islands/inspection-gestures.ts',
		pattern: /if \(!\w+\.metaKey && !\w+\.ctrlKey\) return;/g,
		replacement: 'return;',
	},
];

async function checkSensitivity(scenario: SensitivityCase): Promise<void> {
	const assets = join(resultsRoot, scenario.name);
	await cp(baseline, assets, { recursive: true });
	if (scenario.css) {
		const file = join(assets, 'theme.css');
		await writeFile(file, (await readFile(file, 'utf8')) + '\n' + scenario.css);
	} else {
		const file = join(assets, 'runtime.js');
		const source = await readFile(file, 'utf8');
		const start = source.indexOf(`//#region ${scenario.section}`);
		const end = source.indexOf('//#endregion', start);
		if (start < 0 || end < 0 || !scenario.pattern || !scenario.replacement)
			throw new Error(`Missing mutation boundary: ${scenario.name}`);
		const section = source.slice(start, end);
		const matches = section.match(scenario.pattern);
		if (matches?.length !== 1)
			throw new Error(`Mutation must match once: ${scenario.name}, found ${matches?.length ?? 0}`);
		await writeFile(
			file,
			source.slice(0, start) +
				section.replace(scenario.pattern, scenario.replacement) +
				source.slice(end),
		);
	}
	const child = spawn(
		'pnpm',
		[
			'exec',
			'vitest',
			'run',
			'--config',
			'vitest.browser.config.ts',
			'--testNamePattern',
			scenario.testName,
		],
		{
			cwd: appRoot,
			env: { ...process.env, DEMISTIFY_BROWSER_ASSETS: assets, NO_COLOR: '1' },
			stdio: ['ignore', 'pipe', 'pipe'],
			timeout: 45_000,
		},
	);
	let output = '';
	child.stdout.on('data', (chunk: Buffer): void => {
		output += chunk.toString();
	});
	child.stderr.on('data', (chunk: Buffer): void => {
		output += chunk.toString();
	});
	const exitCode = await new Promise<number | null>((resolve, reject): void => {
		child.once('error', reject);
		child.once('exit', resolve);
	});
	await writeFile(join(resultsRoot, `${scenario.name}.log`), output);
	const expectedFailures = scenario.expectedFailures ?? 1;
	if (
		exitCode !== 1 ||
		!new RegExp(`Tests\\s+${expectedFailures} failed`).test(output) ||
		!/(AssertionError|Matcher did not succeed in time)/.test(output)
	)
		throw new Error(
			`Sensitivity did not produce the intended assertion failure: ${scenario.name}; see ${resultsRoot}`,
		);
	process.stdout.write(
		`Caught ${scenario.name}: ${expectedFailures} expected test failure(s), exit 1\n`,
	);
}

// Sequential sessions keep browser resource usage bounded; every mutation owns a separate copy.
const selectedCases: readonly SensitivityCase[] = process.argv.includes('--native-gestures')
	? [
			{
				name: 'disabled-native-modifier-zoom',
				testName: 'native (picture|Mermaid) gestures',
				section: 'src/islands/inspection-gestures.ts',
				pattern: /if \(!\w+\.metaKey && !\w+\.ctrlKey\) return;/g,
				replacement: 'return;',
				expectedFailures: 2,
			},
		]
	: cases;
await selectedCases.reduce<Promise<void>>(
	(previous, scenario): Promise<void> =>
		previous.then((): Promise<void> => checkSensitivity(scenario)),
	Promise.resolve(),
);
process.stdout.write(`Sensitivity logs: ${resultsRoot}\n`);
