import { readFile, realpath, writeFile } from 'node:fs/promises';
import { join, relative, resolve } from 'node:path';

import typescript from 'typescript';

import type { DecisionRecord } from '../../core/decision-record.ts';
import { escapeHtml } from '../../core/html-text.ts';
import { parseReviewAnswers, type ReviewedAnswer } from '../../core/review-model.ts';
import { buildInitiative } from './check-initiative.ts';
import { readDesignSources } from './read-design.ts';
import { readInitiativeManifest } from './read-initiative.ts';
import { relativePage } from './record-projection.ts';

type ChoiceRecord = Extract<DecisionRecord, { kind: 'choice' }>;

interface ReviewAnswerOptions {
	readonly siteRoot?: string | undefined;
	/** The date the owner decided, as yyyy-mm-dd. Defaults to today. */
	readonly decidedOn?: string | undefined;
	readonly write?: boolean | undefined;
}

/** One decided record: its new authority and the text edits that record it. */
interface AnsweredRecord {
	readonly id: string;
	readonly question: string;
	readonly chosen: ChoiceRecord['options'][number];
	readonly sourceKind: 'owner-words' | 'owner-accepted';
	readonly recordFile: string;
	readonly line: number;
	readonly pageFile: string;
	readonly answerParagraph?: string;
}

interface TextEdit {
	readonly start: number;
	readonly end: number;
	readonly text: string;
}

const isoDate = /^\d{4}-\d{2}-\d{2}$/u;

const pad = (value: number): string => String(value).padStart(2, '0');

function today(): string {
	const now = new Date();
	return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
}

function propertyName(property: typescript.PropertyAssignment): string | undefined {
	const { name } = property;
	return typescript.isIdentifier(name) || typescript.isStringLiteral(name) ? name.text : undefined;
}

interface AuthoritySpan {
	readonly start: number;
	readonly end: number;
	readonly line: number;
	readonly indent: string;
	readonly quotedKeys: boolean;
}

/** Find the `authority` value of the record literal whose `id` is `localId`. */
function findAuthority(file: string, text: string, localId: string): AuthoritySpan[] {
	const source = typescript.createSourceFile(file, text, typescript.ScriptTarget.Latest, true);
	const spans: AuthoritySpan[] = [];
	const visit = (node: typescript.Node): void => {
		if (typescript.isObjectLiteralExpression(node)) {
			const properties = node.properties.filter(typescript.isPropertyAssignment);
			const id = properties.find((property) => propertyName(property) === 'id');
			const authority = properties.find((property) => propertyName(property) === 'authority');
			if (
				id &&
				authority &&
				typescript.isStringLiteral(id.initializer) &&
				id.initializer.text === localId
			) {
				const start = authority.initializer.getStart(source);
				const { line } = source.getLineAndCharacterOfPosition(authority.getStart(source));
				const lineStart = source.getPositionOfLineAndCharacter(line, 0);
				spans.push({
					start,
					end: authority.initializer.getEnd(),
					line: line + 1,
					indent: /^[\t ]*/u.exec(text.slice(lineStart))?.[0] ?? '',
					quotedKeys: typescript.isStringLiteral(authority.name),
				});
			}
		}
		typescript.forEachChild(node, visit);
	};
	visit(source);
	return spans;
}

/** Write a data literal in the record file's own style: JSON keys, or bare keys and single quotes. */
function formatLiteral(value: unknown, indent: string, quotedKeys: boolean): string {
	const unit = indent.includes('\t') ? '\t' : '  ';
	if (typeof value === 'string')
		return quotedKeys
			? JSON.stringify(value)
			: `'${JSON.stringify(value).slice(1, -1).replaceAll('\\"', '"').replaceAll("'", "\\'")}'`;
	if (typeof value === 'boolean') return String(value);
	if (typeof value !== 'object' || value === null || Array.isArray(value))
		throw new Error('An authority holds only strings, booleans and objects.');
	const inner = indent + unit;
	const entries = Object.entries(value).map(([key, entry]) => {
		const name = quotedKeys ? JSON.stringify(key) : key;
		return `${inner}${name}: ${formatLiteral(entry, inner, quotedKeys)}`;
	});
	return `{\n${entries.join(',\n')}\n${indent}}`;
}

