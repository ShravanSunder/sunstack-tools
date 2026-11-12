/** Authored real-set rules and labelled controls for cases the real set cannot demonstrate. */
import filesystem from 'node:fs';
import path from 'node:path';

import { writeJson, type JsonObject } from '../rulebook/rulebook-fixture-support.ts';
import { snapshotRealSet } from './html-corpus.ts';

export const htmlConfig = 'sunstack.lint.json';
function rule(name: string, evidence: string, tool: JsonObject): JsonObject {
	return {
		name,
		evidence,
		severity: 'error',
		decision: {
			start: 'check',
			steps: {
				check: { mechanical_check: tool, yes: 'pass', ['no']: { finding: `Inspect ${name}.` } },
			},
		},
	};
}
export function writeProject(
	root: string,
	rules: JsonObject,
	kinds: JsonObject,
	files: string[],
	definitions: string,
): void {
	filesystem.mkdirSync(path.join(root, 'evidence/rules'), { recursive: true });
	filesystem.writeFileSync(path.join(root, 'evidence/sgconfig.yml'), 'ruleDirs: [rules]\n');
	filesystem.writeFileSync(path.join(root, 'evidence/rules/html.yml'), definitions);
	filesystem.writeFileSync(
		path.join(root, 'rulebook.md'),
		`${Object.keys(rules)
			.map((marker) => `Inspect authored document evidence. <!-- ${marker} -->`)
			.join('\n\n')}\n`,
	);
	writeJson(path.join(root, 'rules.json'), rules);
	writeJson(path.join(root, htmlConfig), {
		rulebook: [{ markdown: 'rulebook.md', rules: 'rules.json' }],
		marker_prefix: 'K',
		check: { files },
		evidence: { ast_grep: 'evidence/sgconfig.yml', kinds },
		decision_model: { model: 'jev-latest' },
	});
}
const basicRules =
	"id: whole-document\nlanguage: Html\nrule: {kind: document}\n---\nid: link\nlanguage: Html\nrule:\n  kind: element\n  regex: '^<a\\s+'\n";

export function prepareRealFixture(options: { root: string; source: string }): void {
	snapshotRealSet({ source: options.source, destination: path.join(options.root, 'pages') });
	const definitions = `${basicRules}---\nid: requirement-row\nlanguage: Html\nrule: {kind: element, regex: '^<tr\\s+id="R[0-9]+"'}\n---\nid: entity-row\nlanguage: Html\nrule: {kind: element, regex: '^<tr\\s+id="E[0-9]+"'}\n---\nid: verification-row\nlanguage: Html\nrule:\n  all:\n    - {kind: element, regex: '^<tr><td>U[0-9]+;'}\n    - inside:\n        kind: element\n        regex: '^<ex-document\\s+id="document"'\n        has: {kind: start_tag, regex: '^<ex-document\\s+id="document"', stopBy: end}\n        stopBy: end\n---\nid: binding-row\nlanguage: Html\nrule: {kind: element, regex: '^<tr><td>E[0-9]'}\n`;
	writeProject(
		options.root,
		{
			K001: rule('requirements-covered', 'requirement', {
				tool: 'contains_field',
				field: 'coverage',
				needle: 'identifier',
			}),
			K002: rule('entities-bound', 'entity', {
				tool: 'contains_field',
				field: 'coverage',
				needle: 'identifier',
			}),
		},
		{
			requirement: {
				rule: 'requirement-row',
				fields: {
					identifier: { from: 'capture', regex: 'id="(R[0-9]+)"' },
					body: { from: 'remainder' },
					coverage: {
						from: 'document',
						path: 'verification-design.html',
						rule: 'verification-row',
					},
				},
			},
			entity: {
				rule: 'entity-row',
				fields: {
					identifier: { from: 'capture', regex: 'id="(E[0-9]+)"' },
					body: { from: 'remainder' },
					coverage: { from: 'document', path: 'program-design.html', rule: 'binding-row' },
				},
			},
		},
		['pages/specification.html'],
		definitions,
	);
}

