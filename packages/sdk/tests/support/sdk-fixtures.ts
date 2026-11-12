// Synthetic checked repositories and the equivalent inline and file configurations.
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import type { Configuration } from '../../src/index.ts';

/** A temporary directory owned by one test and removed by it. */
export interface OwnedDirectory {
	readonly path: string;
	remove(): void;
}

export function ownedDirectory(prefix: string): OwnedDirectory {
	const path = mkdtempSync(join(tmpdir(), `sunstack-sdk-${prefix}-`));
	return { path, remove: () => rmSync(path, { recursive: true, force: true }) };
}

export function writeFiles(root: string, files: Readonly<Record<string, string>>): void {
	for (const [relative, content] of Object.entries(files)) {
		const path = join(root, relative);
		mkdirSync(join(path, '..'), { recursive: true });
		writeFileSync(path, content);
	}
}

/** Two skills: one reusable guidance folder and one repository-local procedure. */
export function writeSkillRepository(root: string): void {
	writeFiles(root, {
		'skills/alpha/SKILL.md': '# Alpha\n\nReview each pull request before merging. alpha-portable\n',
		'skills/beta/SKILL.md': '# Beta\n\nRun the deploy script on our staging host. beta-local\n',
	});
}

export const rulebookMarkdown =
	'# Standards\n\nEach instruction names its responsible actor. <!-- K001 -->\n';

export const ruleEntries = {
	K001: {
		name: 'instruction_actor',
		evidence: 'paragraph',
		severity: 'warn',
		only_when: { purpose: ['eligible'] },
		questions: {
			actor: {
				type: 'noul',
				instructions: { check: 'body', against: 'standard', task: 'Is the actor named?' },
			},
		},
		decision: {
			start: 'actor',
			steps: {
				actor: {
					ask: 'actor',
					yes_at_or_above: 0.8,
					no_at_or_below: 0.2,
					yes: 'pass',
					no: { finding: 'Name the actor.', at: 'body' },
					uncertain: 'uncertain',
				},
			},
		},
	},
} as const;

export const purposeFilter = {
	per: 'skills/{name}',
	reads: { file: 'SKILL.md', kind: 'paragraph', fields: ['body'] },
	labels: ['eligible', 'blocked'],
	questions: {
		purpose: {
			type: 'choice',
			instructions: { check: 'body', task: 'Classify whether this is reusable guidance.' },
			criteria: { portable: 'Reusable guidance.', local: 'One repository procedure.' },
		},
	},
	decision: {
		start: 'purpose',
		steps: {
			purpose: {
				ask: 'purpose',
				min_probability: 0.7,
				routes: { portable: { label: 'eligible' }, local: { label: 'blocked' } },
				uncertain: 'unsure',
			},
		},
	},
} as const;

/** The Specification's complete inline example, plus a referenced utility. */
export function inlineConfiguration(baseDirectory: string, endpoint: string): Configuration {
	return {
		id: 'instruction-standards',
		base_directory: baseDirectory,
		marker_prefix: 'K',
		check: { files: ['skills/**/SKILL.md'] },
		decision_model: { model: 'jev-latest', endpoint },
		limits: { max_file_bytes: 1_048_576n },
		evidence: {
			kind: 'inline',
			utilities: {
				'paragraph-node': { language: 'Markdown', rule: { kind: 'paragraph' } },
			},
			matchers: {
				paragraph: {
					language: 'Markdown',
					rule: { matches: 'paragraph-node' },
					metadata: { owner: 'docs', weight: 2 },
				},
			},
			kinds: {
				paragraph: { rule: 'paragraph', fields: { body: { from: 'remainder' } } },
			},
		},
		filters: { purpose: purposeFilter },
		rulebooks: [
			{
				kind: 'inline',
				id: 'ownership-standards',
				markdown: rulebookMarkdown,
				entries: ruleEntries,
			},
		],
	} satisfies Configuration;
}

/** The same definitions as CLI files: config, rulebook pair and an ast-grep project. */
export function writeFileProject(directory: string, endpoint: string): void {
	writeFiles(directory, {
		'sunstack.lint.json': `${JSON.stringify(
			{
				rulebook: [{ markdown: 'standards/rulebook.md', rules: 'standards/rules.json' }],
				marker_prefix: 'K',
				check: { files: ['skills/**/SKILL.md'] },
				evidence: {
					ast_grep: 'evidence/sgconfig.yml',
					kinds: { paragraph: { rule: 'paragraph', fields: { body: { from: 'remainder' } } } },
				},
				decision_model: { model: 'jev-latest', endpoint },
				limits: { max_file_bytes: 1_048_576 },
				filters: { purpose: purposeFilter },
			},
			null,
			2,
		)}\n`,
		'standards/rulebook.md': rulebookMarkdown,
		'standards/rules.json': `${JSON.stringify(ruleEntries, null, 2)}\n`,
		'evidence/sgconfig.yml': 'ruleDirs: [rules]\nutilDirs: [utils]\n',
		'evidence/utils/paragraph-node.yml':
			'id: paragraph-node\nlanguage: md\nrule: {kind: paragraph}\n',
		'evidence/rules/paragraph.yml':
			'id: paragraph\nlanguage: Markdown\nrule: {matches: paragraph-node}\nmetadata: {owner: docs, weight: 2}\n',
	});
}

/**
 * The admitted value of each normalized inline source in `inlineConfiguration`, as the
 * native contract at revision 3 records it: Sunstack defaults filled (`check.ignore`,
 * `max_question_words`, unset limits as null), authoring-only members omitted (`id`,
 * `base_directory`, `evidence.kind`, `rulebooks`), and ast-grep's upstream defaults written
 * out for matchers and utilities.
 */
export function expectedInlineSources(endpoint: string): {
	root: unknown;
	entries: unknown;
	utility: unknown;
	matcher: unknown;
} {
	return {
		root: {
			check: { files: ['skills/**/SKILL.md'], ignore: [] },
			decision_model: { endpoint, model: 'jev-latest' },
			filters: { purpose: purposeFilter },
			kinds: { paragraph: { fields: { body: { from: 'remainder' } }, rule: 'paragraph' } },
			limits: {
				attempt_timeout_seconds: null,
				evidence_max_chars: null,
				max_concurrent_requests: null,
				max_file_bytes: 1_048_576,
				max_files: null,
				max_input_tokens_per_request: null,
				max_locate_lines: null,
				max_matches: null,
				max_model_requests: null,
				max_total_bytes: null,
				request_max_chars: null,
			},
			marker_prefix: 'K',
			max_question_words: 400,
		},
		entries: ruleEntries,
		utility: {
			constraints: null,
			language: 'Markdown',
			rule: { kind: 'paragraph' },
			transform: null,
			utils: null,
		},
		matcher: {
			constraints: null,
			files: null,
			fix: null,
			ignores: null,
			labels: null,
			language: 'Markdown',
			message: '',
			metadata: { owner: 'docs', weight: 2 },
			note: null,
			rewriters: null,
			rule: { matches: 'paragraph-node' },
			severity: 'hint',
			transform: null,
			url: null,
			utils: null,
		},
	};
}
