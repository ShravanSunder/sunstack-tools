import { realpath } from 'node:fs/promises';
import { relative, resolve } from 'node:path';

import typescript from 'typescript';

import {
	assumptionRecord,
	decisionActorField,
	decisionRecord,
} from '../../core/decision-record.ts';
import { escapeHtml } from '../../core/html-text.ts';
import { openEditableTree } from './editable-tree.ts';
import { unwrapExpression } from './read-design.ts';
import {
	applyEditsThenCheck,
	formatLiteral,
	localDate,
	paragraphBeforeDocumentEnd,
	type TextEdit,
	textCache,
} from './record-edits.ts';
import { relativePage } from './record-projection.ts';
import { requiredSectionHolds } from './section-layout.ts';

export const scaffoldKinds = ['choice', 'ruling', 'assumption'] as const;
type ScaffoldKind = (typeof scaffoldKinds)[number];

interface AddRecordOptions {
	/** In an initiative, `initiative` or a project name from initiative.ts; a design set has none. */
	readonly namespace?: string | undefined;
	/** The page that places the record, relative to the working directory. */
	readonly page: string;
	/** The owner's exact words that prompted the record. */
	readonly words?: string | undefined;
	/** The author's alias, for an agent's ruling or an assumption. */
	readonly author?: string | undefined;
	readonly siteRoot?: string | undefined;
	readonly write?: boolean | undefined;
}

const recordHelpers: Readonly<Record<ScaffoldKind, string>> = {
	choice: 'defineChoice',
	ruling: 'defineRuling',
	assumption: 'defineAssumption',
};

/**
 * Append `record` to the file's default-exported array, in the file's own style: a plain
 * literal, or a record helper call (without `kind`) whose import is added when missing.
 */