export interface ExpectedControl {
	readonly file: string;
	readonly outcome: 'pass' | 'finding' | 'incomplete';
	readonly reason: string;
}
export function prepareRealForward(options: { root: string; source: string }): void {
	snapshotRealSet({ source: options.source, destination: path.join(options.root, 'pages') });
	writeProject(
		options.root,
		{ K001: rule('real-fragments', 'link', { tool: 'anchor_exists', field: 'links' }) },
		{
			link: {
				rule: 'fragment-link',
				fields: { links: { from: 'target', regex: 'href="([^"]*#[^"]*)"', resolve: 'path' } },
			},
		},
		['pages/*.html'],
		`${basicRules}---\nid: fragment-link\nlanguage: Html\nrule: {kind: element, regex: '^<a\\s+[^>]*href="[^"]*#'}\n`,
	);
}
export function prepareTokenFixture(root: string): ExpectedControl[] {
	const cases = [
		['start', 'R1'],
		['punctuation', '(R1)'],
		['slash', 'R1 / U1'],
		['longer', 'R10'],
		['prefix', 'XR1'],
		['underscore-before', '_R1'],
		['underscore-after', 'R1_extra'],
		['unicode-letter', '文R1'],
		['unicode-digit', 'R1٢'],
	] as const;
	const expected: ExpectedControl[] = [];
	for (const [index, [name, target]] of cases.entries()) {
		const directory = path.join(root, 'cases', name);
		filesystem.mkdirSync(directory, { recursive: true });
		filesystem.writeFileSync(
			path.join(directory, 'source.html'),
			'<p data-needle="R1">An authored token.</p>\n',
		);
		filesystem.writeFileSync(
			path.join(directory, 'target.html'),
			`<div id="coverage">${target}</div>\n`,
		);
		expected.push({
			file: `cases/${name}/source.html`,
			outcome: index < 3 ? 'pass' : 'finding',
			reason: `whole-token ${name}`,
		});
	}
	writeProject(
		root,
		{
			K001: rule('whole-token', 'token', {
				tool: 'contains_field',
				field: 'coverage',
				needle: 'identifier',
			}),
		},
		{
			token: {
				rule: 'token',
				fields: {
					identifier: { from: 'capture', regex: 'data-needle="([^"]*)"' },
					coverage: { from: 'document', path: 'target.html', rule: 'coverage' },
				},
			},
		},
		['cases/*/source.html'],
		`${basicRules}---\nid: token\nlanguage: Html\nrule: {kind: element, regex: '^<p\\s+data-needle='}\n---\nid: coverage\nlanguage: Html\nrule: {kind: element, regex: '^<div\\s+id="coverage"'}\n`,
	);
	return expected;
}

export function prepareAnchorFixture(root: string): ExpectedControl[] {
	const target =
		'<!doctype html><p id="present">Present.</p><p id="A&amp;B">Entity.</p><p id="N&#49;">Numeric.</p><p id="A B">Percent.</p><p id="猫">Unicode.</p><p id="A&amp B">Legacy raw.</p><pre>&lt;p id="fake"&gt;Example&lt;/p&gt;</pre>';
	filesystem.mkdirSync(path.join(root, 'pages'), { recursive: true });
	filesystem.writeFileSync(path.join(root, 'pages/target.html'), target);
	filesystem.writeFileSync(path.join(root, 'pages/target.md'), '# Heading\n');
	filesystem.writeFileSync(path.join(root, 'pages/entity&path.html'), target);
	filesystem.writeFileSync(path.join(root, 'pages/percent path.html'), target);
	const cases: readonly [string, string, ExpectedControl['outcome']][] = [
		['found', 'target.html#present', 'pass'],
		['absent', 'target.html#absent', 'finding'],
		['escaped-fake', 'target.html#fake', 'finding'],
		['missing-file', 'not-created.html#present', 'finding'],
		['file-only', 'target.html', 'pass'],
		['unsupported', 'target.md#heading', 'incomplete'],
		['entity', 'target.html#A&amp;B', 'pass'],
		['numeric', 'target.html#N1', 'pass'],
		['percent-fragment', 'target.html#A%20B', 'pass'],
		['percent-unicode', 'target.html#%E7%8C%AB', 'pass'],
		['entity-path', 'entity&amp;path.html#present', 'pass'],
		['percent-path', 'percent%20path.html#present', 'pass'],
		['same-file', '#self', 'pass'],
		['legacy-raw', 'target.html#A&amp B', 'pass'],
		['legacy-normalized', 'target.html#A& B', 'finding'],
	];
	const expected: ExpectedControl[] = [];
	for (const [name, href, outcome] of cases) {
		const file = `pages/${name}.html`;
		filesystem.writeFileSync(
			path.join(root, file),
			`<a id="self" href="${href}">Inspect this link.</a>\n`,
		);
		expected.push({ file, outcome, reason: `anchor ${name}` });
	}
	writeProject(
		root,
		{ K001: rule('anchors-resolve', 'link', { tool: 'anchor_exists', field: 'links' }) },
		{
			link: {
				rule: 'link',
				fields: {
					links: { from: 'target', regex: 'href="([^"]+)"', resolve: 'path', opening_chars: 5 },
				},
			},
		},
		cases.map(([name]) => `pages/${name}.html`),
		basicRules,
	);
	return expected;
}