function chosenOption(
	answer: ReviewedAnswer,
	record: ChoiceRecord,
): ChoiceRecord['options'][number] {
	const [selected, ...extra] = answer.selected;
	if (!selected || extra.length)
		throw new Error(
			`${record.id}: select exactly one option; the review selects ${answer.selected.length}.`,
		);
	const prefix = `${answer.questionId}--`;
	const option = record.options.find((candidate) =>
		selected.optionId.startsWith(prefix)
			? selected.optionId.slice(prefix.length) === candidate.id
			: selected.optionId === candidate.id || selected.label === candidate.title,
	);
	if (!option)
		throw new Error(
			`${record.id}: the review selects ${selected.optionId}, which is not one of its options.`,
		);
	return option;
}

const readText = (file: string): Promise<string> => readFile(file, 'utf8');
const readOptionalText = (file: string): Promise<string | undefined> =>
	readFile(file, 'utf8').catch((error: unknown) => {
		if (error instanceof Error && 'code' in error && error.code === 'ENOENT') return undefined;
		throw error;
	});

async function recordFiles(
	root: string,
	namespace: string,
	projects: ReadonlyMap<string, string>,
): Promise<{ base: string; files: string[] }> {
	if (namespace === 'initiative') {
		const initiativeFile = join(root, 'initiative.ts');
		const manifest = readInitiativeManifest(initiativeFile, await readFile(initiativeFile, 'utf8'));
		return { base: root, files: manifest.decisions ? [manifest.decisions] : [] };
	}
	const directory = [...projects].find(([, name]) => name === namespace)?.[0];
	if (!directory) throw new Error(`No project named ${namespace} in this initiative.`);
	const { designFile, sources } = await readDesignSources(directory, readText, readOptionalText);
	return { base: directory, files: [designFile, ...sources.keys()] };
}

/**
 * Record the owner's review answers on their open choices: the authority, the
 * owner's words on the page that places the record, and refreshed generated views.
 * Without `write` it only reports what it would change.
 */
