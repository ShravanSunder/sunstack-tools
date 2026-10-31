import path from 'node:path';

import { expect, test } from 'vitest';

import {
	requireFieldDiagnosticLocation,
	requireKindDiagnosticLocation,
} from './selection-refusals.ts';

const configText = [
	'{',
	'  "evidence": {',
	'    "kinds": {',
	'      "container": {"fields":{"body":{"kind":"step"}}},',
	'      "step": {"rule":"step"},',
	'      "load": {"fields":{"step":{"from":"first_line"}}}',
	'    }',
	'  }',
	'}',
].join('\n');
const file = path.resolve('fixture-config.json');

test('kind diagnostic accepts the owning key with earlier matching values and nested field names', () => {
	expect(() =>
		requireKindDiagnosticLocation({
			configText,
			kind: 'step',
			file,
			diagnostic: `${file}:5:7: kind dependency cycle: step -> container -> step`,
		}),
	).not.toThrow();
});

test('kind diagnostic rejects an earlier value, a nested same-name field and a wrong column', () => {
	for (const location of ['4:51', '6:26', '5:8'])
		expect(() =>
			requireKindDiagnosticLocation({
				configText,
				kind: 'step',
				file,
				diagnostic: `${file}:${location}: kind dependency cycle: step -> container -> step`,
			}),
		).toThrow('Kind-plan diagnostic must locate the owning evidence.kinds.step key');
});

test.each(['a.b', 'a.fields.body', 'ordinary_kind', 'odd"\\kind'])(
	'literal kind %s stays one authored key rather than structural segments',
	(kind) => {
		const literalConfig = [
			'{',
			'  "evidence": {',
			'    "kinds": {',
			'      "a": {"fields":{"body":{"from":"first_line"}}},',
			`      ${JSON.stringify(kind)}: {"rule":"section"}`,
			'    }',
			'  }',
			'}',
		].join('\n');
		expect(() =>
			requireKindDiagnosticLocation({
				configText: literalConfig,
				kind,
				file,
				diagnostic: `${file}:5:7: undefined kind missing_literal_kind`,
			}),
		).not.toThrow();
		for (const location of ['1:1', '4:24'])
			expect(() =>
				requireKindDiagnosticLocation({
					configText: literalConfig,
					kind,
					file,
					diagnostic: `${file}:${location}: undefined kind missing_literal_kind`,
				}),
			).toThrow('Kind-plan diagnostic must locate the owning evidence.kinds.');
	},
);

const fieldCollisionConfig = [
	'{',
	'  "evidence": {',
	'    "kinds": {',
	'      "a": {',
	'        "rule": "section",',
	'        "within": [',
	'          "foo.fields.body"',
	'        ],',
	'        "fields": {}',
	'      },',
	'      "foo.fields.body": {"rule":"section","fields":{}},',
	'      "a.within.foo": {',
	'        "rule": "section",',
	'        "fields": {',
	'          "body": {',
	'            "from": "inside",',
	'            "kind": "missing_field_kind"',
	'          }',
	'        }',
	'      }',
	'    }',
	'  }',
	'}',
].join('\n');

test('field-source diagnostic locates the field under the full literal kind identity', () => {
	expect(() =>
		requireFieldDiagnosticLocation({
			configText: fieldCollisionConfig,
			kind: 'a.within.foo',
			field: 'body',
			file,
			diagnostic: `${file}:15:11: undefined evidence kind`,
		}),
	).not.toThrow();
	expect(() =>
		requireKindDiagnosticLocation({
			configText: fieldCollisionConfig,
			kind: 'a.within.foo',
			file,
			diagnostic: `${file}:12:7: undefined kind missing_field_kind`,
		}),
	).not.toThrow();
});

test('field-source diagnostic rejects a shorter kind container, root key and fallback', () => {
	for (const location of ['7:11', '12:7', '1:1'])
		expect(() =>
			requireFieldDiagnosticLocation({
				configText: fieldCollisionConfig,
				kind: 'a.within.foo',
				field: 'body',
				file,
				diagnostic: `${file}:${location}: undefined evidence kind`,
			}),
		).toThrow('Field-source diagnostic must locate the field inside its complete literal kind');
});
