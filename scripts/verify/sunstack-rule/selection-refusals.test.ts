import path from 'node:path';

import { expect, test } from 'vitest';

import { requireKindDiagnosticLocation } from './selection-refusals.ts';

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
