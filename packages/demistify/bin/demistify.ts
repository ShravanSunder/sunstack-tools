#!/usr/bin/env node
/**
 * demistify check [--fix] [--render] [--no-diagrams] <page.html …>
 * demistify check --hook      (Claude Code PostToolUse hook: reads the hook event from stdin)
 *
 * Prints one line per problem: file:line:col severity rule: message. Exits 1 when any error remains.
 * In hook mode it stays silent on clean or non-explainer files, and otherwise prints the problems to
 * stderr and exits 2 so the agent sees them and fixes the page.
 */
import { readFile, writeFile } from 'node:fs/promises';
import { dirname, relative, resolve } from 'node:path';

import { applyEdits, type CheckOptions, checkHtml, formatFindings } from '../src/node/check.ts';
import { indexCollectionSearch, type SearchPageSource } from '../src/node/collection-search.ts';
import { embedMarkdownFiles, markdownFileReferences } from '../src/node/markdown-files.ts';
import { renderCheck } from '../src/node/render-check.ts';

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
			'Usage: demistify check [--fix] [--render] [--no-diagrams] <page.html …>\n       demistify check --hook',
		);
		return command ? 1 : 0;
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

	const files = rest.filter((argument): boolean => !argument.startsWith('--'));
	const results = await Promise.all(
		files.map(async (file) => {
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
	let errors = 0;
	let warnings = 0;
	for (const { file, findings } of results) {
		write(formatFindings(file, findings));
		errors += findings.filter((finding): boolean => finding.severity === 'error').length;
		warnings += findings.filter((finding): boolean => finding.severity === 'warning').length;
	}
	if (flags.has('--render') && files.length > 0) {
		for (const finding of await renderCheck(process.cwd(), files)) {
			write(`${finding.file} render(${finding.width}): ${finding.message}`);
			errors += 1;
		}
	}
	write(
		`${files.length} file${files.length === 1 ? '' : 's'}: ${errors} error${errors === 1 ? '' : 's'}, ${warnings} warning${warnings === 1 ? '' : 's'}`,
	);
	return errors > 0 ? 1 : 0;
}

process.exitCode = await main();
