import { realpath } from 'node:fs/promises';
import { dirname, join, relative, resolve } from 'node:path';

import typescript from 'typescript';

import { decisionActorField } from '../../core/decision-record.ts';
import { escapeHtml } from '../../core/html-text.ts';
import { buildInitiative } from './check-initiative.ts';
import {
	applyEditsThenCheck,
	formatLiteral,
	localDate,
	paragraphBeforeDocumentEnd,
	readOptionalText,
	readText,
	type TextEdit,
	textCache,
} from './initiative-edits.ts';
import { unwrapExpression } from './read-design.ts';
import { readInitiativeManifest } from './read-initiative.ts';
import { relativePage } from './record-projection.ts';

export const scaffoldKinds = ['choice', 'ruling', 'assumption'] as const;
type ScaffoldKind = (typeof scaffoldKinds)[number];

interface AddRecordOptions {
	/** `initiative` or a project name from initiative.ts. */
	readonly namespace: string;
	/** The page that places the record, relative to the working directory. */
	readonly page: string;
	/** The owner's exact words that prompted the record. */
	readonly words?: string | undefined;
	/** The author's alias, for an agent's ruling or an assumption. */
	readonly author?: string | undefined;
	readonly siteRoot?: string | undefined;
	readonly write?: boolean | undefined;
}

/** The record file and the directory its links resolve from. */
async function targetRecordFile(
	root: string,
	namespace: string,
	kind: ScaffoldKind,
	projects: ReadonlyMap<string, string>,
): Promise<{ file: string; base: string; projectDirectory?: string }> {
	const field = kind === 'assumption' ? 'assumptions' : 'decisions';
	if (namespace === 'initiative') {
		const initiativeFile = join(root, 'initiative.ts');
		const manifest = readInitiativeManifest(initiativeFile, await readText(initiativeFile));
		const file = manifest[field];
		if (!file) throw new Error(`initiative.ts registers no ${field} file; add one first.`);
		return { file, base: dirname(file) };
	}
	const directory = [...projects].find(([, name]) => name === namespace)?.[0];
	if (!directory) throw new Error(`No project named ${namespace} in this initiative.`);
	const file = join(directory, `design-${field}.ts`);
	if ((await readOptionalText(file)) === undefined)
		throw new Error(
			`${relative(root, file)} does not exist. Create it with \`export default [];\`, or add the record by hand to another record file.`,
		);
	return { file, base: directory, projectDirectory: directory };
}

/** Append `record` to the file's default-exported array, in the file's own style. */
function appendRecord(file: string, text: string, record: Record<string, unknown>): TextEdit {
	const source = typescript.createSourceFile(file, text, typescript.ScriptTarget.Latest, true);
	const exported = source.statements.find(typescript.isExportAssignment);
	const array = exported && unwrapExpression(exported.expression);
	if (!array || !typescript.isArrayLiteralExpression(array))
		throw new Error(`${file}: expected \`export default [ … ]\`.`);
	if (array.elements.some((element) => typescript.isCallExpression(unwrapExpression(element))))
		throw new Error(`${file} uses record helpers; add the record by hand.`);
	const last = array.elements.at(-1);
	if (!last) {
		const literal = formatLiteral(record, '  ', false);
		const start = array.getStart(source) + 1;
		return { start, end: start, text: `\n  ${literal}\n` };
	}
	const { line } = source.getLineAndCharacterOfPosition(last.getStart(source));
	const lineStart = source.getPositionOfLineAndCharacter(line, 0);
	const indent = /^[\t ]*/u.exec(text.slice(lineStart))?.[0] ?? '';
	const lastLiteral = unwrapExpression(last);
	const firstName =
		typescript.isObjectLiteralExpression(lastLiteral) &&
		lastLiteral.properties.find(typescript.isPropertyAssignment)?.name;
	const quotedKeys = Boolean(firstName && typescript.isStringLiteral(firstName));
	return {
		start: last.getEnd(),
		end: last.getEnd(),
		text: `,\n${indent}${formatLiteral(record, indent, quotedKeys)}`,
	};
}

/** The first section a new record lands in. */
function firstSection(kind: ScaffoldKind, worded: boolean): string {
	if (kind === 'choice') return 'needs-owner';
	if (kind === 'assumption') return 'assumed';
	return worded ? 'settled' : 'decided-by-agent';
}

function choiceOption(id: string): string {
	return `<ex-option data-option="${id}"><template slot="summary">TODO: ${id} in one sentence.</template></ex-option>`;
}

function sectionItem(kind: ScaffoldKind, reference: string): string {
	const attribute = escapeHtml(reference);
	if (kind !== 'choice')
		return `<ex-section-item record="${attribute}"><p>TODO: explain ${attribute} here.</p></ex-section-item>`;
	return `<ex-section-item record="${attribute}"><ex-choice>${choiceOption('option-a')}${choiceOption('option-b')}</ex-choice></ex-section-item>`;
}

/**
 * Scaffold a choice, ruling or assumption in one step: the record with `TODO:` placeholders,
 * its section item on the page, and the owner's words when given. Without `write` it only
 * reports what it would add.
 */
