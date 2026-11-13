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
	{
		name: 'cdata-inner-greater',
		raw: '<svg><text><![CDATA[Visible > literal]]></text></svg>',
		expected: 'Visible > literal',
		cut: false,
	},
	{
		name: 'cdata-no-greater',
		raw: '<svg><text><![CDATA[Visible literal]]></text></svg>',
		expected: 'Visible literal',
		cut: false,
	},
	{
		name: 'cdata-unsafe',
		raw: 'Before</h2><![CDATA[Visible > literal After',
		expected: 'Before [cut: html_projection_unterminated_markup]',
		cut: true,
	},
	{
		name: 'cdata-literal-entity',
		raw: 'Before</h2><![CDATA[Visible &amp; > literal]]>After',
		expected: 'Before Visible &amp; > literal After',
		cut: false,
	},
	{
		name: 'script-slash',
		raw: 'Before</h2><script/>Hidden</script>After',
		expected: 'Before After',
		cut: false,
	},
	{
		name: 'style-slash',
		raw: 'Before</h2><style/>Hidden</style>After',
		expected: 'Before After',
		cut: false,
	},
	{
		name: 'excluded-cdata-unsafe',
		raw: 'Before</h2><script/><![CDATA[Hidden</script><p>After</p>',
		expected: 'Before After',
		cut: false,
	},
	{
		name: 'script-split-entity',
		raw: 'Before</h2><script/>Hidden &amp; More</script>After',
		expected: 'Before After',
		cut: false,
	},
	{
		name: 'style-split-entity',
		raw: 'Before</h2><style/>Hidden &amp; More</style>After',
		expected: 'Before After',
		cut: false,
	},
	{
		name: 'script-split-cdata',
		raw: 'Before</h2><script/><![CDATA[Literal &amp; text]]>Hidden</script>After',
		expected: 'Before After',
		cut: false,
	},
	{
		name: 'style-split-cdata',
		raw: 'Before</h2><style/><![CDATA[Literal &amp; text]]>Hidden</style>After',
		expected: 'Before After',
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
