import { cp, mkdir, mkdtemp, readFile, readdir, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { extname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { prepareDesignPages } from '@sunstack/demistify/design-collection';
import { collectionSchema, recordSchema } from '@sunstack/demistify/design-records';
import type { Plugin } from 'vite';
import { z } from 'zod';

import { indexCollectionSearch } from '../../../../packages/demistify/src/node/collection-search.ts';
import { embedMarkdownFiles } from '../../../../packages/demistify/src/node/markdown-files.ts';
import { writeCollectionFixture } from '../collection-fixture.ts';
import { installDefaultCommentsFixtures } from './default-comments-fixtures.ts';

const repositoryRoot = fileURLToPath(new URL('../../../../', import.meta.url));
const contentTypes: Readonly<Record<string, string>> = {
	'.html': 'text/html',
	'.js': 'text/javascript',
	'.css': 'text/css',
	'.md': 'text/markdown',
	'.woff2': 'font/woff2',
	'.json': 'application/json',
	'.svg': 'image/svg+xml',
};

function projectionBoundaryPage(
	records: readonly unknown[],
	content: string,
	body: string,
): string {
	return `<!doctype html><html><head><meta name="viewport" content="width=device-width,initial-scale=1"><title>Projection boundaries</title><link rel="stylesheet" href="/__demistify/library/theme.css"><script type="module" src="/__demistify/library/runtime.js"></script></head><body><ex-document id="content"><h1>Projection boundaries</h1><span id="outside-label">Outside label</span><p id="outside-description">Outside description</p>${content}</ex-document><script type="application/json" data-ex-records>${JSON.stringify(records)}</script>${body}</body></html>`;
}

function projectionBoundaryBody(name: string): string {
	return `<label id="shared-label" for="shared-input" data-local-label>${name} label</label><span id="shared-extra" data-local-extra>${name} extra</span><input id="shared-input" data-local-input aria-labelledby="shared-label shared-extra outside-label" aria-describedby="shared-target outside-description"><p id="shared-target" data-local-target>${name} intended target</p><a data-local-link href="#shared-target">Local target</a><a data-encoded-link href="#%73hared-target">Encoded target</a><a data-malformed-link href="#%ZZ">Malformed target</a><a data-external-link href="https://example.org/reference#outside">External target</a>`;
}

/** Freeze built assets and authored pages; serve bytes without Vite rewriting worker imports. */
export async function builtFixtures(): Promise<Plugin> {
	const snapshot = await mkdtemp(join(tmpdir(), '2026-10-demistify-browser-'));
	await cp(
		process.env['DEMISTIFY_BROWSER_ASSETS'] ?? resolve(repositoryRoot, 'packages/demistify/dist'),
		join(snapshot, 'library'),
		{ recursive: true },
	);
	// Keep the real built consumer on the same declared library snapshot as every fixture.
	const surfacesHtml = await readFile(
		resolve(
			repositoryRoot,
			'apps/demistify-poc/dist/experiments/html-islands-content-surfaces-live.html',
		),
		'utf8',
	);
	await mkdir(join(snapshot, 'content-surfaces'));
	await writeFile(
		join(snapshot, 'content-surfaces/index.html'),
		surfacesHtml
			.replaceAll('href="islands/', 'href="/__demistify/library/')
			.replaceAll('src="islands/', 'src="/__demistify/library/'),
	);
	await Promise.all(
		[
			['navigation', 'navigation-markdown-fix'],
			['layout', 'layout-regression-fix'],
			['gallery', 'integrated-review'],
			['explorer', 'round-13-inspection'],
			['ui', 'review-ui'],
			['show-me-work', 'show-me-work'],
			['search', 'command-search'],
			['review', 'choices-comments'],
			['palette', 'diagram-colors'],
			['shared-navigation', 'shared-navigation'],
		].map(async ([name, source]): Promise<void> => {
			if (!name || !source) throw new Error('Missing fixture name.');
			await cp(
				resolve(repositoryRoot, 'apps/demistify-poc/experiments/agent-written', source),
				join(snapshot, name),
				{ recursive: true },
			);
			await Promise.all(
				(await readdir(join(snapshot, name)))
					.filter((file): boolean => file.endsWith('.html'))
					.map(async (filename): Promise<void> => {
						const file = join(snapshot, name, filename);
						const html = await readFile(file, 'utf8');
						await writeFile(
							file,
							html
								.replaceAll('/experiments/islands/', '/__demistify/library/')
								.replaceAll('../../../../../packages/demistify/dist/', '/__demistify/library/'),
						);
					}),
			);
		}),
	);
	const sharedState = await readFile(join(snapshot, 'shared-navigation/state.html'), 'utf8');
	await writeFile(
		join(snapshot, 'shared-navigation/unavailable.html'),
		sharedState.replace('./navigation.json', './missing.json'),
	);
	await writeFile(
		join(snapshot, 'shared-navigation/invalid.html'),
		sharedState.replace('./navigation.json', './invalid.json'),
	);
	await writeFile(
		join(snapshot, 'shared-navigation/invalid.json'),
		JSON.stringify({
			version: 1,
			pages: [{ id: 'work', title: 'Work', href: 'how-work-runs.html', icon: 'path' }],
			map: [{ label: 'Missing', icon: 'user', target: { page: 'missing' } }],
		}),
	);
	// Exercise the optional Walkthrough launcher beside Map without changing the fixed author input.
	const storyControls = await readFile(join(snapshot, 'show-me-work/index.html'), 'utf8');
	await writeFile(
		join(snapshot, 'show-me-work/reader-controls.html'),
		storyControls.replace(
			'<ex-document id="work">',
			'<ex-nav for="work"></ex-nav><ex-presentation id="story-walkthrough" title="Read the story" description="Inspect the supplied account and its record."><ex-step to="story-main">Read the supplied account.</ex-step><ex-step to="story-other">Read the independent verification account.</ex-step></ex-presentation><ex-document id="work">',
		),
	);
	await cp(
		process.env['DEMISTIFY_BROWSER_LAB'] ??
			resolve(repositoryRoot, 'apps/demistify-poc/public/experiments/understanding-lab'),
		join(snapshot, 'understanding'),
		{ recursive: true },
	);
	await cp(join(snapshot, 'library'), join(snapshot, 'understanding/library'), { recursive: true });
	const rulePage = await readFile(join(snapshot, 'understanding/requirements.html'), 'utf8');
	const carrier = /(<script[^>]*data-ex-design-collection[^>]*>)([\s\S]*?)(<\/script>)/.exec(
		rulePage,
	);
	if (!carrier?.[2]) throw new Error('Missing prepared rule carrier.');
	const ruleCollection = collectionSchema.parse(JSON.parse(carrier[2]));
	const trailingNoteCollection = {
		...ruleCollection,
		records: ruleCollection.records.map((item) =>
			item.record.id === 'S2'
				? Object.assign({}, item, {
						bodyHtml: `${item.bodyHtml}<p>A trailing author note is not the observable rule.</p>`,
					})
				: item,
		),
	};
	await writeFile(
		join(snapshot, 'understanding/trailing-note.html'),
		rulePage.replace(
			carrier[0],
			`${carrier[1]}${JSON.stringify(trailingNoteCollection).replaceAll('<', '\\u003c')}${carrier[3]}`,
		),
	);
	const collectionSource = join(snapshot, 'collection-source');
	await writeCollectionFixture(collectionSource);
	const collectionFixture = await prepareDesignPages(collectionSource);
	await cp(join(snapshot, 'understanding'), join(snapshot, 'understanding-collection'), {
		recursive: true,
	});
	await Promise.all(
		[...collectionFixture.pages].map(([file, html]) =>
			writeFile(join(snapshot, 'understanding-collection', file), html),
		),
	);
	const documentSource = resolve(
		repositoryRoot,
		'apps/demistify-poc/experiments/agent-written/document-browsers',
	);
	const collision = join(snapshot, 'document-browsers', 'cross-page');
	await mkdir(collision, { recursive: true });
	const need = {
		kind: 'need',
		id: 'BoundaryNeed',
		title: 'Keep the intended target',
		capability: 'Navigation',
		bodyRef: 'need-body',
		summaryRef: 'need-summary',
	};
	const rule = {
		kind: 'specification',
		id: 'BoundaryRule',
		title: 'The local target belongs to this record',
		needs: ['BoundaryNeed'],
		bodyRef: 'rule-body',
		summaryRef: 'rule-summary',
	};
	const design = {
		kind: 'program-design',
		id: 'BoundaryDesign',
		title: 'The owner keeps its identifiers',
		specifications: ['BoundaryRule'],
		bodyRef: 'design-body',
		summaryRef: 'design-summary',
	};
	const scenario = {
		kind: 'scenario',
		id: 'BoundaryScenario',
		title: 'Follow a procedure target',
		specifications: ['BoundaryRule'],
		bodyRef: 'scenario-body',
		summaryRef: 'scenario-summary',
	};
	const observation = {
		kind: 'observation',
		id: 'BoundaryObservation',
		scenario: 'BoundaryScenario',
		outcome: 'Partial',
		bodyRef: 'observation-body',
		summaryRef: 'observation-summary',
	};
	const reader = '<ex-requirements id="boundary-reader"></ex-requirements>';
	await writeFile(
		join(collision, 'requirements.html'),
		projectionBoundaryPage(
			[need],
			`<p id="shared-target">Root competing target</p><label id="shared-label" for="shared-input">Root competing label</label><span id="shared-extra">Root competing extra</span><input id="shared-input"><p id="procedure-target">Root competing procedure</p>${reader}`,
			'<template id="need-body"><p id="need-summary">Follow this record, not a similarly named element.</p></template>',
		),
	);
	await writeFile(
		join(collision, 'specification.html'),
		projectionBoundaryPage(
			[rule],
			reader,
			`<template id="rule-body"><p id="rule-summary">The specification is local.</p><section data-ex-part="rule">${projectionBoundaryBody('Specification')}</section></template>`,
		),
	);
	await writeFile(
		join(collision, 'program-design.html'),
		projectionBoundaryPage(
			[design],
			reader,
			`<template id="design-body"><p id="design-summary">The design is local.</p>${projectionBoundaryBody('Design')}</template>`,
		),
	);
	await writeFile(
		join(collision, 'verification.html'),
		projectionBoundaryPage(
			[scenario, observation],
			reader,
			`<template id="scenario-body"><p id="scenario-summary" aria-describedby="procedure-target">The procedure is local. <a data-procedure-link href="#procedure-target">Read procedure</a></p><details open id="procedure-target" data-ex-part="procedure"><summary>Procedure target</summary>${projectionBoundaryBody('Procedure')}</details></template><template id="observation-body"><p id="observation-summary">Only the supplied partial result is known.</p><p id="observation-target">Observed detail</p><a data-observation-link href="#observation-target">Read observation</a></template>`,
		),
	);
	await writeFile(
		join(collision, 'execution.html'),
		projectionBoundaryPage(
			[
				{
					kind: 'frame',
					id: 'LinkedFrame',
					name: 'Input.read',
					source: 'input.ts:1',
					responsibility: 'Read the input.',
					statement: 'readInput();',
					requirement: 'BoundaryNeed',
					category: 'Input',
				},
				{
					kind: 'frame',
					id: 'SupportingFrame',
					name: 'Channel.forward',
					source: 'channel.ts:2',
					responsibility: 'Forward the input.',
					statement: 'forwardInput();',
					category: 'Plumbing',
					supporting: true,
				},
				{
					kind: 'frame',
					id: 'ActionFrame',
					name: 'Batch.finish',
					source: 'action.ts:3',
					responsibility: 'Finish the batch.',
					statement: 'finishBatch();',
					category: 'Action',
					supporting: false,
				},
				{
					kind: 'stack',
					id: 'BoundaryStack',
					title: 'Read and finish',
					description: 'A supplied path, not a captured trace.',
					primaryNeed: 'BoundaryNeed',
					frames: ['LinkedFrame', 'SupportingFrame', 'ActionFrame'],
				},
			],
			'<ex-call-stack id="boundary-calls"></ex-call-stack>',
			'',
		),
	);
	const collisionPrepared = await prepareDesignPages(collision);
	await Promise.all(
		[...collisionPrepared.pages].map(([name, html]) => writeFile(join(collision, name), html)),
	);
	const publicStyle = join(snapshot, 'document-browsers', 'public-style');
	await cp(
		resolve(repositoryRoot, 'apps/demistify-poc/experiments/understanding-lab'),
		publicStyle,
		{ recursive: true },
	);
	await Promise.all(
		(await readdir(publicStyle))
			.filter((name) => name.endsWith('.html'))
			.map(async (name): Promise<void> => {
				const file = join(publicStyle, name);
				const html = (await readFile(file, 'utf8')).replace(
					/\sclass="([^"]*)"/g,
					(_match, classes: string): string => {
						const kept = classes.split(/\s+/).filter((value) => !value.startsWith('ex-design-'));
						return kept.length ? ` class="${kept.join(' ')}"` : '';
					},
				);
				if (/class="[^"]*ex-design-/.test(html))
					throw new Error('Private author class in public fixture');
				await writeFile(file, html);
			}),
	);
	const publicPrepared = await prepareDesignPages(publicStyle);
	await Promise.all(
		[...publicPrepared.pages].map(([name, html]) => writeFile(join(publicStyle, name), html)),
	);
	await cp(join(snapshot, 'library'), join(publicStyle, 'library'), { recursive: true });
	const inspectorPage = await readFile(join(publicStyle, 'requirements.html'), 'utf8');
	await writeFile(
		join(publicStyle, 'inspector-controls.html'),
		inspectorPage.replace(
			'</ex-document>',
			'<ex-zoom label="Unrelated disclosure" zoom><details id="ordinary-detail"><summary>Authored extra</summary><p>Ordinary authored disclosure.</p></details></ex-zoom></ex-document>',
		),
	);
	await writeFile(
		join(publicStyle, 'inspector-copy.html'),
		inspectorPage
			.replace('><ex-requirements', '><section><ex-requirements')
			.replace('</ex-requirements></ex-zoom>', '</ex-requirements></section></ex-zoom>'),
	);
	const callInspectorPage = await readFile(join(publicStyle, 'execution.html'), 'utf8');
	await writeFile(
		join(publicStyle, 'inspector-calls.html'),
		callInspectorPage.replace(
			/(<ex-call-stack\b[^>]*>[\s\S]*?<\/ex-call-stack\s*>)/,
			'<ex-zoom label="Call reader" zoom>$1</ex-zoom>',
		),
	);
	const documentCases = [
		{
			name: 'wide-table',
			files: ['requirements.html', 'specification.html'],
		},
		{ name: 'needs', files: ['requirements.html'] },
		{ name: 'rules', files: ['requirements.html', 'specification.html'] },
		{
			name: 'design',
			files: ['requirements.html', 'specification.html', 'program-design.html', 'notes.md'],
		},
		{
			name: 'full',
			files: [
				'requirements.html',
				'specification.html',
				'program-design.html',
				'verification.html',
				'notes.md',
			],
		},
	];
	await Promise.all(
		documentCases.map(async ({ name, files }): Promise<void> => {
			const destination = join(snapshot, 'document-browsers', name);
			await mkdir(destination, { recursive: true });
			await Promise.all(
				files.map((file) => cp(join(documentSource, file), join(destination, file))),
			);
			if (name === 'wide-table') {
				const file = join(destination, 'specification.html');
				const columns = [
					'InitialInputIdentifier',
					'WriterSourceFingerprint',
					'RefusalEvidenceChecksum',
					'ResolutionBoundaryName',
					'RecoveredPayloadIdentifier',
				];
				const table = `<div class="ex-table-scroll" data-ex-part="cases"><table><thead><tr>${columns.map((column) => `<th>${column}</th>`).join('')}</tr></thead><tbody><tr>${columns.map((column) => `<td>${column}-retained</td>`).join('')}</tr></tbody></table></div>`;
				await writeFile(
					file,
					(await readFile(file, 'utf8')).replace(
						/<figure data-ex-part="cases">[\s\S]*?<\/figure>/,
						table,
					),
				);
			}
			const prepared = await prepareDesignPages(destination);
			await Promise.all(
				[...prepared.pages].map(([file, html]) => writeFile(join(destination, file), html)),
			);
		}),
	);
	const multipleSource = join(snapshot, 'multiple-observations-source');
	await cp(documentSource, multipleSource, { recursive: true });
	const verificationFile = join(multipleSource, 'verification.html');
	const verificationSource = await readFile(verificationFile, 'utf8');
	const authoredBlock = /(<script[^>]*data-ex-records[^>]*>)([\s\S]*?)(<\/script>)/.exec(
		verificationSource,
	);
	if (!authoredBlock?.[2]) throw new Error('Missing authored observation records.');
	const authoredRecords = z.array(recordSchema).parse(JSON.parse(authoredBlock[2]));
	const firstPass = recordSchema.parse({
		kind: 'observation',
		id: 'DraftPass',
		scenario: 'DraftCheck',
		outcome: 'Pass',
		bodyRef: 'pass-body',
		summaryRef: 'pass-summary',
	});
	const withBothOutcomes = authoredRecords.flatMap((record) =>
		record.id === 'DraftObservation' ? [firstPass, record] : [record],
	);
	await writeFile(
		verificationFile,
		verificationSource
			.replace(
				authoredBlock[0],
				`${authoredBlock[1]}${JSON.stringify(withBothOutcomes)}${authoredBlock[3]}`,
			)
			.replace(
				'</body>',
				'<template id="pass-body"><p id="pass-summary">Observed first pass: the draft remained.</p></template></body>',
			),
	);
	const multiplePrepared = await prepareDesignPages(multipleSource);
	const multipleOutput = join(snapshot, 'document-browsers', 'multiple');
	await cp(multipleSource, multipleOutput, { recursive: true });
	await Promise.all(
		[...multiplePrepared.pages].map(([file, html]) => writeFile(join(multipleOutput, file), html)),
	);
	const revisionCases = ['revision-base', 'revision-body'];
	await Promise.all(
		revisionCases.map(async (name): Promise<void> => {
			const destination = join(snapshot, 'document-browsers', name);
			await cp(documentSource, destination, { recursive: true });
			if (name === 'revision-body') {
				const file = join(destination, 'specification.html');
				await writeFile(
					file,
					(await readFile(file, 'utf8')).replace(
						'the same draft text remains editable',
						'the same draft text and cursor remain editable',
					),
				);
			}
			if (name.startsWith('revision-md-'))
				await writeFile(join(destination, 'notes.md'), `${name}: changed **Markdown** source.`);
			const prepared = await prepareDesignPages(destination);
			await Promise.all(
				[...prepared.pages].map(([file, html]) => writeFile(join(destination, file), html)),
			);
			const repeated = await prepareDesignPages(destination);
			if (repeated.collection.identity !== prepared.collection.identity)
				throw new Error('Repeated preparation changed revision.');
		}),
	);
	const markdownWorking = join(snapshot, 'markdown-source');
	await cp(join(snapshot, 'document-browsers', 'revision-base'), markdownWorking, {
		recursive: true,
	});
	await ['revision-md-first', 'revision-md-second'].reduce(
		async (previous, name): Promise<void> => {
			await previous;
			await writeFile(join(markdownWorking, 'notes.md'), `${name}: changed **Markdown** source.`);
			const prepared = await prepareDesignPages(markdownWorking);
			await Promise.all(
				[...prepared.pages].map(([file, html]) => writeFile(join(markdownWorking, file), html)),
			);
			if (
				!(await readFile(join(markdownWorking, 'program-design.html'), 'utf8')).includes(
					'src="notes.md"',
				)
			)
				throw new Error('Preparation consumed the authored Markdown source reference.');
			await cp(markdownWorking, join(snapshot, 'document-browsers', name), { recursive: true });
		},
		Promise.resolve(),
	);
	const unprepared = await readFile(join(documentSource, 'requirements.html'), 'utf8');
	await writeFile(join(snapshot, 'document-browsers', 'unprepared.html'), unprepared);
	const preparedFull = await readFile(
		join(snapshot, 'document-browsers', 'full', 'requirements.html'),
		'utf8',
	);
	await writeFile(
		join(snapshot, 'document-browsers', 'invalid.html'),
		preparedFull.replace('"version":1', '"version":2'),
	);
	const preparedCarrier =
		/(<script[^>]*data-ex-design-collection[^>]*>)([\s\S]*?)(<\/script>)/.exec(preparedFull);
	if (!preparedCarrier?.[2]) throw new Error('Missing public prepared carrier.');
	const parsedCarrier = collectionSchema.parse(JSON.parse(preparedCarrier[2]));
	await writeFile(
		join(snapshot, 'document-browsers', 'empty.html'),
		preparedFull.replace(
			preparedCarrier[0],
			`${preparedCarrier[1]}${JSON.stringify(Object.assign({}, parsedCarrier, { records: [] }))}${preparedCarrier[3]}`,
		),
	);
	await cp(
		process.env['DEMISTIFY_BROWSER_AUTHOR'] ?? join(snapshot, 'document-browsers', 'full'),
		join(snapshot, 'document-author'),
		{ recursive: true },
	);
	const labelPage = await readFile(
		join(snapshot, 'document-browsers', 'full', 'verification.html'),
		'utf8',
	);
	await writeFile(
		join(snapshot, 'document-browsers', 'labeled.html'),
		labelPage
			.replace('<ex-requirements id=', '<ex-requirements label="Draft design needs" id=')
			.replace('<ex-call-stack id=', '<ex-call-stack label="Draft source calls" id='),
	);
	const reviewHtml = await readFile(join(snapshot, 'review/index.html'), 'utf8');
	await writeFile(
		join(snapshot, 'review/boundary.html'),
		reviewHtml.replace(
			'document-id="demistify-choice-fixture"',
			'document-id="demistify-review-boundary-fixture"',
		),
	);
	const searchPages = await Promise.all(
		['index.html', 'evidence.html'].map(async (name) => ({
			filePath: join(snapshot, 'search', name),
			html: await readFile(join(snapshot, 'search', name), 'utf8'),
			markdownFiles: {},
		})),
	);
	await Promise.all(
		indexCollectionSearch(searchPages).map(async (page): Promise<void> =>
			writeFile(page.filePath, page.html),
		),
	);
	const navigationHtml = await readFile(join(snapshot, 'navigation/index.html'), 'utf8');
	const notesSource = await readFile(join(snapshot, 'navigation/notes.md'), 'utf8');
	await mkdir(join(snapshot, 'embedded'));
	await writeFile(
		join(snapshot, 'embedded/index.html'),
		embedMarkdownFiles(navigationHtml, { 'notes.md': notesSource }),
	);
	return {
		name: 'demistify-built-fixtures',
		configureServer(server): void {
			installDefaultCommentsFixtures(server);
			process.stdout.write(`Demistify browser snapshot: ${snapshot}\n`);
			server.middlewares.use('/__demistify/', (request, response): void => {
				const path = decodeURIComponent((request.url ?? '').split('?')[0] ?? '');
				if (path.split('/').some((part): boolean => part === '..') || path.includes('\\')) {
					response.writeHead(400).end();
					return;
				}
				const file = join(snapshot, path);
				void readFile(file).then(
					(content): void => {
						response.setHeader(
							'Content-Type',
							contentTypes[extname(file)] ?? 'application/octet-stream',
						);
						response.setHeader('Cache-Control', 'no-store');
						response.end(content);
					},
					(): void => {
						response.writeHead(404).end();
					},
				);
			});
		},
	};
}
