/** Owned inputs and literal byte oracle for two adjacent real HTML nodes. */
import filesystem from 'node:fs';
import path from 'node:path';

import { baselineGit, gitCommand, writeJson } from '../rulebook/rulebook-fixture-support.ts';

export const byteFixtureSource = '<p>café yes</p><p>no</p>\r\n';
export const byteFixturePath = 'docs/pieces.html';
export const byteFixtureCases = [
	{
		id: 'first',
		bytes: [0, 16],
		text_sha256: '561de257158b71f3d98ee74cc24288403bd77d0aa5bb13aaf06287ac7002cfb4',
		expected: 'pass',
	},
	{
		id: 'second',
		bytes: [16, 25],
		text_sha256: '61e0a88edb8e39ec015b46afb3483d429cfe55c3e2e2140315f5aae69a950ede',
		expected: 'warn',
	},
] as const;

/** Write and commit the synthetic repository; its cases live outside the checked tree. */
export function createByteFixture(
	root: string,
	endpoint: string,
): {
	readonly revision: string;
	readonly unselectedRevision: string;
	readonly casesPath: string;
} {
	filesystem.mkdirSync(path.join(root, 'docs'), { recursive: true });
	filesystem.mkdirSync(path.join(root, 'evidence', 'rules'), { recursive: true });
	filesystem.writeFileSync(path.join(root, '.gitattributes'), '* -text -export-subst\n');
	filesystem.writeFileSync(path.join(root, byteFixturePath), byteFixtureSource);
	filesystem.writeFileSync(path.join(root, 'evidence', 'sgconfig.yml'), 'ruleDirs: [rules]\n');
	filesystem.writeFileSync(
		path.join(root, 'evidence', 'rules', 'paragraph.yml'),
		"id: paragraph\nlanguage: Html\nrule: {kind: element, regex: '^<p(?: |>)'}\n",
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
		check: { files: ['docs/*.html'] },
		decision_model: { model: 'jev-latest', endpoint },
		rulebook: [{ markdown: 'rulebook.md', rules: 'rules.json' }],
		evidence: {
			ast_grep: 'evidence/sgconfig.yml',
			kinds: { paragraph: { rule: 'paragraph', fields: { body: { from: 'remainder' } } } },
		},
	});
	baselineGit(root);
	const revision = gitCommand(root, ['rev-parse', 'HEAD']);
	filesystem.appendFileSync(path.join(root, byteFixturePath), '<p>Later revision.</p>\r\n');
	gitCommand(root, ['add', byteFixturePath]);
	gitCommand(root, ['commit', '-qm', 'Later fixture revision']);
	const unselectedRevision = gitCommand(root, ['rev-parse', 'HEAD']);
	const casesPath = path.join(path.dirname(root), 'byte-cases.json');
	writeJson(casesPath, {
		repository: root,
		cases: [
			...byteFixtureCases.map((item) => ({
				...item,
				kind: 'rule',
				rule: 'approval',
				file: byteFixturePath,
				revision,
				lines: [1, 1],
				set: 'development',
				label_from: 'blind-agents',
			})),
			{
				...byteFixtureCases[0],
				id: 'unselected-revision',
				kind: 'rule',
				rule: 'approval',
				file: byteFixturePath,
				revision: unselectedRevision,
				lines: [1, 1],
				set: 'development',
				label_from: 'blind-agents',
			},
		],
		bars: [],
	});
	return { revision, unselectedRevision, casesPath };
}
