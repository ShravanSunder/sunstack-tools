/** Authored verifier fixtures exercise request building without reconstructing expected requests. */
import filesystem from 'node:fs';
import path from 'node:path';

import {
	baselineGit,
	readObject,
	objectValue,
	writeJson,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';
import {
	askingStepRule,
	copySelectionFixture,
	changeSelectionKinds,
	researchFile,
} from '../selection/selection-fixture.ts';

export const prepareConfig = 'skill-rules.config.json';
export const prepareRules = 'rules.json';
export const prepareMember = 'skills/portable/notes/one.md';
export const longBody = `HEAD_UTF8_☃_${'a'.repeat(1400)}_MIDDLE_SHOULD_BE_CUT_${'z'.repeat(1400)}_TAIL_SENTINEL`;
export function prepareRequestFixture(root: string): void {
	for (const folder of ['evidence/rules', 'skills/portable/notes', 'skills/product/notes'])
		filesystem.mkdirSync(path.join(root, folder), { recursive: true });
	writeJson(path.join(root, 'evidence/sgconfig.yml'), { ruleDirs: ['rules'] });
	filesystem.writeFileSync(
		path.join(root, 'evidence/rules/paragraph.yml'),
		'id: paragraph\nlanguage: Markdown\nrule: {kind: paragraph}\n',
	);
	for (const [folder, description] of [
		['portable', 'PORTABLE_FILTER_SENTINEL'],
		['product', 'PRODUCT_FILTER_SENTINEL'],
	]) {
		filesystem.writeFileSync(
			path.join(root, `skills/${folder}/overview.md`),
			`---\ndescription: ${description}\n---\n`,
		);
		filesystem.writeFileSync(
			path.join(root, `skills/${folder}/notes/one.md`),
			`${longBody}\n\nSECOND_ORDERED_TEXT_SENTINEL.\n`,
		);
	}
	const judged: JsonObject = {
		name: 'prepared_plain',
		evidence: 'subject',
		severity: 'warn',
		reference: {
			examples: { '10': 'ten first lexically', '2': 'two later lexically' },
			explanation: 'Unicode ☃ and "quotes" remain exact.',
		},
		questions: {
			verdict: {
				type: 'noul',
				instructions: {
					check: 'body',
					context: ['peers', 'reference.examples'],
					task: 'Does this subject meet the controlled standard?',
				},
			},
			unused: {
				type: 'noul',
				instructions: {
					check: 'body',
					task: 'This independently offered answer may remain unused.',
				},
			},
		},
		decision: {
			start: 'prefix',
			steps: {
				prefix: {
					mechanical_check: { tool: 'contains', field: 'body', text: 'SENTINEL' },
					yes: 'verdict',
					['no']: 'pass',
				},
				verdict: {
					ask: 'verdict',
					yes_at_or_above: 0.8,
					no_at_or_below: 0.2,
					yes: 'pass',
					['no']: {
						finding: 'Controlled finding; read the selected line.',
						['at']: 'locator_context',
						locate: {
							name: 'focus_line',
							task: 'Which offered line describes the issue?',
							min_probability: 0.7,
						},
					},
					uncertain: 'uncertain',
				},
			},
		},
	};
	writeJson(path.join(root, prepareRules), {
		K001: {
			name: 'prepared_mechanical',
			evidence: 'subject',
			decision: {
				start: 'check',
				steps: {
					check: {
						mechanical_check: { tool: 'contains', field: 'body', text: 'SENTINEL' },
						yes: 'pass',
						['no']: { finding: 'Missing controlled text.' },
					},
				},
			},
		},
		K002: judged,
		K003: { ...judged, name: 'prepared_gated', only_when: { category: ['eligible'] } },
	});
	filesystem.writeFileSync(
		path.join(root, 'rulebook.md'),
		'Mechanical text has its sentinel. <!-- K001 -->\n\nPlain text is assessed. <!-- K002 -->\n\nGated text is assessed. <!-- K003 -->\n',
	);
	writeJson(path.join(root, prepareConfig), {
		rulebook: [{ markdown: 'rulebook.md', rules: prepareRules }],
		marker_prefix: 'K',
		check: { files: ['skills/*/notes/*.md'] },
		evidence: {
			ast_grep: 'evidence/sgconfig.yml',
			kinds: {
				description: {
					inline_rule: '{language: Markdown, files: ["**/overview.md"], rule: {kind: document}}',
					fields: { description: { from: 'yaml', key: 'description' } },
				},
				subject: {
					rule: 'paragraph',
					fields: {
						body: { from: 'first_line' },
						peers: { from: 'same_file', rule: 'paragraph', text: 'full' },
						locator_context: { from: 'first_line' },
					},
				},
			},
		},
		filters: {
			category: {
				per: 'skills/{name}',
				reads: { file: 'overview.md', kind: 'description', fields: ['description'] },
				labels: ['eligible', 'blocked'],
				reference: { definition: 'Portable and product are disjoint controlled labels.' },
				questions: {
					category: {
						type: 'choice',
						instructions: {
							check: 'description',
							context: ['reference.definition'],
							task: 'Which controlled group applies?',
						},
						criteria: { portable: 'Portable text.', product: 'Product text.' },
					},
				},
				decision: {
					start: 'category',
					steps: {
						category: {
							ask: 'category',
							min_probability: 0.7,
							routes: { portable: { label: 'eligible' }, product: { label: 'blocked' } },
							uncertain: 'unsure',
						},
					},
				},
			},
		},
		decision_model: { model: 'synthetic/prepare-proof' },
		limits: {
			evidence_max_chars: 1000,
			request_max_chars: 50000,
			max_input_tokens_per_request: 50000,
			max_model_requests: 50,
			max_concurrent_requests: 1,
		},
	});
	baselineGit(root);
}
export function prepareSelectionFixture(root: string, sourceRoot: string): void {
	copySelectionFixture({ destination: root, sourceRoot, files: [researchFile] });
	askingStepRule(root);
	const rulesPath = path.join(root, prepareRules);
	const rules = readObject(rulesPath);
	objectValue(objectValue(objectValue(objectValue(rules['K001'])['decision'])['steps'])['verdict'])[
		'uncertain'
	] = 'uncertain';
	writeJson(rulesPath, rules);
	changeSelectionKinds(root, (kinds) => {
		// The retained fixture has workflow_section selected; step and load depend on it.
		if (!kinds['workflow_section'] || !kinds['step'] || !kinds['load'])
			throw new Error('Selection fixture lost its dependent-kind witnesses');
	});
	const configPath = path.join(root, prepareConfig);
	const config = readObject(configPath);
	objectValue(config['decision_model'])['model'] = 'typesafe/controlled-rulebook';
	writeJson(configPath, config);
	baselineGit(root);
}
