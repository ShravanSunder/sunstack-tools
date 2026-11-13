/** Own-heading and enclosing-heading literals, including exactly-once HTML projection. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';

import { writeProject } from '../html-evidence/html-fixtures.ts';
import { askingRule } from '../html-sections/fixtures.ts';
import {
	arrayValue,
	objectValue,
	readObject,
	writeJson,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';

export const htmlControl =
	'<h1 title="Config &amp; literal" aria-label="Access &amp; readable" alt="Alt &amp; text">A &lt;T&gt; &amp;lt; <em>B</em></h1><h2>C</h2><p>Tail</p>';
export const markdownControl =
	'# A *bold* [link](x) &amp; ###\r\n\r\n## Child `<T>`\r\nBody\r\n\r\nSetext *literal*\r\n---\r\nEnd\r\n';
export function prepareControls(common: string): void {
	for (const format of ['html', 'markdown'] as const) {
		const root = path.join(common, `control-${format}`);
		filesystem.mkdirSync(root, { recursive: true });
		filesystem.writeFileSync(
			path.join(root, format === 'html' ? 'page.html' : 'page.md'),
			format === 'html' ? htmlControl : markdownControl,
		);
		const units =
			format === 'html'
				? { html_units: { mode: 'unit', units: ['section'] } }
				: { markdown_units: { mode: 'unit', units: ['section'] } };
		writeProject(
			root,
			{
				K001: {
					...askingRule,
					questions: {
						quality: {
							type: 'noul',
							instructions: {
								check: 'heading',
								context: ['heading_path'],
								task: 'Read these heading labels.',
							},
						},
					},
				},
			},
			{
				outline: { ...units, fields: { raw: { from: 'first_line' } } },
				section: {
					...units,
					max_chars: 300_000,
					fields: {
						heading: { from: 'section', text: 'heading' },
						heading_path: { from: 'enclosing', kind: 'outline', text: 'heading' },
						full: { from: 'section', text: 'full' },
					},
				},
			},
			[format === 'html' ? 'page.html' : 'page.md'],
			'id: unused\nlanguage: Html\nrule: {kind: document}\n',
		);
		if (format === 'html') {
			const config = readObject(path.join(root, 'sunstack.lint.json'));
			objectValue(config['evidence'])['html'] = { text_attributes: { ['h1']: ['title'] } };
			writeJson(path.join(root, 'sunstack.lint.json'), config);
		}
	}
}
export function requireControl(report: JsonObject, format: 'html' | 'markdown'): void {
	const expected =
		format === 'html'
			? [
					'[title: Config & literal] [aria-label: Access & readable] [alt: Alt & text] A <T> &lt; B',
					'C',
				]
			: ['A *bold* [link](x) &amp;', 'Child `<T>`', 'Setext *literal*'];
	const requests = arrayValue(report['requests']).map(objectValue);
	assert.equal(requests.length, expected.length);
	const pieces = objectValue(report['piece_evidence']);
	for (const [index, text] of expected.entries()) {
		const record = requests[index];
		assert.ok(record);
		const state = objectValue(objectValue(record['request'])['state']);
		assert.equal(state['heading'], text);
		const piece = objectValue(objectValue(pieces[String(index + 1)])['piece']);
		const fields = objectValue(piece['fields']);
		assert.equal(objectValue(fields['heading'])['value'], text);
		const ancestors = index === 0 ? [expected[0]] : [expected[0], text];
		assert.deepEqual(objectValue(fields['heading_path'])['values'], ancestors);
		assert.deepEqual(state['heading_path'], ancestors);
		assert.equal(objectValue(objectValue(piece['model_view'])['fields'])['heading'], text);
		assert.deepEqual(objectValue(piece['model_view'])['cut'], []);
		const raw = format === 'html' ? htmlControl : markdownControl;
		const span = objectValue(arrayValue(objectValue(fields['heading'])['spans'])[0]);
		const headingSource = Buffer.from(raw)
			.subarray(Number(span['byte_start']), Number(span['byte_end']))
			.toString('utf8');
		const expectedSource =
			format === 'html'
				? index === 0
					? '<h1 title="Config &amp; literal" aria-label="Access &amp; readable" alt="Alt &amp; text">A &lt;T&gt; &amp;lt; <em>B</em></h1>'
					: '<h2>C</h2>'
				: index === 0
					? '# A *bold* [link](x) &amp; ###\r\n'
					: index === 1
						? '## Child `<T>`\r\n'
						: 'Setext *literal*\r\n---\r\n';
		assert.equal(headingSource, expectedSource, 'Heading span must cover its original element');
	}
	assert.equal(objectValue(report['usage'])['model_requests'], 0);
}
