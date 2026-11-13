/** Fixed real-input copies and small controls that make each default/opt-in branch reachable. */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import path from 'node:path';

import { z } from 'zod';

import { writeProject } from '../html-evidence/html-fixtures.ts';
import { askingRule, sectionFields } from '../html-sections/fixtures.ts';
import { readObject, writeJson, type JsonObject } from '../rulebook/rulebook-fixture-support.ts';
import { labelLocatorRule } from './label-witness.ts';

export const corpusSchema = z.object({
	groups: z.array(
		z.object({
			name: z.enum(['j1', 'sections', 'anchors', 'ai-tools']),
			config: z.string(),
			config_sha256: z.string(),
			files: z.array(z.object({ file: z.string(), sha256: z.string() })),
			expected_requests: z.number(),
			expected_exit: z.number(),
			git_commit: z.string().nullable(),
		}),
	),
});
export type CorpusGroup = z.infer<typeof corpusSchema>['groups'][number];
export function corpusGroups(): CorpusGroup[] {
	return corpusSchema.parse(readObject(path.join(import.meta.dirname, 'real-corpus.json'))).groups;
}

export function requireInputFiles(input: string, groups: readonly CorpusGroup[]): void {
	for (const group of groups) {
		const root = path.join(input, group.name);
		assert.equal(
			createHash('sha256')
				.update(filesystem.readFileSync(path.join(root, group.config)))
				.digest('hex'),
			group.config_sha256,
		);
		for (const file of group.files)
			assert.equal(
				createHash('sha256')
					.update(filesystem.readFileSync(path.join(root, file.file)))
					.digest('hex'),
				file.sha256,
			);
	}
}

export function prepareCopies(input: string, common: string, groups: readonly CorpusGroup[]): void {
	filesystem.mkdirSync(path.join(common, 'projects'), { recursive: true });
	filesystem.mkdirSync(path.join(common, 'original-configs'));
	filesystem.mkdirSync(path.join(common, 'output'));
	filesystem.mkdirSync(path.join(common, 'locators'));
	for (const group of groups) {
		filesystem.cpSync(path.join(input, group.name), path.join(common, 'projects', group.name), {
			recursive: true,
		});
		filesystem.copyFileSync(
			path.join(input, group.name, group.config),
			path.join(common, 'original-configs', `${group.name}.json`),
		);
	}
}

export function prepareLocator(root: string, files: string[]): void {
	filesystem.mkdirSync(root);
	writeProject(
		root,
		{ K001: mechanical('label', 'raw') },
		{ label: { rule: 'label-attribute', fields: { raw: { from: 'remainder' } } } },
		files,
		labelLocatorRule,
	);
}

function mechanical(evidence: string, field: string): JsonObject {
	return {
		name: 'standard-anchor',
		evidence,
		severity: 'warn',
		decision: {
			start: 'check',
			steps: {
				check: {
					mechanical_check: { tool: evidence === 'link' ? 'anchor_exists' : 'complete', field },
					yes: 'pass',
					['no']: { finding: 'Inspect the standard target.' },
				},
			},
		},
	};
}

export const outlineHtml =
	'<h1>Root</h1><template><h2>Template hidden</h2></template><ex-markdown><h2>Island</h2></ex-markdown><custom-hidden><h2>Generic</h2></custom-hidden><h2>Visible</h2>';
export const askingHtml =
	'<h1>Picture</h1><p>Before</p><ex-mermaid label="CUSTOM_LABEL_015" alt="STD_ALT_015" aria-label="STD_ARIA_015"><p>After</p></ex-mermaid><custom-diagram label="GENERIC_LABEL_015"><p>Tail</p></custom-diagram>';
export const askingTexts = {
	restored:
		'Picture Before [label: CUSTOM_LABEL_015] [alt: STD_ALT_015] [aria-label: STD_ARIA_015] After Tail',
	standard: 'Picture Before [alt: STD_ALT_015] [aria-label: STD_ARIA_015] After Tail',
	generic:
		'Picture Before [alt: STD_ALT_015] [aria-label: STD_ARIA_015] After [label: GENERIC_LABEL_015] Tail',
};

export function prepareControls(root: string): void {
	filesystem.mkdirSync(root);
	const outline = path.join(root, 'outline');
	filesystem.mkdirSync(outline);
	filesystem.writeFileSync(path.join(outline, 'outline.html'), outlineHtml);
	writeProject(
		outline,
		{ K001: mechanical('section', 'full') },
		{
			section: {
				html_units: { mode: 'unit', units: ['section'] },
				fields: sectionFields,
				max_chars: 300_000,
			},
		},
		['outline.html'],
		'id: unused\nlanguage: Html\nrule: {kind: document}\n',
	);
	const asking = path.join(root, 'asking');
	filesystem.mkdirSync(asking);
	filesystem.writeFileSync(path.join(asking, 'asking.html'), askingHtml);
	writeProject(
		asking,
		{
			K001: {
				...askingRule,
				questions: {
					quality: {
						type: 'noul',
						instructions: {
							check: 'full',
							task: 'Read these standard and configured text attributes.',
						},
					},
				},
			},
		},
		{ section: { rule: 'whole', fields: { full: { from: 'remainder' } }, max_chars: 300_000 } },
		['asking.html'],
		'id: whole\nlanguage: Html\nrule: {kind: document}\n',
	);
	const anchors = path.join(root, 'anchors');
	filesystem.mkdirSync(path.join(anchors, 'links'), { recursive: true });
	filesystem.mkdirSync(path.join(anchors, 'targets'));
	filesystem.writeFileSync(
		path.join(anchors, 'targets/target.md'),
		'# Heading text\n\n<div id="explicit-standard"></div>\n',
	);
	filesystem.writeFileSync(
		path.join(anchors, 'targets/target.html'),
		'<p id="html-id">Standard HTML.</p>',
	);
	filesystem.writeFileSync(path.join(anchors, 'targets/target.txt'), 'Unsupported text format.');
	const links = {
		heading: 'target.md#heading-text',
		explicit: 'target.md#explicit-standard',
		'missing-heading': 'target.md#absent',
		html: 'target.html#html-id',
		'missing-html': 'target.html#absent',
		'file-only': 'target.md',
		unsupported: 'target.txt#fragment',
	};
	for (const [name, target] of Object.entries(links))
		filesystem.writeFileSync(
			path.join(anchors, `links/${name}.html`),
			`<a href="../targets/${target}">Inspect standard target.</a>`,
		);
	writeProject(
		anchors,
		{ K001: mechanical('link', 'links') },
		{
			link: {
				rule: 'link',
				fields: { links: { from: 'target', regex: 'href="([^"]+)"', resolve: 'path' } },
			},
		},
		['links/*.html'],
		"id: link\nlanguage: Html\nrule: {kind: element, regex: '^<a\\s+'}\n",
	);
	writeJson(
		path.join(root, 'original-control-configs.json'),
		Object.fromEntries(
			['outline', 'asking', 'anchors'].map((name) => [
				name,
				readObject(path.join(root, name, 'sunstack.lint.json')),
			]),
		),
	);
}
