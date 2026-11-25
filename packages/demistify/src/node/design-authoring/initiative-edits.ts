import { readFile, writeFile } from 'node:fs/promises';

import { buildInitiative } from './check-initiative.ts';

/** Replace `start..end` of a file's text; an insertion has `start === end`. */
export interface TextEdit {
	readonly start: number;
	readonly end: number;
	readonly text: string;
}

export const isoDate = /^\d{4}-\d{2}-\d{2}$/u;

const pad = (value: number): string => String(value).padStart(2, '0');

/** Today's local date as yyyy-mm-dd. */
export function localDate(): string {
	const now = new Date();
	return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
}

export const readText = (file: string): Promise<string> => readFile(file, 'utf8');
export const readOptionalText = (file: string): Promise<string | undefined> =>
	readFile(file, 'utf8').catch((error: unknown) => {
		if (error instanceof Error && 'code' in error && error.code === 'ENOENT') return undefined;
		throw error;
	});

/** Read each file once per command, so every edit is planned against the same text. */
export function textCache(): (file: string) => Promise<string> {
	const texts = new Map<string, Promise<string>>();
	return (file) => {
		const known = texts.get(file) ?? readText(file);
		texts.set(file, known);
		return known;
	};
}

/** Write a data literal in the record file's own style: JSON keys, or bare keys and single quotes. */
export function formatLiteral(value: unknown, indent: string, quotedKeys: boolean): string {
	const unit = indent.includes('\t') ? '\t' : '  ';
	if (typeof value === 'string')
		return quotedKeys
			? JSON.stringify(value)
			: `'${JSON.stringify(value).slice(1, -1).replaceAll('\\"', '"').replaceAll("'", "\\'")}'`;
	if (typeof value === 'boolean') return String(value);
	if (typeof value !== 'object' || value === null)
		throw new Error('A record literal holds only strings, booleans, arrays and objects.');
	const inner = indent + unit;
	if (Array.isArray(value))
		return `[\n${value.map((entry) => `${inner}${formatLiteral(entry, inner, quotedKeys)}`).join(',\n')}\n${indent}]`;
	const entries = Object.entries(value).map(([key, entry]) => {
		const name = quotedKeys ? JSON.stringify(key) : key;
		return `${inner}${name}: ${formatLiteral(entry, inner, quotedKeys)}`;
	});
	return `{\n${entries.join(',\n')}\n${indent}}`;
}

/**
 * Insert `html` before the page's last `</ex-document>` (or `</body>`): on its own line at
 * the closing tag's indentation, or just before the tag when it shares a line.
 */
export function paragraphBeforeDocumentEnd(
	pageText: string,
	label: string,
	html: string,
): TextEdit {
	const close = pageText.lastIndexOf('</ex-document>');
	const insertAt = close === -1 ? pageText.lastIndexOf('</body>') : close;
	if (insertAt === -1)
		throw new Error(`${label} has no </ex-document> or </body> to write before.`);
	const lineStart = pageText.lastIndexOf('\n', insertAt - 1) + 1;
	const indent = pageText.slice(lineStart, insertAt);
	const ownLine = indent.trim() === '';
	const insertionPoint = ownLine ? lineStart : insertAt;
	return { start: insertionPoint, end: insertionPoint, text: `${ownLine ? indent : ''}${html}\n` };
}

/**
 * Write every edit, then run `initiative check --write`. When the check fails, restore
 * each changed file to the text the edits were planned against.
 */
export async function applyEditsThenCheck(
	root: string,
	edits: ReadonlyMap<string, readonly TextEdit[]>,
	textOf: (file: string) => Promise<string>,
	siteRoot: string | undefined,
): Promise<string> {
	const originals = new Map(
		await Promise.all([...edits.keys()].map(async (file) => [file, await textOf(file)] as const)),
	);
	await Promise.all(
		[...edits].map(([file, fileEdits]) => {
			let text = originals.get(file) ?? '';
			for (const edit of fileEdits.toSorted((left, right) => right.start - left.start))
				text = text.slice(0, edit.start) + edit.text + text.slice(edit.end);
			return writeFile(file, text);
		}),
	);
	try {
		return (await buildInitiative(root, { siteRoot, write: true })).result;
	} catch (error) {
		await Promise.all([...originals].map(([file, text]) => writeFile(file, text)));
		throw new Error(
			`The check failed after applying, so every changed file was restored.\n${error instanceof Error ? error.message : String(error)}`,
			{ cause: error },
		);
	}
}
