import { execFile } from 'node:child_process';
import { cp, mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { promisify } from 'node:util';
import { gzipSync } from 'node:zlib';

import { comparisonReport } from '../src/experiment-builders/comparison-report.ts';
import type {
	PrototypeMeasurement,
	SharedAssetMeasurement,
} from '../src/experiment-builders/comparison-report.ts';
import {
	applyGraphChanges,
	changeSetSchema,
	graphSchema,
	graphToD2,
	graphToMermaid,
	overlayGraph,
} from '../src/experiment-builders/graph-changes.ts';
import { escapeHtml, htmlDocument } from '../src/experiment-builders/html-document.ts';
import { prepareIslandContent } from '../src/experiment-builders/islands-packager.ts';
import {
	planningD2,
	planningViewsSchema,
	themedPlanningMermaid,
} from '../src/experiment-builders/planning-fixture.ts';
import { applySvgTheme } from '../src/experiment-builders/svg-theme.ts';
import type { AuthorUsageFile } from '../src/experiment-builders/usage-measurement.ts';

const executeFile = promisify(execFile);
const outputDirectory = 'public/experiments';

async function referencedLibraryChunks(
	sourceTexts: readonly string[],
	availableChunkNames: readonly string[],
	visitedChunks: Set<string>,
): Promise<readonly SharedAssetMeasurement[]> {
	const chunkNames = availableChunkNames.filter(
		(name): boolean =>
			!visitedChunks.has(name) && sourceTexts.some((source): boolean => source.includes(name)),
	);
	for (const name of chunkNames) visitedChunks.add(name);
	if (chunkNames.length === 0) return [];
	const chunks = await Promise.all(
		chunkNames.map(async (name) => ({
			name,
			source: await readFile(`${outputDirectory}/islands/chunks/${name}`, 'utf8'),
		})),
	);
	const dependencies = await referencedLibraryChunks(
		chunks.map((chunk): string => chunk.source),
		availableChunkNames,
		visitedChunks,
	);
	return [
		...chunks.map((chunk): SharedAssetMeasurement => ({
			name: chunk.name,
			bytes: Buffer.byteLength(chunk.source),
		})),
		...dependencies,
	];
}
await mkdir(`${outputDirectory}/islands`, { recursive: true });
const viewsSource = await readFile('fixtures/planning-views.json', 'utf8');
const views = planningViewsSchema.parse(JSON.parse(viewsSource));
const baseSource = await readFile('fixtures/planning-base.json', 'utf8');
const changeSource = await readFile('experiments/graph-diff/planning-changes.json', 'utf8');
const baseGraph = graphSchema.parse(JSON.parse(baseSource));
const changeSet = changeSetSchema.parse(JSON.parse(changeSource));
if (changeSet.base !== 'fixtures/planning-base.json')
	throw new Error('The planning corpus has one canonical base.');
const proposedGraph = applyGraphChanges(baseGraph, changeSet);
const libraryDirectory = resolve('../../packages/demistify/dist');
await cp(libraryDirectory, `${outputDirectory}/islands`, { recursive: true });
const themeCss = await readFile(`${libraryDirectory}/theme.css`, 'utf8');
const darkThemeBlock = /:root\[data-theme=['"]dark['"]\]\s*\{([\s\S]*?)\}/.exec(themeCss)?.[1];
if (!darkThemeBlock) throw new Error('The diagram theme must come from the shared CSS tokens.');
const diagramColors = Object.fromEntries(
	Array.from(
		darkThemeBlock.matchAll(/--([\w-]+):\s*([^;]+);/g),
		(match): readonly [string, string] => [match[1] ?? '', match[2]?.trim() ?? ''],
	),
);
const graphThemeColors = Object.fromEntries(
	Object.entries(diagramColors).filter(
		([name]) => name === 'ink' || name === 'sheet' || name.startsWith('diff-'),
	),
);
const diagramReferences: Record<string, string> = {};
for (const name of ['roadmap', 'design', 'roles'])
	diagramReferences[`fixtures/${name}.mmd`] = themedPlanningMermaid(
		views[name === 'roadmap' ? 'roadmap' : name === 'design' ? 'design' : 'roles'],
	);
diagramReferences['fixtures/current.mmd'] = graphToMermaid(baseGraph);
diagramReferences['fixtures/diff.mmd'] = graphToMermaid(
	overlayGraph(baseGraph, changeSet),
	changeSet,
);
diagramReferences['fixtures/proposed.mmd'] = graphToMermaid(proposedGraph);
// These two references belong only to the separate capability showcase.
diagramReferences['fixtures/baseline.mmd'] = diagramReferences['fixtures/current.mmd'] ?? '';
diagramReferences['fixtures/proposal.mmd'] = diagramReferences['fixtures/diff.mmd'] ?? '';

// The demo reuses the independently built component library.
const iconFontBytes = await readFile(`${libraryDirectory}/Phosphor.woff2`);
const standaloneThemeCss = themeCss.replace(
	"'./Phosphor.woff2'",
	`'data:font/woff2;base64,${iconFontBytes.toString('base64')}'`,
);
const runtimeSource = await readFile(`${outputDirectory}/islands/runtime.js`, 'utf8');
const islandSource = await readFile('experiments/html-islands/planning.html', 'utf8');
await writeFile(
	`${outputDirectory}/html-islands-live.html`,
	prepareIslandContent(islandSource, diagramReferences, false),
);
const standaloneIslandHtml = prepareIslandContent(islandSource, diagramReferences, true)
	.replace(
		/<link\s+rel="stylesheet"\s+href="islands\/theme.css"\s*\/?\s*>/,
		`<style>${standaloneThemeCss}</style>`,
	)
	.replace(
		'<script type="module" src="islands/runtime.js"></script>',
		`<script type="module">${runtimeSource.replaceAll('</script', '<\\/script')}</script>`,
	);
const capabilitySource = await readFile('experiments/html-islands/capabilities.html', 'utf8');
await writeFile(
	`${outputDirectory}/html-islands-capabilities-live.html`,
	prepareIslandContent(capabilitySource, diagramReferences, false),
);
await writeFile(
	`${outputDirectory}/html-islands-composition-live.html`,
	prepareIslandContent(
		await readFile('experiments/html-islands/composition-demo.html', 'utf8'),
		diagramReferences,
		false,
	),
);
await writeFile(
	`${outputDirectory}/html-islands-layouts-live.html`,
	prepareIslandContent(
		await readFile('experiments/html-islands/layouts-demo.html', 'utf8'),
		diagramReferences,
		false,
	),
);

await writeFile(
	`${outputDirectory}/html-islands-navigation-live.html`,
	prepareIslandContent(
		await readFile('experiments/html-islands/navigation-demo.html', 'utf8'),
		diagramReferences,
		false,
	),
);

if (process.argv.includes('--islands-only')) {
	await writeFile(`${outputDirectory}/html-islands-rendered.html`, standaloneIslandHtml);
	process.stdout.write(
		'Packaged HTML Islands document using the existing component library. Run the full comparison build to refresh its report.\n',
	);
} else {
	const { renderD2Sources } = await import('../src/experiment-builders/d2-renderer.ts');
	const { renderPlan } = await import('vplan');
	const { renderMarkdownDirectives } =
		await import('../src/experiment-builders/markdown-renderer.ts');
	const { measureUsageFiles } = await import('../src/experiment-builders/usage-measurement.ts');
	const d2Sources = {
		roadmap: planningD2(views.roadmap, diagramColors),
		design: planningD2(views.design, diagramColors),
		roles: planningD2(views.roles, diagramColors),
		markdownRoadmap: planningD2(views.roadmap, diagramColors, 'plain'),
		markdownDesign: planningD2(views.design, diagramColors, 'plain'),
		markdownRoles: planningD2(views.roles, diagramColors, 'plain'),
		current: graphToD2(baseGraph, graphThemeColors).replace(
			'direction: down',
			'grid-columns: 2\ngrid-gap: 80',
		),
		proposed: graphToD2(proposedGraph, graphThemeColors).replace(
			'direction: down',
			'grid-columns: 2\ngrid-gap: 80',
		),
		diff: graphToD2(overlayGraph(baseGraph, changeSet), graphThemeColors, changeSet).replace(
			'direction: down',
			'grid-columns: 2\ngrid-gap: 80',
		),
	};
	const diagrams = await renderD2Sources(d2Sources);
	const mdxSource = await readFile('experiments/visual-plan/planning.mdx', 'utf8');
	const quickSource = await readFile('experiments/quick-json/planning.json', 'utf8');
	const markdownSource = await readFile('experiments/markdown-directives/planning.md', 'utf8');
	const graphDocumentSource = await readFile('experiments/graph-diff/planning.md', 'utf8');
	const mdxHtml = await renderPlan(mdxSource, { theme: 'light', enableSharing: false });
	await executeFile(
		process.execPath,
		[
			resolve('experiments/quick-json/upstream/render.mjs'),
			resolve('experiments/quick-json/planning.json'),
			resolve(`${outputDirectory}/quick-json-rendered.html`),
		],
		{ timeout: 30000 },
	);
	const quickHtml = await readFile(`${outputDirectory}/quick-json-rendered.html`, 'utf8');
	const plainDiagramReferences: Record<string, string> = {
		'fixtures/roadmap.d2': applySvgTheme(diagrams['markdownRoadmap'] ?? '', diagramColors),
		'fixtures/design.d2': applySvgTheme(diagrams['markdownDesign'] ?? '', diagramColors),
		'fixtures/roles.d2': applySvgTheme(diagrams['markdownRoles'] ?? '', diagramColors),
	};
	for (const name of ['current', 'diff', 'proposed']) {
		plainDiagramReferences[`fixtures/${name}.d2`] = applySvgTheme(
			diagrams[name] ?? '',
			graphThemeColors,
		);
		plainDiagramReferences[`planning-changes.json#${name}`] =
			plainDiagramReferences[`fixtures/${name}.d2`] ?? '';
	}
	const markdownHtml = htmlDocument(
		'Planning DAG · Markdown',
		await renderMarkdownDirectives(markdownSource, plainDiagramReferences),
	);
	const diffHtml = htmlDocument(
		'Planning DAG · semantic graph changes',
		await renderMarkdownDirectives(graphDocumentSource, plainDiagramReferences),
	);

	interface PrototypeDefinition {
		readonly id: string;
		readonly title: string;
		readonly authorFiles: readonly AuthorUsageFile[];
		readonly html: string;
		readonly implementation: string;
		readonly gains: readonly string[];
		readonly costs: readonly string[];
		readonly references: readonly { readonly title: string; readonly source: string }[];
		readonly liveOutput?: string;
	}
	const prototypes: readonly PrototypeDefinition[] = [
		{
			id: 'visual-plan',
			title: 'Prototype 1 · Visual Plan MDX',
			authorFiles: [{ path: 'experiments/visual-plan/planning.mdx', source: mdxSource }],
			html: mdxHtml,
			implementation: 'Unmodified vplan 0.24.0; a compiled React/beautiful-mermaid document.',
			gains: [
				'Existing MDX vocabulary handles three real diagrams and the complete jobs/boundaries narrative.',
				'Diagram expansion is provided by the upstream library.',
			],
			costs: [
				'Graph declarations and unchanged nodes are repeated in author input.',
				'The standalone artifact includes the upstream runtime and uses its own theme.',
			],
			references: [],
		},
		{
			id: 'quick-json',
			title: 'Prototype 2 · Quick JSON',
			authorFiles: [{ path: 'experiments/quick-json/planning.json', source: quickSource }],
			html: quickHtml,
			implementation:
				'Unmodified visual-explainer Quick renderer pinned at 33a6b4d43958d2d152c9cb13762af802062fade5.',
			gains: [
				'All 25 nodes, 24 relationships and boundaries fit its validated static schema.',
				'The HTML contains no application JavaScript.',
			],
			costs: [
				'Flows are cards and textual edges, not connected graphs.',
				'Detail tables are dense and prose repeats JSON property names.',
			],
			references: [],
		},
		{
			id: 'markdown-directives',
			title: 'Prototype 3 · Markdown + D2 references',
			authorFiles: [
				{ path: 'experiments/markdown-directives/planning.md', source: markdownSource },
			],
			html: markdownHtml,
			implementation: 'Local Markdown/directive resolver; D2 WASM executes at packaging time.',
			gains: [
				'The narrative stays Markdown and diagrams are references.',
				'The viewer receives six inline SVGs and no diagram engine.',
			],
			costs: [
				'The resolver supports the named corpus, not arbitrary repository imports.',
				'Long diagrams need browser zoom; this prototype has no reusable zoom island.',
			],
			references: [{ title: 'Referenced diagram corpus', source: viewsSource }],
		},
		{
			id: 'graph-diff',
			title: 'Prototype 4 · Semantic graph changes',
			authorFiles: [
				{ path: 'experiments/graph-diff/planning.md', source: graphDocumentSource },
				{ path: 'experiments/graph-diff/planning-changes.json', source: changeSource },
			],
			html: diffHtml,
			implementation:
				'Stable identities derive current, change and proposed graphs. The complete authored Markdown explanation and JSON patch are both counted.',
			gains: [
				'Six nodes and six edges are added once in the change set.',
				'Added labels and colors make the graph meaning explicit.',
			],
			costs: [
				'The base/plan split interprets a dated status snapshot, not implementation proof.',
				'The format is a flat graph; separate layouts can move unchanged nodes.',
			],
			references: [
				{ title: 'Canonical base', source: baseSource },
				{ title: 'Full source corpus', source: viewsSource },
			],
		},
		{
			id: 'html-islands',
			title: 'Prototype 5 · HTML Islands',
			authorFiles: [{ path: 'experiments/html-islands/planning.html', source: islandSource }],
			html: standaloneIslandHtml,
			liveOutput: 'html-islands-live.html',
			implementation:
				'Layout-independent native islands: Markdown, Mermaid, generic modal/sidebar inspection and arrows between HTML IDs. D2 is not part of Prototype 5.',
			gains: [
				'The author composes the same explanation in ordinary HTML layouts; shared styles and interaction implementation are reused.',
				'Markdown, Mermaid and other content can expand into a modal or side panel without fixed inline diagram frames.',
				'Arrow primitives connect island IDs across flow, grid and absolute-positioned layouts.',
				'The reference typography, palette and contents rail are reused with local fonts.',
			],
			costs: [
				'Mermaid references are packaged from the supplied corpus; arbitrary repository resolution is not implemented.',
				'The Mermaid adapter supports beautiful-mermaid’s subset, not full Mermaid compatibility.',
				'Reusable runtime, theme and fonts are delivery costs; they are excluded from authoring tokens.',
				'Diff preparation uses the source model; arbitrary Mermaid semantic patching is not implemented.',
			],
			references: [{ title: 'Referenced source corpus', source: viewsSource }],
		},
	];
	const measurements = await Promise.all(
		prototypes.map(async (prototype): Promise<PrototypeMeasurement> => {
			await writeFile(`${outputDirectory}/${prototype.id}-rendered.html`, prototype.html);
			const usage = measureUsageFiles(prototype.authorFiles);
			const countedSources = prototype.authorFiles
				.map((file): string => {
					const fileMeasurement = usage.authorFiles.find(
						(item): boolean => item.path === file.path,
					);
					return `<details><summary>Inspect authored source · ${escapeHtml(file.path)} · ${fileMeasurement?.tokens.toLocaleString()} tokens</summary><pre>${escapeHtml(file.source)}</pre></details>`;
				})
				.join('');
			const artifactBytes = Buffer.byteLength(prototype.html);
			const gzipBytes = gzipSync(prototype.html).byteLength;
			const references = prototype.references
				.map(
					(reference): string =>
						`<details><summary>${escapeHtml(reference.title)} · ${Buffer.byteLength(reference.source)} bytes</summary><pre>${escapeHtml(reference.source)}</pre></details>`,
				)
				.join('');
			const previewFile = prototype.liveOutput ?? `${prototype.id}-rendered.html`;
			await writeFile(
				`${outputDirectory}/${prototype.id}.html`,
				htmlDocument(
					prototype.title,
					`<nav><a href="../..">← All five prototypes</a> · <a href="report.html">Comparison report</a> · <a href="${previewFile}" target="_blank" rel="noopener">Open full page ↗</a> · <a href="${prototype.id}-rendered.html" download>Download standalone HTML</a></nav><h1>${escapeHtml(prototype.title)}</h1><p>${escapeHtml(prototype.implementation)}</p><p>Same complete explanation: three perspectives, 25 nodes, 24 relationships, jobs/statuses/boundaries/source references, and current/change/proposed views. Source status is a dated snapshot.</p><h2>Authored usage only</h2><p><strong>${usage.usageTokens.toLocaleString()} source tokens</strong> · ${usage.usageBytes.toLocaleString()} UTF-8 bytes · ${usage.authorFiles.length} counted file(s). Encoding: <code>o200k_base</code>. Library code, themes, fonts, engines, packaging and generated output are excluded.</p>${countedSources}<h2>Existing referenced inputs — excluded from usage</h2>${references || '<p>Diagrams are declared directly in the authored input.</p>'}<h2>Rendered result</h2><iframe title="${escapeHtml(prototype.title)} rendered result" src="${previewFile}" style="width:100%;height:1100px;border:1px solid #d7e2e8" allow="fullscreen" sandbox="allow-scripts allow-same-origin allow-downloads"></iframe><h2>Gains</h2><ul>${prototype.gains.map((gain): string => `<li>${escapeHtml(gain)}</li>`).join('')}</ul><h2>Costs and verified limitations</h2><ul>${prototype.costs.map((cost): string => `<li>${escapeHtml(cost)}</li>`).join('')}</ul><h2>Delivery sizes — separate from authoring</h2><table><tr><th>Standalone output</th><td>${artifactBytes.toLocaleString()} bytes</td></tr><tr><th>Standalone output, gzip</th><td>${gzipBytes.toLocaleString()} bytes</td></tr></table><p>Translated fixtures using an equally available source corpus. Source tokens are measured locally; billed generation cost and comprehension are unmeasured. Extra HTML Islands component demonstrations are outside this comparison.</p>`,
				),
			);
			return Object.assign(
				{
					id: prototype.id,
					title: prototype.title,
					artifactBytes,
					artifactGzipBytes: gzipBytes,
					gains: prototype.gains,
					costs: prototype.costs,
					href: `/experiments/${prototype.id}.html`,
				},
				usage,
			);
		}),
	);
	await writeFile(
		`${outputDirectory}/measurements.json`,
		`${JSON.stringify(measurements, null, 2)}\n`,
	);
	const chunkNames = await readdir(`${outputDirectory}/islands/chunks`);
	const chunks = await referencedLibraryChunks([runtimeSource], chunkNames, new Set<string>());
	const report = comparisonReport({
		measurements,
		sharedAssets: [
			{ name: 'HTML Islands core runtime', bytes: Buffer.byteLength(runtimeSource) },
			{
				name: 'Theme + embedded local fonts',
				bytes: Buffer.byteLength(themeCss),
			},
			{ name: 'Phosphor regular icons (shared WOFF2)', bytes: iconFontBytes.byteLength },
			...chunks,
		],
		existingInputs: [
			{ name: 'fixtures/planning-views.json', bytes: Buffer.byteLength(viewsSource) },
			{ name: 'fixtures/planning-base.json', bytes: Buffer.byteLength(baseSource) },
		],
	});
	await writeFile(`${outputDirectory}/report.html`, report);
	process.stdout.write(
		'Built five separate complex prototype pages and their comparison report.\n',
	);
}
