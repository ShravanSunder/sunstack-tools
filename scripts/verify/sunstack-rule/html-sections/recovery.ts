/** C7: recovered heading openings remain real sections and never become empty prepared success. */
import assert from 'node:assert/strict';
import path from 'node:path';

import { preparedPiece } from '../html-evidence/html-checks.ts';
import { listingPieces } from '../rulebook/rulebook-evidence-checks.ts';
import {
	arrayValue,
	objectValue,
	numberValue,
	readObject,
	textValue,
	writeJson,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';
import type { RulebookProcess } from '../rulebook/rulebook-process.ts';
import { requireSections } from './checks.ts';
import type { Section } from './corpus.ts';
import { unrecoverableMarkupMarker } from './error-oracle.ts';
import { prepareProject } from './fixtures.ts';

export interface RecoveryCase {
	readonly name: string;
	readonly raw: string;
	readonly markers: readonly [string, string];
	readonly titles: readonly [string, string];
	readonly nested: boolean;
	readonly allowIncomplete?: boolean;
}
export const recoveryCases: readonly RecoveryCase[] = [
	{
		name: 'both-unclosed',
		allowIncomplete: true,
		raw: '<h2>Parent<h3>Child',
		markers: ['<h2>Parent', '<h3>Child'],
		titles: ['Parent', 'Child'],
		nested: true,
	},
	{
		name: 'newline',
		allowIncomplete: true,
		raw: '<h2>Parent\n<h3>Child\n',
		markers: ['<h2>Parent', '<h3>Child'],
		titles: ['Parent', 'Child'],
		nested: true,
	},
	{
		name: 'parent-closed',
		allowIncomplete: true,
		raw: '<h2>Parent</h2><h3>Child',
		markers: ['<h2>Parent', '<h3>Child'],
		titles: ['Parent', 'Child'],
		nested: true,
	},
	{
		name: 'child-closed',
		allowIncomplete: true,
		raw: '<h2>Parent<h3>Child</h3>',
		markers: ['<h2>Parent', '<h3>Child'],
		titles: ['Parent', 'Child'],
		nested: true,
	},
	{
		name: 'siblings',
		allowIncomplete: true,
		raw: '<h2>A<h2>B',
		markers: ['<h2>A', '<h2>B'],
		titles: ['A', 'B'],
		nested: false,
	},
	{
		name: 'proper',
		raw: '<h2>Parent</h2><h3>Child</h3>',
		markers: ['<h2>Parent', '<h3>Child'],
		titles: ['Parent', 'Child'],
		nested: true,
	},
	{
		name: 'unicode',
		allowIncomplete: true,
		raw: '猫<h2>Parent🙂<h3>Child',
		markers: ['<h2>Parent', '<h3>Child'],
		titles: ['Parent🙂', 'Child'],
		nested: true,
	},
	{
		name: 'excluded-recovery',
		allowIncomplete: true,
		raw: '<template><h2>Hidden<h3>Hidden</template><ex-markdown><h2>Island<h3>Island</ex-markdown><!-- <h2>Comment --><script>const example="<h2>Script";</script><h2>Visible<h3>Visible child',
		markers: ['<h2>Visible', '<h3>Visible child'],
		titles: ['Visible', 'Visible child'],
		nested: true,
	},
];
/** Literal two-heading relationships are independent of product parsing or recovered AST nodes. */
export function recoverySections(control: RecoveryCase): Section[] {
	const bytes = Buffer.from(control.raw);
	const first = bytes.indexOf(control.markers[0]);
	const second = bytes.indexOf(control.markers[1]);
	assert.ok(first >= 0 && second > first);
	return [
		{
			level: 2,
			start: first,
			end: control.nested ? bytes.length : second,
			own_end: second,
			parent: null,
			heading: control.titles[0],
		},
		{
			level: control.nested ? 3 : 2,
			start: second,
			end: bytes.length,
			own_end: bytes.length,
			parent: control.nested ? 0 : null,
			heading: control.titles[1],
		},
	];
}
export function requireRecoveryPrepared(
	report: JsonObject,
	controls: readonly RecoveryCase[],
): void {
	assert.equal(objectValue(report['status'])['kind'], 'prepared');
	assert.equal(objectValue(report['usage'])['model_requests'], 0);
	assert.equal(
		arrayValue(report['incomplete']).length,
		0,
		'Placeable headings must retain prepared owners',
	);
	const requests = arrayValue(report['requests']).map(objectValue);
	assert.equal(
		requests.length,
		controls.length * 2,
		'Recovered headings silently lost prepared work',
	);
	for (const control of controls) {
		const owned = requests.filter(
			(request) =>
				objectValue(objectValue(request['owner'])['piece'])['file'] === `${control.name}.html`,
		);
		assert.equal(owned.length, 2);
		const pieces = owned.map((request) => {
			assert.equal(request['status'], 'prepared');
			const state = objectValue(objectValue(request['request'])['state']);
			assert.deepEqual(state['unavailable-fields'], {});
			const piece = preparedPiece(report, request);
			const start = numberValue(objectValue(objectValue(piece['where'])['span'])['byte_start']);
			const index = recoverySections(control).findIndex((section) => section.start === start);
			assert.ok(index >= 0);
			const title = control.titles[index];
			assert.ok(title);
			const view = objectValue(piece['model_view']);
			assert.equal(view['state'], 'bounded');
			const cut = arrayValue(view['cut']).map(textValue);
			assert.deepEqual(state['cut-fields'] ?? [], cut, 'Heading request lost its model-view cuts');
			assert.ok(cut.every((field) => ['full', 'own', 'parent'].includes(field)));
			const expected = {
				full: index === 0 && control.nested ? `${control.titles[0]} ${control.titles[1]}` : title,
				own: title,
				parent: index === 1 && control.nested ? control.titles[0] : null,
			};
			for (const [field, text] of Object.entries(expected)) {
				assert.deepEqual(objectValue(view['fields'])[field], state[field]);
				if (cut.includes(field)) {
					assert.ok(
						control.allowIncomplete && text !== null,
						'False cut on a complete heading or absent parent',
					);
					assert.ok(
						textValue(state[field]).endsWith(unrecoverableMarkupMarker),
						'Heading cut lacks the adopted reason',
					);
				} else {
					assert.equal(state[field], text, 'Recovered heading words lost their boundary');
				}
			}

			return piece;
		});
		requireSections({ pieces, sections: recoverySections(control), raw: Buffer.from(control.raw) });
	}
}
export async function observeRecovery(options: {
	observer: RulebookProcess;
	evidence: string;
}): Promise<void> {
	const root = path.join(options.evidence, 'recovered-headings');
	prepareProject({
		root,
		files: Object.fromEntries(
			recoveryCases.map((control) => [`${control.name}.html`, control.raw]),
		),
	});
	const configFile = path.join(root, 'sunstack.lint.json');
	const config = readObject(configFile);
	objectValue(config['decision_model'])['endpoint'] = options.observer.endpoint;
	writeJson(configFile, config);
	await options.observer.run({
		root,
		name: 'recovery-check',
		arguments: ['lint', '--check'],
		expectedExit: 0,
	});
	const listing = await options.observer.run({
		root,
		name: 'recovery-evidence',
		arguments: ['lint', '--all', '--evidence', 'section'],
		expectedExit: 0,
	});
	const pieces = listingPieces(listing.stdout);
	assert.equal(pieces.length, recoveryCases.length * 2);
	for (const control of recoveryCases)
		requireSections({
			pieces: pieces.filter(
				(piece) => objectValue(piece['where'])['file'] === `${control.name}.html`,
			),
			sections: recoverySections(control),
			raw: Buffer.from(control.raw),
		});
	const saved = path.join(options.evidence, 'recovery-prepared.json');
	await options.observer.run({
		root,
		name: 'recovery-prepare',
		arguments: ['lint', '--all', '--prepare', saved],
		expectedExit: 6,
	});
	requireRecoveryPrepared(readObject(saved), recoveryCases);
}
