#!/usr/bin/env node
/**
 * demistify check [--fix] [--render] [--no-diagrams] [--site-root <dir>] [--write] [--json] <page.html | directory …>
 *   A directory with initiative.ts or design.ts, or a page that binds generated design data, gets
 *   that tree's typed check (once per tree). A typed page outside any tree is a published copy and,
 *   like every other page, gets the page check.
 * demistify check --hook      (Claude Code PostToolUse hook: reads the hook event from stdin)
 *
 * Prints one line per problem: file:line:col severity rule: message. Exits 1 when any error remains.
 * With --json it prints one object instead: error and warning counts, each typed tree's error,
 * warnings and report, published copies, each page's findings and any render findings.
 * In hook mode it stays silent on clean or non-explainer files, and otherwise prints the problems to
 * stderr and exits 2 so the agent sees them and fixes the page.
 */
import { readFile, writeFile } from 'node:fs/promises';
import { dirname, relative, resolve } from 'node:path';
import { text as readStreamText } from 'node:stream/consumers';
import { parseArgs } from 'node:util';

import { applyEdits, type CheckOptions, checkHtml, formatFindings } from '../src/node/check.ts';
import { indexCollectionSearch, type SearchPageSource } from '../src/node/collection-search.ts';
import { prepareDesignPages } from '../src/node/design-collection.ts';
import { embedMarkdownFiles, markdownFileReferences } from '../src/node/markdown-files.ts';

const write = (text: string): void => {
	if (text) process.stdout.write(`${text}\n`);
};

async function readHookFile(): Promise<string | undefined> {
	process.stdin.setEncoding('utf8');
	let text = '';
	for await (const chunk of process.stdin) text += typeof chunk === 'string' ? chunk : '';
	try {
		const event: unknown = JSON.parse(text);
		const input =
			typeof event === 'object' && event !== null && 'tool_input' in event
				? event.tool_input
				: undefined;
		const path =
			typeof input === 'object' && input !== null && 'file_path' in input
				? input.file_path
				: undefined;
		return typeof path === 'string' && path.endsWith('.html') ? path : undefined;
	} catch {
		return undefined;
	}
}

