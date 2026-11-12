/** Authored malformed-text controls; expectations come from the recovery contract. */
import filesystem from 'node:fs';
import path from 'node:path';

import { writeProject } from '../html-evidence/html-fixtures.ts';
import { askingRule } from './fixtures.ts';

export const errorControls = [
	{ name: 'stray', raw: 'Before</h2>After', expected: 'Before After', cut: false },
	{
		name: 'nested',
		raw: 'Before</h2>Middle</section>After',
		expected: 'Before Middle After',
		cut: false,
	},
	{
		name: 'entity',
		raw: 'Before</h2>Caf&eacute; &amp; 文',
		expected: 'Before Café & 文',
		cut: false,
	},
	{
		name: 'unsafe',
		raw: 'Safe prefix <broken title="unterminated',
		expected: 'Safe prefix [cut: html_projection_unterminated_markup]',
		cut: true,
	},
	{
		name: 'script-recovery',
		raw: 'Before</h2><script>HIDDEN_SCRIPT</wrong>After',
		expected: 'Before',
		cut: false,
	},
	{
		name: 'comment',
		raw: 'Before</h2><!-- HIDDEN_COMMENT -->After',
		expected: 'Before After',
		cut: false,
	},
	{
		name: 'well-formed',
		raw: '<p>Before</p><script>KEPT_SCRIPT</script><style>KEPT_STYLE</style><p>After &amp; 文</p>',
		expected: 'Before KEPT_SCRIPT KEPT_STYLE After & 文',
		cut: false,
	},
] as const;

export function prepareErrorProject(
	root: string,
	files: Readonly<Record<string, string>>,
	errors = false,
): void {
	filesystem.mkdirSync(root);
	for (const [file, raw] of Object.entries(files)) {
		filesystem.mkdirSync(path.dirname(path.join(root, file)), { recursive: true });
		filesystem.writeFileSync(path.join(root, file), raw);
	}
	writeProject(
		root,
		{
			K001: {
				...askingRule,
				questions: {
					quality: {
						type: 'noul',
						instructions: { check: 'full', task: 'Read the complete explanation.' },
					},
				},
			},
		},
		{
			section: {
				rule: errors ? 'error' : 'whole',
				max_chars: 300_000,
				fields: { full: { from: 'remainder' } },
			},
		},
		Object.keys(files),
		`id: whole\nlanguage: Html\nrule: {kind: document}\n---\nid: error\nlanguage: Html\nrule: {kind: ERROR}\n`,
	);
}