export async function applyReviewAnswers(
	reviewText: string,
	directory: string,
	options: ReviewAnswerOptions = {},
): Promise<string> {
	const decidedOn = options.decidedOn ?? today();
	if (!isoDate.test(decidedOn))
		throw new Error(`--decided-on takes a date as yyyy-mm-dd, not ${decidedOn}.`);
	const review = parseReviewAnswers(reviewText);
	const root = await realpath(resolve(directory));
	const build = await buildInitiative(root, { siteRoot: options.siteRoot });
	const records = new Map(build.design.records.map((record) => [record.id, record]));

	const texts = new Map<string, Promise<string>>();
	const textOf = (file: string): Promise<string> => {
		const known = texts.get(file) ?? readText(file);
		texts.set(file, known);
		return known;
	};

	type PlannedAnswer =
		| { readonly note: string }
		| { readonly answered: AnsweredRecord; readonly edits: readonly [string, TextEdit][] };
	const planAnswer = async (answer: ReviewedAnswer): Promise<PlannedAnswer> => {
		const id = answer.decisionId;
		if (!id)
			return { note: `Skipped ${answer.questionId}: it is not a typed record (no Decision ID).` };
		const record = records.get(id);
		if (!record) throw new Error(`${id}: no record with this ID in the initiative.`);
		if (record.kind !== 'choice') throw new Error(`${id}: only a choice takes a review answer.`);
		if (record.authority.state !== 'owner-open')
			throw new Error(`${id}: already decided (${record.authority.state}); nothing to apply.`);
		const option = chosenOption(answer, record);
		const slash = id.indexOf('/');
		if (slash === -1)
			throw new Error(`${id}: expected a qualified record ID such as initiative/D1.`);
		const namespace = id.slice(0, slash);
		const localId = id.slice(slash + 1);
		const placement = build.placements.get(id);
		if (!placement) throw new Error(`${id}: the record is not placed on a page.`);
		const { base, files } = await recordFiles(root, namespace, build.projects);
		const spans = (
			await Promise.all(
				files.map(async (file) =>
					findAuthority(file, await textOf(file), localId).map((span) => ({ file, span })),
				),
			)
		).flat();
		const [found, ...duplicates] = spans;
		if (!found || duplicates.length)
			throw new Error(`${id}: expected one record literal for ${localId}, found ${spans.length}.`);

		const targets = new Set([
			answer.questionId,
			...answer.selected.map(({ optionId }) => optionId),
		]);
		const words = answer.comments
			.filter(({ targetId, body }) => targets.has(targetId) && body)
			.map(({ body }) => body)
			.join(' ');
		const page = relativePage(base, placement.file);
		const edits: [string, TextEdit][] = [];
		let answerParagraph: string | undefined;
		let source: { href: string; excerpt: string; verbatim?: true };
		if (words) {
			const paragraphId = `${localId}-answer`;
			const pageText = await textOf(placement.file);
			if (pageText.includes(`id="${paragraphId}"`))
				throw new Error(`${id}: ${relative(root, placement.file)} already has #${paragraphId}.`);
			const close = pageText.lastIndexOf('</ex-document>');
			const insertAt = close === -1 ? pageText.lastIndexOf('</body>') : close;
			if (insertAt === -1)
				throw new Error(
					`${id}: ${relative(root, placement.file)} has no </ex-document> or </body> to write the answer before.`,
				);
			answerParagraph = `<p id="${paragraphId}">Decision ${escapeHtml(id)}. Your words, ${decidedOn}, in review ${escapeHtml(review.reviewId)}: "${escapeHtml(words)}"</p>`;
			// On its own line, at the closing tag's indentation; otherwise just before the tag.
			const lineStart = pageText.lastIndexOf('\n', insertAt - 1) + 1;
			const indent = pageText.slice(lineStart, insertAt);
			const ownLine = indent.trim() === '';
			const paragraphAt = ownLine ? lineStart : insertAt;
			edits.push([
				placement.file,
				{
					start: paragraphAt,
					end: paragraphAt,
					text: `${ownLine ? indent : ''}${answerParagraph}\n`,
				},
			]);
			source = { href: `${page}#${paragraphId}`, excerpt: words, verbatim: true };
		} else
			source = { href: `${page}#${encodeURIComponent(placement.fragment)}`, excerpt: option.title };
		const sourceKind = words ? 'owner-words' : 'owner-accepted';
		const authority = {
			state: 'owner-decided',
			sourceKind,
			decidedOn,
			chosen: option.id,
			decision: option.title,
			source,
		};
		edits.push([
			found.file,
			{
				start: found.span.start,
				end: found.span.end,
				text: formatLiteral(authority, found.span.indent, found.span.quotedKeys),
			},
		]);
		return {
			answered: {
				id,
				question: record.question,
				chosen: option,
				sourceKind,
				recordFile: found.file,
				line: found.span.line,
				pageFile: placement.file,
				...(answerParagraph && { answerParagraph }),
			},
			edits,
		};
	};

	const planned = await Promise.all(review.answers.map(planAnswer));
	const notes = planned.flatMap((item) => ('note' in item ? [item.note] : []));
	const answered = planned.flatMap((item) => ('answered' in item ? [item.answered] : []));
	const edits = new Map<string, TextEdit[]>();
	for (const item of planned)
		if ('edits' in item)
			for (const [file, edit] of item.edits) edits.set(file, [...(edits.get(file) ?? []), edit]);

	const report = [
		...answered.flatMap((item) => [
			`${item.id} · ${item.question}`,
			`  chosen: ${item.chosen.id} (${item.chosen.title}) · ${item.sourceKind} · ${decidedOn}`,
			`  record: ${relative(root, item.recordFile)}:${item.line} (authority replaced)`,
			...(item.answerParagraph
				? [`  page: ${relative(root, item.pageFile)} gains ${item.answerParagraph}`]
				: []),
		]),
		...notes,
	];
	if (!answered.length) return [...report, 'No typed choice to record.'].join('\n');
	if (!options.write)
		return [...report, 'Preview only; nothing written. Run again with --write to apply.'].join(
			'\n',
		);

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
		const checked = await buildInitiative(root, { siteRoot: options.siteRoot, write: true });
		return [...report, 'Applied.', checked.result].join('\n');
	} catch (error) {
		await Promise.all([...originals].map(([file, text]) => writeFile(file, text)));
		throw new Error(
			`The check failed after applying, so every changed file was restored.\n${error instanceof Error ? error.message : String(error)}`,
			{ cause: error },
		);
	}
}