export function prepareAskingFixture(root: string): void {
	filesystem.mkdirSync(path.join(root, 'pages'), { recursive: true });
	filesystem.writeFileSync(
		path.join(root, 'pages/asking.html'),
		'<p data-secret="ATTRIBUTE_ONLY_SENTINEL">Café &amp; <b>Unicode 文</b>  body.</p>\n',
	);
	const asking: JsonObject = {
		name: 'html-asking',
		evidence: 'paragraph',
		severity: 'warn',
		questions: {
			quality: { type: 'noul', instructions: { check: 'body', task: 'Is this text clear?' } },
		},
		decision: {
			start: 'quality',
			steps: {
				quality: {
					ask: 'quality',
					yes_at_or_above: 0.8,
					no_at_or_below: 0.2,
					yes: 'pass',
					['no']: { finding: 'Inspect clarity.' },
					uncertain: 'uncertain',
				},
			},
		},
	};
	writeProject(
		root,
		{ K001: asking },
		{ paragraph: { rule: 'paragraph', fields: { body: { from: 'remainder' } } } },
		['pages/asking.html'],
		`${basicRules}---\nid: paragraph\nlanguage: Html\nrule: {kind: element, regex: '^<p\\s+data-secret='}\n`,
	);
}

export function prepareRealAsking(options: { root: string; source: string }): void {
	snapshotRealSet({ source: options.source, destination: path.join(options.root, 'pages') });
	const asking: JsonObject = {
		name: 'real-html-asking',
		evidence: 'requirement',
		severity: 'warn',
		questions: {
			quality: {
				type: 'noul',
				instructions: { check: 'body', task: 'Is this requirement clear?' },
			},
		},
		decision: {
			start: 'quality',
			steps: {
				quality: {
					ask: 'quality',
					yes_at_or_above: 0.8,
					no_at_or_below: 0.2,
					yes: 'pass',
					['no']: { finding: 'Inspect clarity.' },
					uncertain: 'uncertain',
				},
			},
		},
	};
	writeProject(
		options.root,
		{ K001: asking },
		{ requirement: { rule: 'one-requirement', fields: { body: { from: 'remainder' } } } },
		['pages/specification.html'],
		`${basicRules}---\nid: one-requirement\nlanguage: Html\nrule: {kind: element, regex: '^<tr\\s+id="R1"'}\n`,
	);
}

export function prepareGapFixture(
	root: string,
	gap: 'empty-needle' | 'missing-document' | 'empty-target',
): ExpectedControl[] {
	const directory = path.join(root, 'pages');
	filesystem.mkdirSync(directory, { recursive: true });
	filesystem.writeFileSync(
		path.join(directory, 'source.html'),
		`<p data-needle="${gap === 'empty-needle' ? '' : 'R1'}" href="">Authored gap control.</p>\n`,
	);
	filesystem.writeFileSync(path.join(directory, 'target.html'), '<div id="coverage">R1</div>\n');
	writeProject(
		root,
		{
			K001: rule(
				'gap-is-incomplete',
				'token',
				gap === 'empty-target'
					? { tool: 'anchor_exists', field: 'links' }
					: { tool: 'contains_field', field: 'coverage', needle: 'identifier' },
			),
		},
		{
			token: {
				rule: 'token',
				fields: {
					identifier: { from: 'capture', regex: 'data-needle="([^"]*)"' },
					links: { from: 'target', regex: 'href="([^"]*)"', resolve: 'path' },
					coverage: {
						from: 'document',
						path: gap === 'missing-document' ? 'absent.html' : 'target.html',
						rule: 'coverage',
					},
				},
			},
		},
		['pages/source.html'],
		`${basicRules}---\nid: token\nlanguage: Html\nrule: {kind: element, regex: '^<p\\s+data-needle='}\n---\nid: coverage\nlanguage: Html\nrule: {kind: element, regex: '^<div\\s+id="coverage"'}\n`,
	);
	return [{ file: 'pages/source.html', outcome: 'incomplete', reason: gap }];
}
