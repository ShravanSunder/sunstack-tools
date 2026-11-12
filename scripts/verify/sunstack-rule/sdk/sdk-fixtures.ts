/** Synthetic checked inputs and equivalent file/inline definitions; no product fixture imports. */
import filesystem from 'node:fs';
import path from 'node:path';

export function sdkInlineConfiguration(
	base: string,
	endpoint: string,
	standard = 'ACTOR_A',
): object {
	return {
		id: 'sdk-verification',
		base_directory: base,
		marker_prefix: 'K',
		check: { files: ['skills/**/SKILL.md'] },
		decision_model: { model: 'jev-latest', endpoint },
		limits: { max_file_bytes: 1048576n },
		evidence: {
			kind: 'inline',
			utilities: { sdk_paragraph: { language: 'Markdown', rule: { kind: 'paragraph' } } },
			matchers: { paragraph: { language: 'Markdown', rule: { matches: 'sdk_paragraph' } } },
			kinds: { paragraph: { rule: 'paragraph', fields: { body: { from: 'remainder' } } } },
		},
		filters: {
			purpose: {
				per: 'skills/{name}',
				reads: { file: 'SKILL.md', kind: 'paragraph', fields: ['body'] },
				labels: ['eligible', 'blocked'],
				questions: {
					purpose: {
						type: 'choice',
						instructions: { check: 'body', task: 'Classify reusable guidance.' },
						criteria: { portable: 'Reusable guidance.', local: 'Local procedure.' },
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
			},
		},
		rulebooks: [
			{
				kind: 'inline',
				id: 'sdk-standards',
				markdown: `${standard}: Each instruction names its actor. <!-- K001 -->\n`,
				entries: sdkRuleEntries(),
			},
		],
	};
}
export function sdkRuleEntries(): object {
	return {
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
						['no']: { finding: 'Name the actor.', ['at']: 'body' },
						uncertain: 'uncertain',
					},
				},
			},
		},
	};
}
export function createCheckedSdkInputs(root: string, sentinel = 'CHECKED_A'): void {
	filesystem.mkdirSync(path.join(root, 'skills', 'portable'), { recursive: true });
	filesystem.writeFileSync(
		path.join(root, 'skills', 'portable', 'SKILL.md'),
		`${sentinel}: The caller names its actor.\n`,
	);
}
export function createSdkFileDefinitions(root: string, endpoint: string): void {
	filesystem.mkdirSync(path.join(root, 'evidence', 'rules'), { recursive: true });
	filesystem.mkdirSync(path.join(root, 'evidence', 'utils'), { recursive: true });
	filesystem.writeFileSync(
		path.join(root, 'evidence', 'sgconfig.yml'),
		'ruleDirs: [rules]\nutilDirs: [utils]\n',
	);
	filesystem.writeFileSync(
		path.join(root, 'evidence', 'utils', 'paragraph.yml'),
		'id: sdk_paragraph\nlanguage: Markdown\nrule: {kind: paragraph}\n',
	);
	filesystem.writeFileSync(
		path.join(root, 'evidence', 'rules', 'paragraph.yml'),
		'id: paragraph\nlanguage: Markdown\nrule: {matches: sdk_paragraph}\n',
	);
	filesystem.writeFileSync(
		path.join(root, 'rulebook.md'),
		'ACTOR_A: Each instruction names its actor. <!-- K001 -->\n',
	);
	filesystem.writeFileSync(path.join(root, 'rules.json'), JSON.stringify(sdkRuleEntries()));
	const inline = sdkInlineConfiguration(root, endpoint);
	// These fields are authored independently in the CLI vocabulary; no hidden SDK file conversion.
	filesystem.writeFileSync(
		path.join(root, 'sunstack.lint.json'),
		JSON.stringify({
			marker_prefix: 'K',
			check: { files: ['skills/**/SKILL.md'] },
			decision_model: { model: 'jev-latest', endpoint },
			limits: { max_file_bytes: 1048576 },
			rulebook: [{ markdown: 'rulebook.md', rules: 'rules.json' }],
			evidence: {
				ast_grep: 'evidence/sgconfig.yml',
				kinds: { paragraph: { rule: 'paragraph', fields: { body: { from: 'remainder' } } } },
			},
			...('filters' in inline ? { filters: inline.filters } : {}),
		}),
	);
}
