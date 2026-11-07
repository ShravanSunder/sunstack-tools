/** Literal regression fixtures for selected-field dependence and failed-admission allowance. */
import filesystem from 'node:fs';
import path from 'node:path';

import {
	baselineGit,
	objectValue,
	readObject,
	writeJson,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';
import { prepareConfig, prepareRules, prepareSelectionFixture } from './prepare-fixtures.ts';

export function judgedFixtureRule(name: string, evidence: string, field: string): JsonObject {
	return {
		name,
		evidence,
		severity: 'warn',
		questions: {
			verdict: {
				type: 'noul',
				instructions: { check: field, task: 'Does this controlled evidence meet the standard?' },
			},
		},
		decision: {
			start: 'verdict',
			steps: {
				verdict: {
					ask: 'verdict',
					yes_at_or_above: 0.8,
					no_at_or_below: 0.2,
					yes: 'pass',
					['no']: { finding: 'Controlled finding.' },
					uncertain: 'uncertain',
				},
			},
		},
	};
}
export function composedSelectionFixture(root: string, sourceRoot: string): void {
	prepareSelectionFixture(root, sourceRoot);
	const configFile = path.join(root, prepareConfig);
	const config = readObject(configFile);
	const kinds = objectValue(objectValue(config['evidence'])['kinds']);
	kinds['z_composed'] = {
		max_chars: 6000,
		inline_rule: '{language: Markdown, files: ["**/SKILL.md"], rule: {kind: document}}',
		fields: {
			selected_sections: { from: 'same_file', kind: 'workflow_section', text: 'full' },
			document_name: { from: 'yaml', key: 'name' },
		},
	};
	kinds['z_independent'] = {
		inline_rule: '{language: Markdown, files: ["**/SKILL.md"], rule: {kind: atx_heading}}',
		fields: { body: { from: 'first_line' } },
	};
	writeJson(configFile, config);
	const rules = readObject(path.join(root, prepareRules));
	rules['K002'] = judgedFixtureRule('composed_sections', 'z_composed', 'selected_sections');
	rules['K003'] = judgedFixtureRule('independent_heading', 'z_independent', 'body');
	rules['K004'] = judgedFixtureRule('plain_document_name', 'z_composed', 'document_name');
	rules['K005'] = {
		name: 'known_document_name',
		evidence: 'z_composed',
		decision: {
			start: 'known',
			steps: {
				known: {
					mechanical_check: {
						tool: 'contains',
						field: 'document_name',
						text: 'practices-research',
					},
					yes: 'pass',
					['no']: { finding: 'Known document name was lost.' },
				},
			},
		},
	};
	writeJson(path.join(root, prepareRules), rules);
	filesystem.writeFileSync(
		path.join(root, 'rulebook.md'),
		'Read step evidence. <!-- K001 -->\n\nRead selected section evidence. <!-- K002 -->\n\nRead independent heading evidence. <!-- K003 -->\n\nRead document-name evidence. <!-- K004 -->\n\nKeep known raw name evidence. <!-- K005 -->\n',
	);
}
export function failedAdmissionFixture(root: string): void {
	for (const directory of ['evidence/rules', 'docs'])
		filesystem.mkdirSync(path.join(root, directory), { recursive: true });
	filesystem.writeFileSync(path.join(root, 'evidence/sgconfig.yml'), 'ruleDirs: [rules]\n');
	filesystem.writeFileSync(
		path.join(root, 'evidence/rules/document.yml'),
		'id: document\nlanguage: Markdown\nrule: {kind: document}\n',
	);
	filesystem.writeFileSync(path.join(root, 'docs/one.md'), 'VALID_SECOND_REQUEST_SENTINEL.\n');
	const first = judgedFixtureRule('a_oversized', 'subject', 'body');
	first['reference'] = { definition: 'TOKEN_BUDGET_SENTINEL '.repeat(1000) };
	objectValue(objectValue(objectValue(first['questions'])['verdict'])['instructions'])['context'] =
		['reference.definition'];
	writeJson(path.join(root, prepareRules), {
		K001: first,
		K002: judgedFixtureRule('z_valid', 'subject', 'body'),
	});
	filesystem.writeFileSync(
		path.join(root, 'rulebook.md'),
		'Oversized first question. <!-- K001 -->\n\nValid second question. <!-- K002 -->\n',
	);
	writeJson(path.join(root, prepareConfig), {
		rulebook: [{ markdown: 'rulebook.md', rules: prepareRules }],
		marker_prefix: 'K',
		check: { files: ['docs/one.md'] },
		evidence: {
			ast_grep: 'evidence/sgconfig.yml',
			kinds: { subject: { rule: 'document', fields: { body: { from: 'first_line' } } } },
		},
		decision_model: { model: 'typesafe/controlled-rulebook' },
		limits: {
			evidence_max_chars: 1000,
			request_max_chars: 50000,
			max_input_tokens_per_request: 500,
			max_model_requests: 1,
			max_concurrent_requests: 1,
		},
	});
	baselineGit(root);
}
