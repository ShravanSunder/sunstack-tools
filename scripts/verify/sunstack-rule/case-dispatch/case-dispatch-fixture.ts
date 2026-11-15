/** Two real candidate pieces in one file, for a selected/excluded observer control. */
import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import path from 'node:path';

import { baselineGit, gitCommand, writeJson } from '../rulebook/rulebook-fixture-support.ts';

export const selectedText = 'Approved selected café.\n';
export const excludedText = 'Approved excluded sentinel.\n';
export const caseFilePath = 'docs/pieces.md';

export function createCaseDispatchFixture(
	root: string,
	endpoint: string,
): {
	readonly revision: string;
	readonly casesPath: string;
} {
	filesystem.mkdirSync(path.join(root, 'docs'), { recursive: true });
	filesystem.mkdirSync(path.join(root, 'evidence', 'rules'), { recursive: true });
	filesystem.writeFileSync(path.join(root, caseFilePath), `${selectedText}\n${excludedText}`);
	filesystem.writeFileSync(path.join(root, 'evidence', 'sgconfig.yml'), 'ruleDirs: [rules]\n');
	filesystem.writeFileSync(
		path.join(root, 'evidence', 'rules', 'paragraph.yml'),
		'id: paragraph\nlanguage: Markdown\nrule: {kind: paragraph}\n',
	);
	filesystem.writeFileSync(
		path.join(root, 'rulebook.md'),
		'The paragraph is approved. <!-- K001 -->\n',
	);
	writeJson(path.join(root, 'rules.json'), {
		K001: {
			name: 'approval',
			evidence: 'paragraph',
			severity: 'warn',
			questions: {
				approval: {
					type: 'noul',
					instructions: { check: 'body', task: 'Is this paragraph approved?' },
				},
			},
			decision: {
				start: 'approval',
				steps: {
					approval: {
						ask: 'approval',
						yes_at_or_above: 0.8,
						no_at_or_below: 0.2,
						yes: 'pass',
						['no']: { finding: 'Not approved.' },
						uncertain: 'uncertain',
					},
				},
			},
		},
	});
	writeJson(path.join(root, 'sunstack.lint.json'), {
		marker_prefix: 'K',
		check: { files: ['docs/*.md'] },
		decision_model: { model: 'jev-latest', endpoint },
		rulebook: [{ markdown: 'rulebook.md', rules: 'rules.json' }],
		evidence: {
			ast_grep: 'evidence/sgconfig.yml',
			kinds: { paragraph: { rule: 'paragraph', fields: { body: { from: 'remainder' } } } },
		},
	});
	baselineGit(root);
	const revision = gitCommand(root, ['rev-parse', 'HEAD']);
	const makeCase = (id: string, line: number, text: string): object => ({
		id,
		kind: 'rule',
		rule: 'approval',
		file: caseFilePath,
		revision,
		lines: [line, line],
		text_sha256: createHash('sha256').update(text).digest('hex'),
		expected: 'pass',
		set: 'development',
		label_from: 'blind-agents',
	});
	const casesPath = path.join(path.dirname(root), 'case-dispatch-input.json');
	writeJson(casesPath, {
		repository: root,
		cases: [
			makeCase('selected-a', 1, selectedText),
			makeCase('selected-b', 1, selectedText),
			makeCase('excluded', 3, excludedText),
		],
		bars: [],
	});
	return { revision, casesPath };
}