export async function addRecord(
	kind: ScaffoldKind,
	directory: string,
	options: AddRecordOptions,
): Promise<string> {
	const worded = Boolean(options.words?.trim());
	if (kind === 'assumption' && !options.author)
		throw new Error('An assumption needs --author <alias> for the agent who holds it.');
	if (kind === 'ruling' && !worded && !options.author)
		throw new Error("An agent's ruling needs --author <alias>, or --words with the owner's words.");
	const root = await realpath(resolve(directory));
	const pageFile = await realpath(resolve(options.page));
	const build = await buildInitiative(root, { siteRoot: options.siteRoot });
	const { namespace } = options;
	const prefix = kind === 'assumption' ? 'A' : 'D';
	const taken = build.design.records
		.filter(({ id }) => id.startsWith(`${namespace}/`))
		.map(({ id }) => new RegExp(`^${prefix}(\\d+)$`, 'u').exec(id.slice(namespace.length + 1)))
		.map((match) => (match?.[1] ? Number(match[1]) : 0));
	const localId = `${prefix}${Math.max(0, ...taken) + 1}`;
	const id = `${namespace}/${localId}`;
	const { file, base, projectDirectory } = await targetRecordFile(
		root,
		namespace,
		kind,
		build.projects,
	);
	const insideProject =
		projectDirectory !== undefined && !relative(projectDirectory, pageFile).startsWith('..');
	const reference = insideProject ? localId : id;

	const textOf = textCache();
	const pageText = await textOf(pageFile);
	const pageLabel = relative(root, pageFile);
	const holds = firstSection(kind, worded);
	const section = new RegExp(
		`<ex-section\\b[^>]*\\bholds="${holds}"[^>]*>\\s*</ex-section>`,
		'u',
	).exec(pageText);
	if (!section)
		throw new Error(
			`${pageLabel} has no <ex-section holds="${holds}">. Declare the sections a ${kind} can reach first.`,
		);
	const sectionLineStart = pageText.lastIndexOf('\n', section.index - 1) + 1;
	const sectionIndent = /^[\t ]*/u.exec(pageText.slice(sectionLineStart))?.[0] ?? '';
	const afterSection = section.index + section[0].length;
	const pageEdits: TextEdit[] = [
		{
			start: afterSection,
			end: afterSection,
			text: `\n${sectionIndent}${sectionItem(kind, reference)}`,
		},
	];

	const decidedOn = localDate();
	const words = options.words?.trim();
	let source: { href: string; excerpt: string; verbatim: true } | undefined;
	if (words) {
		const paragraphId = `${reference}-source`;
		if (pageText.includes(`id="${escapeHtml(paragraphId)}"`))
			throw new Error(`${pageLabel} already has #${paragraphId}.`);
		pageEdits.push(
			paragraphBeforeDocumentEnd(
				pageText,
				pageLabel,
				`<p id="${escapeHtml(paragraphId)}">${escapeHtml(id)}. Your words, ${decidedOn}: "${escapeHtml(words)}"</p>`,
			),
		);
		source = {
			href: `${relativePage(base, pageFile)}#${encodeURIComponent(paragraphId)}`,
			excerpt: words,
			verbatim: true,
		};
	}
	const author = options.author ? { alias: options.author, role: 'lead' } : undefined;
	const identity = { id: localId, title: `TODO: a short title for ${localId}` };
	const sources = source ? { sources: [source] } : {};
	const record: Record<string, unknown> =
		kind === 'choice'
			? {
					...identity,
					...sources,
					kind,
					question: 'TODO: the question the owner answers?',
					options: [
						{ id: 'option-a', title: 'TODO: the first option' },
						{ id: 'option-b', title: 'TODO: the second option' },
					],
					recommended: 'none',
					authority: { state: 'owner-open' },
				}
			: kind === 'ruling'
				? {
						...identity,
						...sources,
						kind,
						statement: 'TODO: the rule, in one sentence.',
						authority: source
							? { state: 'owner-decided', sourceKind: 'owner-words', decidedOn, source }
							: {
									state: 'agent-decided',
									[decisionActorField]: author,
									sourceKind: 'author',
									rationale: 'TODO: why this holds.',
									decidedOn,
									weight: 'design',
								},
					}
				: {
						...identity,
						...sources,
						kind,
						statement: 'TODO: what is assumed, in one sentence.',
						[decisionActorField]: author,
						weight: 'design',
						status: 'open',
						settles: 'TODO: the observation that would settle it.',
					};

	const recordText = await textOf(file);
	const edits = new Map<string, TextEdit[]>([
		[file, [appendRecord(file, recordText, record)]],
		[pageFile, pageEdits],
	]);
	const report = [
		`${id} · new ${kind}${words ? ' from the owner’s words' : ''}`,
		`  record: appended to ${relative(root, file)}`,
		`  page: ${pageLabel} gains <ex-section-item record="${reference}"> after its holds="${holds}" section${words ? ` and <p id="${reference}-source">` : ''}`,
		`  fill in every TODO: in ${relative(root, file)} (${localId}) and in its item on ${pageLabel}`,
	];
	if (!options.write)
		return [...report, 'Preview only; nothing written. Run again with --write to apply.'].join(
			'\n',
		);
	const checked = await applyEditsThenCheck(root, edits, textOf, options.siteRoot);
	return [...report, 'Added.', checked].join('\n');
}
