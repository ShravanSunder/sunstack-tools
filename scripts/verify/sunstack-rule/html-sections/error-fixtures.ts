/** Authored malformed-text controls; expectations come from the recovery contract. */
import filesystem from 'node:fs';
import path from 'node:path';

import { writeProject } from '../html-evidence/html-fixtures.ts';
import { askingRule } from './fixtures.ts';

export type ProjectionExpectation =
	| { readonly kind: 'exact'; readonly text: string }
	| { readonly kind: 'text-or-incomplete'; readonly text: string; readonly prefix: string }
	| { readonly kind: 'incomplete'; readonly prefix: string };
export interface ErrorControl {
	readonly name: string;
	readonly raw: string;
	readonly projection: ProjectionExpectation;
}

export const errorControls = [
	{
		name: 'stray',
		raw: 'Before</h2>After',
		projection: {
			kind: 'text-or-incomplete',
			text: 'Before After',
			prefix: 'Before',
		},
	},
	{
		name: 'nested',
		raw: 'Before</h2>Middle</section>After',
		projection: {
			kind: 'text-or-incomplete',
			text: 'Before Middle After',
			prefix: 'Before',
		},
	},
	{
		name: 'entity',
		raw: 'Before</h2>Caf&eacute; &amp; 文',
		projection: {
			kind: 'text-or-incomplete',
			text: 'Before Café & 文',
			prefix: 'Before é & 文',
		},
	},
	{
		name: 'unsafe',
		raw: 'Safe prefix <broken title="unterminated',
		projection: {
			kind: 'incomplete',
			prefix: 'Safe prefix',
		},
	},
	{
		name: 'script-recovery',
		raw: 'Before</h2><script>HIDDEN_SCRIPT</wrong>After',
		projection: {
			kind: 'text-or-incomplete',
			text: 'Before',
			prefix: 'Before',
		},
	},
	{
		name: 'comment',
		raw: 'Before</h2><!-- HIDDEN_COMMENT -->After',
		projection: {
			kind: 'text-or-incomplete',
			text: 'Before After',
			prefix: 'Before',
		},
	},
	{
		name: 'well-formed',
		raw: '<p>Before</p><script>KEPT_SCRIPT</script><style>KEPT_STYLE</style><p>After &amp; 文</p>',
		projection: {
			kind: 'exact',
			text: 'Before KEPT_SCRIPT KEPT_STYLE After & 文',
		},
	},
	{
		name: 'cdata-inner-greater',
		raw: '<svg><text><![CDATA[Visible > literal]]></text></svg>',
		projection: {
			kind: 'text-or-incomplete',
			text: 'Visible > literal',
			prefix: '',
		},
	},
	{
		name: 'cdata-no-greater',
		raw: '<svg><text><![CDATA[Visible literal]]></text></svg>',
		projection: {
			kind: 'text-or-incomplete',
			text: 'Visible literal',
			prefix: '',
		},
	},
	{
		name: 'cdata-unsafe',
		raw: 'Before</h2><![CDATA[Visible > literal After',
		projection: {
			kind: 'incomplete',
			prefix: 'Before',
		},
	},
	{
		name: 'cdata-literal-entity',
		raw: 'Before</h2><![CDATA[Visible &amp; > literal]]>After',
		projection: {
			kind: 'text-or-incomplete',
			text: 'Before Visible &amp; > literal After',
			prefix: 'Before',
		},
	},
	{
		name: 'script-slash',
		raw: 'Before</h2><script/>Hidden</script>After',
		projection: {
			kind: 'text-or-incomplete',
			text: 'Before After',
			prefix: 'Before',
		},
	},
	{
		name: 'style-slash',
		raw: 'Before</h2><style/>Hidden</style>After',
		projection: {
			kind: 'text-or-incomplete',
			text: 'Before After',
			prefix: 'Before',
		},
	},
	{
		name: 'excluded-cdata-unsafe',
		raw: 'Before</h2><script/><![CDATA[Hidden</script><p>After</p>',
		projection: {
			kind: 'text-or-incomplete',
			text: 'Before After',
			prefix: 'Before After',
		},
	},
	{
		name: 'script-split-entity',
		raw: 'Before</h2><script/>Hidden &amp; More</script>After',
		projection: {
			kind: 'text-or-incomplete',
			text: 'Before After',
			prefix: 'Before',
		},
	},
	{
		name: 'style-split-entity',
		raw: 'Before</h2><style/>Hidden &amp; More</style>After',
		projection: {
			kind: 'text-or-incomplete',
			text: 'Before After',
			prefix: 'Before',
		},
	},
	{
		name: 'script-split-cdata',
		raw: 'Before</h2><script/><![CDATA[Literal &amp; text]]>Hidden</script>After',
		projection: {
			kind: 'text-or-incomplete',
			text: 'Before After',
			prefix: 'Before',
		},
	},
	{
		name: 'style-split-cdata',
		raw: 'Before</h2><style/><![CDATA[Literal &amp; text]]>Hidden</style>After',
		projection: {
			kind: 'text-or-incomplete',
			text: 'Before After',
			prefix: 'Before',
		},
	},
	{
		name: 'composed-script-cdata-boundary',
		raw: 'Before</h2><script/><![CDATA[Hidden</script><p>After</p>]]>Tail',
		projection: {
			kind: 'text-or-incomplete',
			text: 'Before After ]]>Tail',
			prefix: 'Before',
		},
	},
	{
		name: 'composed-style-comment-boundary',
		raw: 'Before</h2><style/><![CDATA[Hidden</style><p>After</p><!-- ]]> -->Tail',
		projection: {
			kind: 'text-or-incomplete',
			text: 'Before After Tail',
			prefix: 'Before',
		},
	},
	{
		name: 'native-u1-own-fragment',
		raw: '<h3>U1 · Work is organized as workspace → areas → projects</h3>\n<p>Your words: "workspace -> Areas -> Projects"; "board is not a good name". An <strong>area</strong> replaces today\'s "board". A project is a body of work with a lead.</p>\n<p>Why: today\'s names and levels (boards, workstreams, milestones) don\'t match how you think about the work.</p>\n<p>Served by <a href="specification.html#R1">R1</a>.</p></section>\n<section id="U2">',
		projection: {
			kind: 'exact',
			text: 'U1 · Work is organized as workspace → areas → projects Your words: "workspace -> Areas -> Projects"; "board is not a good name". An area replaces today\'s "board". A project is a body of work with a lead. Why: today\'s names and levels (boards, workstreams, milestones) don\'t match how you think about the work. Served by R1 .',
		},
	},
] as const satisfies readonly ErrorControl[];

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
