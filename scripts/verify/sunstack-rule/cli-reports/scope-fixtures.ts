/** Owned Git repositories keep actual --since acquisition and its binary/limit failures real. */
import filesystem from 'node:fs';
import path from 'node:path';

import {
	baselineGit,
	gitCommand,
	writeJson,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';

export const configFile = 'sunstack.lint.json';
export const checkedFile = 'docs/checked.md';
export const assetFile = 'assets/offending.bin';
export const pngBytes = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0xff]);

export function prepareRepository(options: {
	root: string;
	endpoint: string;
	binaryAsset?: boolean;
	limits?: JsonObject;
}): string {
	for (const directory of ['docs', 'assets', 'evidence/rules'])
		filesystem.mkdirSync(path.join(options.root, directory), { recursive: true });
	filesystem.writeFileSync(path.join(options.root, checkedFile), 'BODY initial.\n');
	filesystem.writeFileSync(
		path.join(options.root, 'docs/peer.md'),
		'An unchecked peer paragraph.\n',
	);
	filesystem.writeFileSync(
		path.join(options.root, assetFile),
		options.binaryAsset ? pngBytes : 'Plain initial asset.\n',
	);
	filesystem.writeFileSync(path.join(options.root, 'evidence/sgconfig.yml'), 'ruleDirs: [rules]\n');
	filesystem.writeFileSync(
		path.join(options.root, 'evidence/rules/paragraph.yml'),
		'id: paragraph\nlanguage: Markdown\nrule: {kind: paragraph}\n',
	);
	filesystem.writeFileSync(
		path.join(options.root, 'rulebook.md'),
		'The checked paragraph has its body. <!-- K001 -->\n',
	);
	writeJson(path.join(options.root, 'rules.json'), {
		K001: {
			name: 'body-present',
			evidence: 'paragraph',
			severity: 'error',
			decision: {
				start: 'body',
				steps: {
					body: {
						mechanical_check: { tool: 'contains', field: 'body', text: 'BODY' },
						yes: 'pass',
						['no']: { finding: 'Missing body.' },
					},
				},
			},
		},
	});
	writeJson(path.join(options.root, configFile), {
		marker_prefix: 'K',
		rulebook: [{ markdown: 'rulebook.md', rules: 'rules.json' }],
		check: { files: [checkedFile] },
		evidence: {
			ast_grep: 'evidence/sgconfig.yml',
			kinds: { paragraph: { rule: 'paragraph', fields: { body: { from: 'remainder' } } } },
		},
		decision_model: { model: 'jev-latest', endpoint: options.endpoint },
		...(options.limits ? { limits: options.limits } : {}),
	});
	baselineGit(options.root);
	return gitCommand(options.root, ['rev-parse', 'HEAD']);
}
