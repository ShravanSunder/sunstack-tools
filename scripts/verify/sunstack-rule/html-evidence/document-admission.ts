/** Keyless CLI admission guards and positive neighbours for deferred document fields. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';

import { z } from 'zod';

import { requireDebugSignature } from '../rulebook/rulebook-amendment.ts';
import {
	objectValue,
	readObject,
	runSequence,
	writeJson,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';
import { declaredTarget, RulebookProcess } from '../rulebook/rulebook-process.ts';
import { htmlConfig, writeProject } from './html-fixtures.ts';

interface AdmissionCase {
	readonly name: string;
	readonly phase: 'selection' | 'as-kind';
	readonly source: 'local' | 'unreferenced-document' | 'document' | 'target';
	readonly diagnostic?: string;
}
const cases: readonly AdmissionCase[] = [
	{ name: 'selection-local', phase: 'selection', source: 'local' },
	{ name: 'selection-unreferenced-document', phase: 'selection', source: 'unreferenced-document' },
	{
		name: 'selection-document',
		phase: 'selection',
		source: 'document',
		diagnostic: 'selection question cannot name a document field',
	},
	{
		name: 'selection-target',
		phase: 'selection',
		source: 'target',
		diagnostic: 'selection question cannot name a target field',
	},
	{ name: 'as-kind-local', phase: 'as-kind', source: 'local' },
	{ name: 'as-kind-unreferenced-document', phase: 'as-kind', source: 'unreferenced-document' },
	{
		name: 'as-kind-document',
		phase: 'as-kind',
		source: 'document',
		diagnostic: 'as_kind must name a document-free kind',
	},
	{
		name: 'as-kind-target',
		phase: 'as-kind',
		source: 'target',
		diagnostic: 'as_kind must name a target-free kind',
	},
];
const definitions =
	"id: paragraph\nlanguage: Html\nrule: {kind: element, regex: '^<p(?: |>)'}\n---\nid: whole\nlanguage: Html\nrule: {kind: document}\n";
const documentField: JsonObject = { from: 'document', path: 'related.html', rule: 'whole' };
const targetField: JsonObject = { from: 'target', regex: 'href="([^"]+)"', resolve: 'path' };

function prepareAdmission(root: string, control: AdmissionCase): void {
	const fields: JsonObject = { body: { from: 'remainder' } };
	const subject: JsonObject = { rule: 'paragraph', fields };
	const kinds: JsonObject = { subject };
	if (control.phase === 'selection') {
		if (control.source !== 'local')
			fields['remote'] = control.source === 'target' ? targetField : documentField;
		subject['select'] = {
			question: {
				type: 'noul',
				instructions: {
					check: ['document', 'target'].includes(control.source) ? 'remote' : 'body',
					task: 'Does this paragraph belong to the selected kind?',
				},
			},
			yes_at_or_above: 0.8,
			no_at_or_below: 0.2,
		};
	} else {
		fields['links'] = { ...targetField, as_kind: 'target_view' };
		const targetFields: JsonObject = { body: { from: 'remainder' } };
		if (control.source === 'document') targetFields['remote'] = documentField;
		if (control.source === 'target') targetFields['remote'] = targetField;
		kinds['target_view'] = { rule: 'whole', fields: targetFields };
		if (control.source === 'unreferenced-document')
			kinds['spare_document'] = {
				rule: 'whole',
				fields: { body: { from: 'remainder' }, remote: documentField },
			};
	}
	writeProject(
		root,
		{
			K001: {
				name: 'admission-local-body',
				evidence: 'subject',
				severity: 'error',
				decision: {
					start: 'check',
					steps: {
						check: {
							mechanical_check: { tool: 'contains', field: 'body', text: 'paragraph' },
							yes: 'pass',
							['no']: { finding: 'Inspect the paragraph.' },
						},
					},
				},
			},
		},
		kinds,
		['pages/source.html'],
		definitions,
	);
	if (control.phase === 'as-kind' && control.source === 'unreferenced-document') {
		const file = path.join(root, 'rules.json');
		const rules = readObject(file);
		rules['K002'] = {
			...objectValue(rules['K001']),
			name: 'plain-document-body',
			evidence: 'spare_document',
		};
		writeJson(file, rules);
		filesystem.appendFileSync(
			path.join(root, 'rulebook.md'),
			'\nInspect a plain document kind. <!-- K002 -->\n',
		);
	}
	filesystem.mkdirSync(path.join(root, 'pages'));
	filesystem.writeFileSync(
		path.join(root, 'pages/source.html'),
		'<p href="related.html">An authored paragraph.</p>',
	);
	filesystem.writeFileSync(path.join(root, 'pages/related.html'), '<p>Related evidence.</p>');
}
const schema = z.object({
	binary: z.string(),
	'source-root': z.string(),
	commit: z.string().regex(/^[a-f0-9]{40}$/),
	sha256: z.string().regex(/^[a-f0-9]{64}$/),
	evidence: z.string(),
});

export async function observeDocumentAdmission(options: z.infer<typeof schema>): Promise<void> {
	const target = declaredTarget({
		binary: options.binary,
		source: options['source-root'],
		commit: options.commit,
		digest: options.sha256,
	});
	requireDebugSignature(target.binary);
	const evidence = path.resolve(options.evidence);
	filesystem.mkdirSync(evidence);
	const observer = new RulebookProcess(target, path.join(evidence, 'run'));
	await observer.start();
	try {
		const account: JsonObject[] = [];
		await runSequence(cases, async (control) => {
			const root = path.join(evidence, control.name);
			prepareAdmission(root, control);
			const file = path.join(root, htmlConfig);
			const config = readObject(file);
			config['decision_model'] = {
				...objectValue(config['decision_model']),
				endpoint: observer.endpoint,
			};
			writeJson(file, config);
			const result = await observer.run({
				root,
				name: control.name,
				arguments: ['lint', '--config', htmlConfig, '--check'],
				expectedExit: control.diagnostic ? 2 : 0,
			});
			assert.equal(result.captures.length, 0, 'Admission check dispatched a model request');
			const output = Buffer.concat([result.stdout, result.stderr]).toString('utf8');
			if (control.diagnostic) {
				assert.ok(output.includes(control.diagnostic), `${control.name}: wrong diagnostic`);
				assert.ok(
					output.includes(path.resolve(file)),
					'Refusal does not identify the config input',
				);
			} else {
				assert.ok(output.includes('K001'), 'Positive admission control lost its authored rule');
				if (control.phase === 'as-kind' && control.source === 'unreferenced-document')
					assert.ok(
						output.includes('K002'),
						'Plain document kind was not admitted as an explicit rule',
					);
			}
			account.push({
				name: control.name,
				expected_exit: control.diagnostic ? 2 : 0,
				diagnostic: control.diagnostic ?? null,
			});
		});
		writeJson(path.join(evidence, 'pass.json'), {
			target,
			cases: account,
			positive_neighbours: 4,
			refusals: 4,
			http_requests: 0,
			limits: [
				'Structural admission only; no model selection or target/document acquisition was requested.',
			],
		});
	} finally {
		await observer.close();
	}
}
if (process.argv[1] === import.meta.filename) {
	const { values } = parseArgs({
		options: {
			binary: { type: 'string' },
			'source-root': { type: 'string' },
			commit: { type: 'string' },
			sha256: { type: 'string' },
			evidence: { type: 'string' },
			help: { type: 'boolean' },
		},
	});
	if (values.help)
		process.stdout.write(
			'Observe Document/Target selection-question and target.as_kind refusals with local-only/unreferenced-document positive neighbours.\n--binary PATH --source-root CHECKOUT --commit FULL_SHA --sha256 DIGEST --evidence NEW_DIRECTORY\nKeyless lint --check only, including selection configs; no --prepare or dispatch.\n',
		);
	else await observeDocumentAdmission(schema.parse(values));
}
