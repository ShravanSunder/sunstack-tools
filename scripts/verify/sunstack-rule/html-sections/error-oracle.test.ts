import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';

import { test } from 'vitest';

import { arrayValue, objectValue, type JsonObject } from '../rulebook/rulebook-fixture-support.ts';
import type { ErrorControl } from './error-fixtures.ts';
import {
	requireAmpersandAskingCoverage,
	requireRecoveredControl,
	requireVersionOnlyBytes,
} from './error-oracle.ts';
import { requireRecoveringParity, requireSourceWords } from './error-parity.ts';

function encoded(value: JsonObject): Buffer {
	return Buffer.from(JSON.stringify(value, null, 2));
}
function parityReport(version: string, text: string): JsonObject {
	const request = { questions: {}, state: { own: text } };
	return {
		version,
		requests: [
			{
				id: 1,
				request,
				input_sha256: createHash('sha256').update(JSON.stringify(request)).digest('hex'),
			},
		],
		piece_evidence: {
			'1': {
				piece: {
					where: { file: 'wrapped.html' },
					text: 'Original raw HTML',
					fields: { own: { value: 'Original raw HTML' } },
					model_view: { state: 'bounded', fields: { own: text }, cut: [] },
				},
			},
		},
	};
}
function pair(): { before: JsonObject; after: JsonObject } {
	const old = 'Your words: "workspace - area';
	const restored =
		'Your words: "workspace -> Areas -> Projects"; "board is not a good name". An area';

	return { before: parityReport('0.14.0', old), after: parityReport('0.14.1', restored) };
}
test('permits exactly the root version while rejecting nested changes', () => {
	const before = encoded({ version: '0.14.0', state: { version: 'unchanged' } });
	requireVersionOnlyBytes(before, encoded({ version: '0.14.1', state: { version: 'unchanged' } }));
	assert.throws(() =>
		requireVersionOnlyBytes(before, encoded({ version: '0.14.1', state: { version: 'changed' } })),
	);
});
test('permits source-word restoration with its actual computed request hash', () => {
	const { before, after } = pair();
	assert.equal(requireRecoveringParity(encoded(before), encoded(after), false).length, 1);
});
test('rejects a restored field whose request still drops words', () => {
	const { before, after } = pair();
	objectValue(objectValue(objectValue(arrayValue(after['requests'])[0])['request'])['state'])[
		'own'
	] = 'Your words: "workspace - area';
	assert.throws(() => requireRecoveringParity(encoded(before), encoded(after), false));
});
test('rejects raw fields changed while the asking text is correct', () => {
	const { before, after } = pair();
	objectValue(
		objectValue(
			objectValue(objectValue(objectValue(after['piece_evidence'])['1'])['piece'])['fields'],
		)['own'],
	)['value'] = 'Changed raw';
	assert.throws(() => requireRecoveringParity(encoded(before), encoded(after), false));
});
test('rejects arbitrary hash changes hidden beside a valid recovery', () => {
	const { before, after } = pair();
	objectValue(arrayValue(after['requests'])[0])['input_sha256'] = 'a'.repeat(64);
	assert.throws(() => requireRecoveringParity(encoded(before), encoded(after), false));
});
test('source scan permits label insertions and rejects lost or reordered words', () => {
	requireSourceWords('A B 文', 'A [alt: Diagram] B 文');
	assert.throws(() => requireSourceWords('A B 文', 'A 文'));
	assert.throws(() => requireSourceWords('A B 文', 'B A 文'));
});

