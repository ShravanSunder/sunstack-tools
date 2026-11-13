import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import operatingSystem from 'node:os';
import path from 'node:path';

import { test } from 'vitest';

import { arrayValue, objectValue, type JsonObject } from '../rulebook/rulebook-fixture-support.ts';
import { requireHeadingReport, type Heading } from './heading-oracle.ts';

const raw = '<h1>A</h1><h2>B</h2>';
const digest = createHash('sha256').update(raw).digest('hex');
const source = {
	kind: 'document',
	repository: 'checked_repository',
	path: 'page.html',
	content_identity: digest,
};
const originalSpan = {
	source,
	byte_start: 0,
	byte_end: 20,
	start_line: 1,
	start_column: 1,
	end_line: 1,
	end_column: 21,
};
const headingSpan = {
	source,
	byte_start: 0,
	byte_end: 10,
	start_line: 1,
	start_column: 1,
	end_line: 1,
	end_column: 11,
};
const oracle: Heading[] = [
	{ file: 'page.html', source_sha256: digest, byte_start: 0, byte_end: 10, text: 'A' },
];
function report(heading: string, derived: boolean): JsonObject {
	const fields = {
		full: { state: 'text', value: raw },
		heading_path: {
			state: 'texts',
			values: [heading],
			spans: [derived ? headingSpan : originalSpan],
		},
	};
	const shared = {
		file: 'page.html',
		kind: 'section',
		fields: {},
		view: { state: 'bounded', fields: {}, cut: [] },
	};
	const state = {
		candidates: { ['c1']: { full: raw, heading_path: [heading] } },
		shared: {},
		where: { file: 'page.html' },
	};
	const request = { state, questions: {} };
	const candidateDescriptor = {
		key: 'c1',
		['id']: { kind: 'section', file: 'page.html', span: originalSpan },
		location: { file: 'page.html', span: originalSpan },
		first_line: raw,
		fields,
		selection_view: { state: 'bounded', fields: state.candidates['c1'], cut: [] },
	};
	const result = {
		usage: { model_requests: 0 },
		requests: [
			{
				status: 'prepared',
				request,
				input_sha256: createHash('sha256').update(JSON.stringify(request)).digest('hex'),
				selection_evidence: { candidates: [candidateDescriptor], shared },
			},
		],
	};
	return objectValue(JSON.parse(JSON.stringify(result)));
}
function descriptor(saved: JsonObject): JsonObject {
	return objectValue(
		arrayValue(
			objectValue(objectValue(arrayValue(saved['requests'])[0])['selection_evidence'])[
				'candidates'
			],
		)[0],
	);
}
test.each([
	'valid',
	'renamed-key',
	'raw',
	'span',
	'order',
	'request',
	'extra-state',
	'other-request',
	'shared-request',
	'where-request',
	'hash',
	'duplicate',
])('source heading oracle checks %s case', (mutation) => {
	const root = filesystem.mkdtempSync(path.join(operatingSystem.tmpdir(), 'sunstack-heading-'));
	try {
		filesystem.writeFileSync(path.join(root, 'page.html'), raw);
		const before = report('A B', false);
		const after = report('A', true);
		const candidate = descriptor(after);
		if (
			[
				'request',
				'renamed-key',
				'extra-state',
				'other-request',
				'shared-request',
				'where-request',
			].includes(mutation)
		) {
			const record = objectValue(arrayValue(after['requests'])[0]);
			const request = objectValue(record['request']);
			const states = objectValue(objectValue(request['state'])['candidates']);
			const state = objectValue(states['c1']);
			if (mutation === 'request') state['heading_path'] = ['wrong serialized heading'];
			else if (mutation === 'extra-state')
				states['c9'] = { heading_path: ['extra serialized candidate'] };
			else if (mutation === 'other-request') state['full'] = 'wrong serialized nonheading input';
			else if (mutation === 'shared-request')
				objectValue(request['state'])['shared'] = { extra: 'wrong shared input' };
			else if (mutation === 'where-request')
				objectValue(request['state'])['where'] = { file: 'wrong.md' };
			else {
				states['c9'] = state;
				delete states['c1'];
				candidate['key'] = 'c9';
			}
			record['input_sha256'] = createHash('sha256').update(JSON.stringify(request)).digest('hex');
		}
		if (mutation === 'raw')
			objectValue(objectValue(candidate['fields'])['full'])['value'] = 'changed';
		if (mutation === 'span')
			objectValue(
				arrayValue(objectValue(objectValue(candidate['fields'])['heading_path'])['spans'])[0],
			)['byte_end'] = 20;
		if (mutation === 'order')
			objectValue(objectValue(candidate['fields'])['heading_path'])['values'] = ['B', 'A'];
		if (mutation === 'hash')
			objectValue(arrayValue(after['requests'])[0])['input_sha256'] = 'arbitrary';
		if (mutation === 'duplicate')
			arrayValue(
				objectValue(objectValue(arrayValue(after['requests'])[0])['selection_evidence'])[
					'candidates'
				],
			).push(candidate);
		const check = (): JsonObject =>
			requireHeadingReport({
				before,
				after,
				headings: oracle,
				root,
				expectedCandidates: 1,
				expectedEntries: 1,
			});
		if (mutation === 'valid' || mutation === 'renamed-key')
			assert.deepEqual(check(), {
				candidates: 1,
				heading_path_entries: 1,
				changed_scalar_entries: 1,
				affected_files: ['page.html'],
			});
		else assert.throws(check);
	} finally {
		filesystem.unlinkSync(path.join(root, 'page.html'));
		filesystem.rmdirSync(root);
	}
});