function appendRecord(
	file: string,
	text: string,
	kind: ScaffoldKind,
	record: Record<string, unknown>,
): TextEdit[] {
	const source = typescript.createSourceFile(file, text, typescript.ScriptTarget.Latest, true);
	const exported = source.statements.find(typescript.isExportAssignment);
	const array = exported && unwrapExpression(exported.expression);
	if (!array || !typescript.isArrayLiteralExpression(array))
		throw new Error(`${file}: expected \`export default [ … ]\`.`);
	const last = array.elements.at(-1);
	if (!last) {
		const literal = formatLiteral(record, '  ', false);
		const start = array.getStart(source) + 1;
		return [{ start, end: start, text: `\n  ${literal}\n` }];
	}
	const { line } = source.getLineAndCharacterOfPosition(last.getStart(source));
	const lineStart = source.getPositionOfLineAndCharacter(line, 0);
	const indent = /^[\t ]*/u.exec(text.slice(lineStart))?.[0] ?? '';
	const lastExpression = unwrapExpression(last);
	const helperCall = typescript.isCallExpression(lastExpression) ? lastExpression : undefined;
	const [helperArgument] = helperCall?.arguments ?? [];
	const lastLiteral = helperArgument ? unwrapExpression(helperArgument) : lastExpression;
	const firstName =
		typescript.isObjectLiteralExpression(lastLiteral) &&
		lastLiteral.properties.find(typescript.isPropertyAssignment)?.name;
	const quotedKeys = Boolean(firstName && typescript.isStringLiteral(firstName));
	if (!helperCall)
		return [
			{
				start: last.getEnd(),
				end: last.getEnd(),
				text: `,\n${indent}${formatLiteral(record, indent, quotedKeys)}`,
			},
		];

	const helper = recordHelpers[kind];
	const fields = Object.fromEntries(Object.entries(record).filter(([key]) => key !== 'kind'));
	const edits: TextEdit[] = [
		{
			start: last.getEnd(),
			end: last.getEnd(),
			text: `,\n${indent}${helper}(${formatLiteral(fields, indent, quotedKeys)})`,
		},
	];
	const sdkImport = source.statements
		.filter(typescript.isImportDeclaration)
		.find(
			({ moduleSpecifier }) =>
				typescript.isStringLiteral(moduleSpecifier) &&
				moduleSpecifier.text.endsWith('design-sdk.generated.ts'),
		);
	const named = sdkImport?.importClause?.namedBindings;
	if (!named || !typescript.isNamedImports(named))
		throw new Error(`${file}: expected \`import { … } from './design-sdk.generated.ts'\`.`);
	const lastSpecifier = named.elements.at(-1);
	if (lastSpecifier && !named.elements.some(({ name }) => name.text === helper))
		edits.push({ start: lastSpecifier.getEnd(), end: lastSpecifier.getEnd(), text: `, ${helper}` });
	return edits;
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
	const tree = await openEditableTree(directory, options.siteRoot);
	const { root } = tree;
	const pageFile = await realpath(resolve(options.page));
	const { namespace } = options;
	const prefix = kind === 'assumption' ? 'A' : 'D';
	const taken = tree.records
		.map(({ id }) => tree.split(id))
		.filter((parts) => parts.namespace === namespace)
		.map(({ localId }) => new RegExp(`^${prefix}(\\d+)$`, 'u').exec(localId))
		.map((match) => (match?.[1] ? Number(match[1]) : 0));
	const localId = `${prefix}${Math.max(0, ...taken) + 1}`;
	const id = tree.qualify(namespace, localId);
	const { file, base, localDirectory } = await tree.newRecordFile(
		namespace,
		kind === 'assumption' ? 'assumptions' : 'decisions',
	);
	// A page inside the record's own project or design set names it by its local ID.
	const insideLocal =
		localDirectory !== undefined && !relative(localDirectory, pageFile).startsWith('..');
	const reference = insideLocal ? localId : id;

	const textOf = textCache();
	const pageText = await textOf(pageFile);
	const pageLabel = relative(root, pageFile);
	const decidedOn = localDate();
	const words = options.words?.trim();
	const paragraphId = `${reference}-source`;
	const source = words
		? {
				href: `${relativePage(base, pageFile)}#${encodeURIComponent(paragraphId)}`,
				excerpt: words,
				verbatim: true as const,
			}
		: undefined;
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

	// The page must declare every section the record can move to, not only where it lands.
	const reachable = requiredSectionHolds(
		kind === 'assumption' ? assumptionRecord.parse(record) : decisionRecord.parse(record),
	);
	const sectionTag = (holds: string): RegExpExecArray | null =>
		new RegExp(`<ex-section\\b[^>]*\\bholds="${holds}"[^>]*>\\s*</ex-section>`, 'u').exec(pageText);
	const missing = reachable.filter((holds) => !sectionTag(holds));
	if (missing.length)
		throw new Error(
			`${pageLabel} needs ${missing.map((holds) => `<ex-section holds="${holds}">`).join(', ')}: ${kind === 'assumption' ? 'an' : 'a'} ${kind} can reach ${reachable.join(', ')}.`,
		);
	const [holds = ''] = reachable;
	const section = sectionTag(holds);
	if (!section) throw new Error(`${pageLabel}: no section holds="${holds}".`);
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
	if (words) {
		if (pageText.includes(`id="${escapeHtml(paragraphId)}"`))
			throw new Error(`${pageLabel} already has #${paragraphId}.`);
		pageEdits.push(
			paragraphBeforeDocumentEnd(
				pageText,
				pageLabel,
				`<p id="${escapeHtml(paragraphId)}">${escapeHtml(id)}. Your words, ${decidedOn}: "${escapeHtml(words)}"</p>`,
			),
		);
	}

	const recordText = await textOf(file);
	const edits = new Map<string, TextEdit[]>([
		[file, appendRecord(file, recordText, kind, record)],
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
	const checked = await applyEditsThenCheck(edits, textOf, tree.checkAndWrite);
	return [...report, 'Added.', checked].join('\n');
}