function ampersandPair(): { report: JsonObject; listing: JsonObject } {
	const pieces: JsonObject = {};
	const requests: JsonObject[] = [];
	const errors: JsonObject[] = [];
	for (let index = 0; index < 5; index++) {
		const file = `page-${index % 3}.html`;
		const source = { path: file, content_identity: `original-${index % 3}` };
		const where = { file, span: { source, byte_start: index * 20 + 2, byte_end: index * 20 + 7 } };
		const error = { where, text: `& expected-${index}` };
		errors.push(error, structuredClone(error));
		pieces[String(index + 1)] = {
			piece: {
				where: { file, span: { source, byte_start: index * 20, byte_end: index * 20 + 10 } },
			},
		};
		requests.push({
			id: index + 1,
			status: 'prepared',
			request: { state: { full: `Caption & expected-${index}` } },
		});
	}
	return { report: { requests, piece_evidence: pieces }, listing: { pieces: errors } };
}
test('joins all ten real-error records to containing prepared owners and actual asking words', () => {
	const { report, listing } = ampersandPair();
	assert.equal(requireAmpersandAskingCoverage(report, listing), 10);
});
test('rejects the old listing-only proof when an error has no prepared owner', () => {
	const { report, listing } = ampersandPair();
	arrayValue(report['requests']).pop();
	assert.throws(() => requireAmpersandAskingCoverage(report, listing), /No prepared asking owner/u);
});
test('rejects a containing owner whose request drops ampersand words', () => {
	const { report, listing } = ampersandPair();
	objectValue(objectValue(objectValue(arrayValue(report['requests'])[0])['request'])['state'])[
		'full'
	] = 'Caption';
	assert.throws(
		() => requireAmpersandAskingCoverage(report, listing),
		/lost the actual ERROR words/u,
	);
});
test('rejects a prepared owner attributed to another content identity', () => {
	const { report, listing } = ampersandPair();
	objectValue(
		objectValue(
			objectValue(objectValue(objectValue(report['piece_evidence'])['1'])['piece'])['where'],
		)['span'],
	)['source'] = { path: 'page-0.html', content_identity: 'other-version' };
	assert.throws(() => requireAmpersandAskingCoverage(report, listing), /No prepared asking owner/u);
});

const fallbackControl: ErrorControl = {
	name: 'fallback',
	raw: 'Before</h2>After',
	projection: { kind: 'text-or-incomplete', text: 'Before After', prefix: 'Before' },
};
function fallbackReport(options: {
	text: string;
	modelCut?: string[];
	requestCut?: string[];
}): JsonObject {
	const cut = options.modelCut ?? [];
	return {
		usage: { model_requests: 0 },
		requests: [
			{
				id: 1,
				status: 'prepared',
				request: { state: { full: options.text, 'cut-fields': options.requestCut ?? cut } },
			},
		],
		piece_evidence: {
			'1': {
				piece: {
					where: { file: 'fallback.html' },
					text: fallbackControl.raw,
					fields: { full: { value: fallbackControl.raw } },
					model_view: { state: 'bounded', fields: { full: options.text }, cut },
				},
			},
		},
	};
}
test('accepts correct text or a truthfully named incomplete under adopted C10', () => {
	requireRecoveredControl(fallbackReport({ text: 'Before After' }), {
		'fallback.html': fallbackControl,
	});
	requireRecoveredControl(
		fallbackReport({
			text: 'Before [cut: html_projection_unrecoverable_markup]',
			modelCut: ['full'],
		}),
		{ 'fallback.html': fallbackControl },
	);
});
test('rejects the silent complete that loses After', () => {
	assert.throws(() =>
		requireRecoveredControl(fallbackReport({ text: 'Before' }), {
			'fallback.html': fallbackControl,
		}),
	);
});
test('requires both request and model-view cuts for the named incomplete', () => {
	const text = 'Before [cut: html_projection_unrecoverable_markup]';
	assert.throws(() =>
		requireRecoveredControl(fallbackReport({ text, modelCut: ['full'], requestCut: [] }), {
			'fallback.html': fallbackControl,
		}),
	);
	assert.throws(() =>
		requireRecoveredControl(fallbackReport({ text, modelCut: [], requestCut: ['full'] }), {
			'fallback.html': fallbackControl,
		}),
	);
});
test('rejects wrong incomplete reasons and lost visible prefixes', () => {
	for (const text of [
		'Before [cut: html_projection_unterminated_markup]',
		'[cut: html_projection_unrecoverable_markup]',
	])
		assert.throws(() =>
			requireRecoveredControl(fallbackReport({ text, modelCut: ['full'] }), {
				'fallback.html': fallbackControl,
			}),
		);
});
test('rejects differing asking payload and model-view text', () => {
	const report = fallbackReport({ text: 'Before After' });
	objectValue(
		objectValue(
			objectValue(objectValue(objectValue(report['piece_evidence'])['1'])['piece'])['model_view'],
		)['fields'],
	)['full'] = 'Before';
	assert.throws(() => requireRecoveredControl(report, { 'fallback.html': fallbackControl }));
});
test('keeps exact markup-only controls strict against false cuts', () => {
	const exact: ErrorControl = {
		...fallbackControl,
		projection: { kind: 'exact', text: 'Before After' },
	};
	requireRecoveredControl(fallbackReport({ text: 'Before After' }), { 'fallback.html': exact });
	assert.throws(() =>
		requireRecoveredControl(
			fallbackReport({
				text: 'Before After [cut: html_projection_unrecoverable_markup]',
				modelCut: ['full'],
			}),
			{ 'fallback.html': exact },
		),
	);
});