async function main(): Promise<number> {
	const [command, ...rest] = process.argv.slice(2);
	if (command === 'open') {
		const usage = 'Usage: demistify open <page.html> [--site-root <directory>] [--no-browser]';
		try {
			const { values, positionals } = parseArgs({
				args: rest,
				allowPositionals: true,
				options: {
					'site-root': { type: 'string' },
					'no-browser': { type: 'boolean' },
					help: { type: 'boolean', short: 'h' },
				},
			});
			if (values.help || positionals.length !== 1 || !positionals[0]) {
				write(usage);
				return values.help ? 0 : 1;
			}
			const { openPage } = await import('../src/node/open-page.ts');
			await openPage(positionals[0], {
				siteRoot: values['site-root'],
				noBrowser: values['no-browser'] ?? false,
			});
			return 0;
		} catch (error) {
			process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
			return 1;
		}
	}
	if (command === 'initiative') {
		const [action, ...args] = rest;
		const usage =
			'Usage: demistify initiative check [directory] [--site-root <directory>] [--write]\n       demistify initiative status [directory] [--site-root <directory>] [--json]\n       demistify initiative answer <review.md|-> [directory] [--site-root <directory>] [--decided-on <yyyy-mm-dd>] [--write]\n       demistify initiative add <choice|ruling|assumption> [directory] --namespace <initiative|project> --page <page.html> [--words <owner words>] [--author <alias>] [--site-root <directory>] [--write]';
		if (!action || action === '--help') {
			write(usage);
			return 0;
		}
		const { values, positionals } = parseArgs({
			args,
			allowPositionals: true,
			options: {
				'site-root': { type: 'string' },
				write: { type: 'boolean' },
				json: { type: 'boolean' },
				'decided-on': { type: 'string' },
				namespace: { type: 'string' },
				page: { type: 'string' },
				words: { type: 'string' },
				author: { type: 'string' },
				help: { type: 'boolean', short: 'h' },
			},
		});
		if (values.help) {
			write(usage);
			return 0;
		}
		if (action === 'answer' && positionals.length >= 1 && positionals.length <= 2 && !values.json) {
			const [reviewFile = '-', answerDirectory = '.'] = positionals;
			const reviewText =
				reviewFile === '-'
					? await readStreamText(process.stdin)
					: await readFile(reviewFile, 'utf8');
			const { applyReviewAnswers } = await import('../src/node/design-authoring/review-answers.ts');
			write(
				await applyReviewAnswers(reviewText, answerDirectory, {
					siteRoot: values['site-root'],
					decidedOn: values['decided-on'],
					write: values.write,
				}),
			);
			return 0;
		}
		if (action === 'add' && positionals.length >= 1 && positionals.length <= 2) {
			const [kind, addDirectory = '.'] = positionals;
			const { addRecord, scaffoldKinds } =
				await import('../src/node/design-authoring/record-scaffold.ts');
			const scaffoldKind = scaffoldKinds.find((candidate) => candidate === kind);
			if (!scaffoldKind || !values.namespace || !values.page) {
				write(usage);
				return 1;
			}
			write(
				await addRecord(scaffoldKind, addDirectory, {
					namespace: values.namespace,
					page: values.page,
					words: values.words,
					author: values.author,
					siteRoot: values['site-root'],
					write: values.write,
				}),
			);
			return 0;
		}
		const [directory = '.'] = positionals;
		if (action === 'check' && positionals.length <= 1 && !values.json && !values['decided-on']) {
			const { checkInitiative } = await import('../src/node/design-authoring/check-initiative.ts');
			write(
				await checkInitiative(directory, { siteRoot: values['site-root'], write: values.write }),
			);
			return 0;
		}
		if (action === 'status' && positionals.length <= 1 && !values.write && !values['decided-on']) {
			const { initiativeStatus } =
				await import('../src/node/design-authoring/initiative-status.ts');
			write(
				await initiativeStatus(directory, { siteRoot: values['site-root'], json: values.json }),
			);
			return 0;
		}
		write(usage);
		return 1;
	}
	if (command === 'brief') {
		const usage = 'Usage: demistify brief <deliverable> [directory] [--site-root <directory>]';
		const { values, positionals } = parseArgs({
			args: rest,
			allowPositionals: true,
			options: { 'site-root': { type: 'string' }, help: { type: 'boolean', short: 'h' } },
		});
		const [reference, directory = '.'] = positionals;
		if (values.help || !reference || positionals.length > 2) {
			write(usage);
			return values.help ? 0 : 1;
		}
		const { deliveryBrief } = await import('../src/node/design-authoring/delivery-brief.ts');
		write(await deliveryBrief(reference, directory, values['site-root']));
		return 0;
	}
	if (command === 'design') {
		const [action, ...args] = rest;
		const usage =
			'Usage: demistify design init [directory]\n       demistify design types [directory]\n       demistify design check [directory] [--site-root <directory>] [--write]';
		if (!action || action === '--help') {
			write(usage);
			return 0;
		}
		const { values, positionals } = parseArgs({
			args,
			allowPositionals: true,
			options: {
				'site-root': { type: 'string' },
				write: { type: 'boolean' },
				help: { type: 'boolean' },
			},
		});
		if (values.help) {
			write(usage);
			return 0;
		}
		const [directory = '.'] = positionals;
		if (
			positionals.length <= 1 &&
			action === 'init' &&
			values['site-root'] === undefined &&
			!values.write
		) {
			const { initDesignSet } = await import('../src/node/design-authoring/init-design.ts');
			write(await initDesignSet(directory));
			return 0;
		}
		if (
			positionals.length <= 1 &&
			action === 'types' &&
			values['site-root'] === undefined &&
			!values.write
		) {
			const { refreshDesignTypes } = await import('../src/node/design-authoring/init-design.ts');
			write(await refreshDesignTypes(directory));
			return 0;
		}
		if (positionals.length <= 1 && action === 'check') {
			const { checkDesignSet } = await import('../src/node/design-authoring/check-design.ts');
			write(await checkDesignSet(directory, values['site-root'], values.write));
			return 0;
		}
		write(usage);
		return 1;
	}
	if (command === 'prepare') {
		const [directory] = rest;
		if (!directory || rest.length !== 1) {
			write('Usage: demistify prepare <collection-directory>');
			return 1;
		}
		const prepared = await prepareDesignPages(directory);
		await Promise.all(
			[...prepared.pages].map(([page, html]) => writeFile(resolve(directory, page), html)),
		);
		write(
			`Prepared ${prepared.pages.size} pages and ${prepared.collection.records.length} records.`,
		);
		return 0;
	}
	if (command === 'index') {
		if (rest.length === 0 || rest.some((file): boolean => !file.endsWith('.html'))) {
			write('Usage: demistify index <page.html …>');
			return 1;
		}
		const sources = await Promise.all(
			rest.map(async (filePath): Promise<SearchPageSource> => {
				const html = await readFile(filePath, 'utf8');
				const markdownFiles: Record<string, string> = {};
				await Promise.all(
					markdownFileReferences(html).map(async (reference): Promise<void> => {
						if (/^[a-z][a-z0-9+.-]*:|^\/\//i.test(reference))
							throw new Error('index reads local Markdown files only.');
						markdownFiles[reference] = await readFile(
							resolve(dirname(filePath), reference),
							'utf8',
						);
					}),
				);
				return { filePath, html, markdownFiles };
			}),
		);
		const indexed = indexCollectionSearch(sources);
		await Promise.all(
			indexed.map(async (page): Promise<void> => writeFile(page.filePath, page.html)),
		);
		write(
			`Indexed ${indexed.length} pages and ${indexed[0]?.destinations.length ?? 0} destinations. Regenerate after changing headings or walkthroughs.`,
		);
		return 0;
	}
	if (command === 'embed') {
		const [input, output] = rest;
		if (!input || !output || rest.length !== 2) {
			write('Usage: demistify embed input.html output.html');
			return 1;
		}
		const html = await readFile(input, 'utf8');
		const sources: Record<string, string> = {};
		for (const reference of markdownFileReferences(html)) {
			if (/^[a-z][a-z0-9+.-]*:|^\/\//i.test(reference))
				throw new Error('embed reads local Markdown files only.');
			// oxlint-disable-next-line no-await-in-loop -- read each local source before writing the output.
			sources[reference] = await readFile(resolve(dirname(input), reference), 'utf8');
		}
		await writeFile(output, embedMarkdownFiles(html, sources));
		write(
			`Embedded ${Object.keys(sources).length} Markdown file(s) into ${output}. Shared library links retained.`,
		);
		return 0;
	}
	if (command !== 'check') {
		write(
			'Usage: demistify check [--fix] [--render] [--no-diagrams] [--site-root <directory>] [--write] [--json] <page.html | directory …>\n       demistify check --hook\n       demistify open <page.html> [--site-root <directory>] [--no-browser]\n       demistify design --help\n       demistify brief <deliverable> [directory] [--site-root <directory>]\n       demistify initiative check [directory] [--site-root <directory>] [--write]\n       demistify initiative status [directory] [--site-root <directory>] [--json]\n       demistify initiative answer <review.md|-> [directory] [--site-root <directory>] [--decided-on <yyyy-mm-dd>] [--write]\n       demistify initiative add <choice|ruling|assumption> [directory] --namespace <initiative|project> --page <page.html> [--words <owner words>] [--author <alias>] [--site-root <directory>] [--write]',
		);
		return !command || ['--help', '-h', 'help'].includes(command) ? 0 : 1;
	}
	const flags = new Set(rest.filter((argument): boolean => argument.startsWith('--')));
	const optionsFor = (page: string): CheckOptions => ({
		diagrams: !flags.has('--no-diagrams'),
		readSource: async (path: string): Promise<string | undefined> =>
			readFile(resolve(dirname(page), path), 'utf8').catch((): undefined => undefined),
	});

	if (flags.has('--hook')) {
		const file = await readHookFile();
		if (!file) return 0;
		const source = await readFile(file, 'utf8');
		if (!source.includes('<ex-')) return 0;
		const findings = await checkHtml(source, optionsFor(file));
		if (findings.length === 0) return 0;
		process.stderr.write(
			`${formatFindings(relative(process.cwd(), file), findings)}\nFix these in the page (fixable ones: run demistify check --fix).\n`,
		);
		return 2;
	}

	const siteRootIndex = rest.indexOf('--site-root');
	const siteRoot = siteRootIndex === -1 ? undefined : rest[siteRootIndex + 1];
	if (siteRootIndex !== -1 && (!siteRoot || siteRoot.startsWith('--'))) {
		write('Usage: demistify check --site-root <directory> <path …>');
		return 1;
	}
	const files = rest.filter(
		(argument, index): boolean =>
			!argument.startsWith('--') && (siteRootIndex === -1 || index !== siteRootIndex + 1),
	);
	if (files.length === 0) {
		write('Usage: demistify check <page.html | directory …>');
		return 1;
	}
	const json = flags.has('--json');
	const text = (line: string): void => {
		if (!json) write(line);
	};
	const { routeCheckTargets } = await import('../src/node/check-routing.ts');
	const routes = await routeCheckTargets(files);
	let errors = 0;
	let warnings = 0;
	const treeReports = await Promise.all(
		routes.trees.map(async (tree) => {
			const directory = relative(process.cwd(), tree.directory) || '.';
			const options = { siteRoot, write: flags.has('--write') };
			try {
				const build =
					tree.kind === 'initiative'
						? await (
								await import('../src/node/design-authoring/check-initiative.ts')
							).buildInitiative(tree.directory, options)
						: await (
								await import('../src/node/design-authoring/check-design.ts')
							).buildDesignSet(tree.directory, options);
				return { ...tree, directory, error: null, warnings: build.warnings, report: build.result };
			} catch (error) {
				const message = error instanceof Error ? error.message : String(error);
				return { ...tree, directory, error: message, warnings: [], report: '' };
			}
		}),
	);
	for (const tree of treeReports) {
		const label = `${tree.directory} (${tree.kind} check for ${tree.inputs.join(', ')})`;
		text(tree.error === null ? `${label}:\n${tree.report}` : `${label}: error: ${tree.error}`);
		if (tree.error !== null) errors += 1;
		warnings += tree.warnings.length;
	}
	for (const page of routes.published)
		text(
			`${page}: no initiative.ts or design.ts encloses it, so it gets the page check as a published copy.`,
		);
	const results = await Promise.all(
		routes.pages.map(async (file) => {
			let source = await readFile(file, 'utf8');
			let findings = await checkHtml(source, optionsFor(file));
			if (flags.has('--fix') && findings.some((finding): boolean => finding.edit !== undefined)) {
				source = applyEdits(source, findings);
				await writeFile(file, source);
				findings = await checkHtml(source, optionsFor(file));
			}
			return { file, findings };
		}),
	);
	for (const { file, findings } of results) {
		text(formatFindings(file, findings));
		errors += findings.filter((finding): boolean => finding.severity === 'error').length;
		warnings += findings.filter((finding): boolean => finding.severity === 'warning').length;
	}
	const htmlFiles = files.filter((file): boolean => file.endsWith('.html'));
	const renderFindings: { file: string; width: number; severity: string; message: string }[] = [];
	if (flags.has('--render') && htmlFiles.length > 0) {
		const { renderCheck } = await import('../src/node/render-check.ts');
		try {
			for (const finding of await renderCheck(process.cwd(), htmlFiles)) {
				const warning = finding.severity === 'warning';
				renderFindings.push({
					file: finding.file,
					width: finding.width,
					severity: finding.severity,
					message: finding.message,
				});
				text(
					`${finding.file} render(${finding.width}px): ${warning ? 'warning: ' : ''}${finding.message}`,
				);
				if (warning) warnings += 1;
				else errors += 1;
			}
		} catch (error) {
			process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
			return 1;
		}
	}
	if (json) {
		write(
			JSON.stringify(
				{
					errors,
					warnings,
					trees: treeReports.map((tree) => ({
						kind: tree.kind,
						directory: tree.directory,
						inputs: tree.inputs,
						error: tree.error,
						warnings: tree.warnings,
						report: tree.report,
					})),
					published: routes.published,
					pages: results.map(({ file, findings }) => ({
						file,
						findings: findings.map(({ edit, ...finding }) => ({
							...finding,
							fixable: edit !== undefined,
						})),
					})),
					render: renderFindings,
				},
				null,
				2,
			),
		);
		return errors > 0 ? 1 : 0;
	}
	const pageCount = routes.pages.length;
	// Page-only runs keep their established summary; typed trees add their own count.
	const checked = routes.trees.length
		? [
				pageCount ? `${pageCount} page${pageCount === 1 ? '' : 's'}` : '',
				`${routes.trees.length} typed tree${routes.trees.length === 1 ? '' : 's'}`,
			]
				.filter(Boolean)
				.join(' and ')
		: `${pageCount} file${pageCount === 1 ? '' : 's'}`;
	write(
		`${checked}: ${errors} error${errors === 1 ? '' : 's'}, ${warnings} warning${warnings === 1 ? '' : 's'}`,
	);
	return errors > 0 ? 1 : 0;
}

process.exitCode = await main().catch((error: unknown): number => {
	process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
	return 1;
});
