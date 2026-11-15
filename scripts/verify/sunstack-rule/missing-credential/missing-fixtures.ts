/** Small synthetic source/definitions; expected labels never enter execution bindings. */
import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import path from 'node:path';

import {
	baselineGit,
	gitCommand,
	writeJson,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';

export const sourceText = 'Useful local café.\n';
export const sourceDigest = createHash('sha256').update(sourceText).digest('hex');
export const sourceFile = 'docs/one.md';
export const localNames = ['local_first', 'local_second', 'local_unreached'] as const;
export const judgedNames = ['judged_first', 'judged_second'] as const;
export type FixtureMode = 'mechanical' | 'mixed' | 'filter' | 'selection' | 'invalid';

function question(): JsonObject {
	return {
		verdict: {
			type: 'noul',
			instructions: { check: 'body', task: 'Does this synthetic paragraph comply?' },
		},
	};
}
function mechanical(name: string, field = 'body'): JsonObject {
	return {
		name,
		evidence: 'paragraph',
		severity: 'error',
		decision: {
			start: 'local',
			steps: {
				local: {
					mechanical_check: { tool: 'regex', field, pattern: '^NEVER' },
					yes: 'pass',
					['no']: { finding: `Original ${name} finding.`, ['at']: field },
				},
			},
		},
	};
}
function judged(name: string): JsonObject {
	return {
		name,
		evidence: 'paragraph',
		severity: 'error',
		questions: question(),
		decision: {
			start: 'judge',
			steps: {
				judge: {
					ask: 'verdict',
					yes_at_or_above: 0.8,
					no_at_or_below: 0.2,
					yes: 'pass',
					['no']: { finding: `Judged ${name}.` },
					uncertain: 'uncertain',
				},
			},
		},
	};
}
export function writeFixture(root: string, endpoint: string, mode: FixtureMode): void {
	for (const directory of ['docs', 'evidence/rules'])
		filesystem.mkdirSync(path.join(root, directory), { recursive: true });
	filesystem.writeFileSync(path.join(root, sourceFile), sourceText);
	filesystem.writeFileSync(path.join(root, 'evidence/sgconfig.yml'), 'ruleDirs: [rules]\n');
	filesystem.writeFileSync(
		path.join(root, 'evidence/rules/paragraph.yml'),
		'id: paragraph\nlanguage: Markdown\nrule: {kind: paragraph}\n',
	);
	const entries: JsonObject = Object.fromEntries(
		localNames.map((name, index) => [`K00${index + 1}`, mechanical(name)]),
	);
	// This authored question is never reached by the mechanical graph.
	const unreached = entries['K003'];
	if (unreached !== null && typeof unreached === 'object' && !Array.isArray(unreached))
		unreached['questions'] = question();
	if (mode !== 'mechanical')
		for (const [index, name] of judgedNames.entries()) entries[`K00${index + 4}`] = judged(name);
	const kinds: JsonObject = {
		paragraph: { rule: 'paragraph', fields: { body: { from: 'remainder' } } },
	};
	const config: JsonObject = {
		marker_prefix: 'K',
		check: { files: [sourceFile] },
		decision_model: { model: 'jev-latest', endpoint },
		limits: { max_model_requests: 20, max_concurrent_requests: 1 },
		rulebook: [{ markdown: 'rulebook.md', rules: 'rules.json' }],
		evidence: { ast_grep: 'evidence/sgconfig.yml', kinds },
	};
	if (mode === 'filter') {
		config['filters'] = {
			category: {
				per: '{name}',
				reads: { file: 'overview.md', kind: 'description', fields: ['body'] },
				labels: ['eligible', 'blocked'],
				questions: {
					category: {
						type: 'choice',
						instructions: { check: 'body', task: 'Select the synthetic group.' },
						criteria: { eligible: 'Eligible.', blocked: 'Blocked.' },
					},
				},
				decision: {
					start: 'category',
					steps: {
						category: {
							ask: 'category',
							min_probability: 0.7,
							routes: { eligible: { label: 'eligible' }, blocked: { label: 'blocked' } },
							uncertain: 'unsure',
						},
					},
				},
			},
		};
		kinds['description'] = {
			inline_rule: '{language: Markdown, files: ["**/overview.md"], rule: {kind: document}}',
			fields: { body: { from: 'remainder' } },
		};
		filesystem.writeFileSync(
			path.join(root, 'docs/overview.md'),
			'Synthetic filter description.\n',
		);
		config['check'] = { files: [sourceFile] };
		entries['K006'] = { ...mechanical('gated_local'), only_when: { category: ['eligible'] } };
	}
	if (mode === 'selection') {
		kinds['selected'] = {
			rule: 'paragraph',
			fields: { body: { from: 'remainder' } },
			select: {
				question: {
					type: 'noul',
					instructions: { check: 'body', task: 'Keep this synthetic paragraph?' },
				},
				yes_at_or_above: 0.8,
				no_at_or_below: 0.2,
			},
		};
		kinds['child'] = {
			rule: 'paragraph',
			within: 'selected',
			fields: { body: { from: 'remainder' } },
		};
		kinds['document'] = {
			inline_rule: '{language: Markdown, rule: {kind: document}}',
			fields: { body: { from: 'remainder' }, selected: { from: 'inside', kind: 'selected' } },
		};
		entries['K006'] = { ...mechanical('selected_child'), evidence: 'child' };
		entries['K007'] = { ...mechanical('needed_field', 'selected'), evidence: 'document' };
		entries['K008'] = { ...mechanical('unused_field'), evidence: 'document' };
	}
	if (mode === 'invalid') config['check'] = { files: [] };
	writeJson(path.join(root, 'rules.json'), entries);
	filesystem.writeFileSync(
		path.join(root, 'rulebook.md'),
		Object.keys(entries)
			.map((marker) => `Original obligation. <!-- ${marker} -->\n`)
			.join('\n'),
	);
	writeJson(path.join(root, 'sunstack.lint.json'), config);
}

export function writeCases(root: string, destination: string, rule = 'judged_first'): string {
	baselineGit(root);
	const revision = gitCommand(root, ['rev-parse', 'HEAD']);
	writeJson(destination, {
		repository: root,
		cases: [
			{
				id: 'known-a',
				kind: 'rule',
				rule,
				file: sourceFile,
				revision,
				lines: [1, 1],
				text_sha256: createHash('sha256').update(sourceText).digest('hex'),
				expected: 'pass',
				set: 'development',
				label_from: 'blind-agents',
			},
			{
				id: 'known-b',
				kind: 'rule',
				rule,
				file: sourceFile,
				revision,
				lines: [1, 1],
				text_sha256: createHash('sha256').update(sourceText).digest('hex'),
				expected: 'pass',
				set: 'development',
				label_from: 'blind-agents',
			},
		],
		bars: [],
	});
	return revision;
}
