/** O5 retains mechanical-only filter input while omitting it from the one model request. */
import filesystem from 'node:fs';
import path from 'node:path';

import {
	filterReply,
	includedFolder,
	includedMembers,
	prepareFilterFixture,
} from './rulebook-filters.ts';
import {
	arrayValue,
	configName,
	configure,
	configureEndpoint,
	equal,
	numberValue,
	objectValue,
	parseObject,
	readObject,
	requireCondition,
	rulebookName,
	rulesName,
	runSequence,
	textValue,
	writeJson,
	type JsonObject,
} from './rulebook-fixture-support.ts';
import { type RulebookProcess } from './rulebook-process.ts';

const mechanicalToken = 'RAW_MECHANICAL_MIDDLE_SENTINEL';
const mechanicalOpening = `${'x'.repeat(1700)} ${mechanicalToken} ${'y'.repeat(1700)}\n`;

export function prepareMechanicalFilterFixture(root: string): void {
	prepareFilterFixture(root);
	filesystem.writeFileSync(
		path.join(root, includedFolder, 'overview.md'),
		`---\ndescription: General instructions for an included folder.\n---\n\n${mechanicalOpening}`,
	);
	filesystem.writeFileSync(
		path.join(root, 'evidence/rules/marker.yml'),
		`id: marker\nlanguage: Markdown\nrule: {kind: paragraph, regex: '${mechanicalToken}'}\n`,
	);
	configure(root, (config) => {
		const member = includedMembers[0];
		requireCondition(member, 'Missing mechanical filter member');
		config['check'] = { files: [member] };
		const description = objectValue(
			objectValue(objectValue(config['evidence'])['kinds'])['description'],
		);
		description['inline_rule'] = '{language: Markdown, rule: {kind: document}}';
		description['max_chars'] = 1000;
		objectValue(description['fields'])['opening'] = { from: 'remainder' };
		const filter = objectValue(objectValue(config['filters'])['category']);
		objectValue(objectValue(objectValue(filter['questions'])['style'])['instructions'])['check'] =
			'description';
		const steps = objectValue(objectValue(filter['decision'])['steps']);
		objectValue(objectValue(steps['style'])['routes'])['approved'] = 'content';
		steps['content'] = {
			mechanical_check: { tool: 'contains', field: 'opening', text: mechanicalToken },
			yes: 'matcher',
			['no']: { label: 'blocked' },
		};
		steps['matcher'] = {
			mechanical_check: { tool: 'ast_grep', rule: 'marker' },
			yes: { label: 'eligible' },
			['no']: { label: 'blocked' },
		};
	});
	const rules = readObject(path.join(root, rulesName));
	writeJson(path.join(root, rulesName), { K001: rules['K001'] });
	filesystem.writeFileSync(
		path.join(root, rulebookName),
		'Instructions have approval. <!-- K001 -->\n',
	);
}

export function requireMechanicalFilterEvidence(evidence: JsonObject): void {
	const piece = objectValue(evidence['piece']);
	const raw = textValue(objectValue(objectValue(piece['fields'])['opening'])['value']);
	requireCondition(
		raw.includes(mechanicalOpening.trimEnd()),
		'Saved filter lost raw mechanical field',
	);
	const modelView = objectValue(piece['model_view']);
	requireCondition(
		arrayValue(modelView['cut']).includes('opening'),
		'Long mechanical field was not projected with a cut',
	);
	const projected = textValue(objectValue(modelView['fields'])['opening']);
	requireCondition(
		!projected.includes(mechanicalToken),
		'Projected opening retained the middle control token',
	);
	requireCondition(
		arrayValue(objectValue(evidence['matches'])['marker']).length > 0,
		'Saved filter lost collected AST matches',
	);
	// The saved raw field alone reconstructs the contains branch; the cut projection would take the other branch.
	equal(raw.includes(mechanicalToken), true, 'Saved raw input cannot re-walk mechanical contains');
	equal(
		projected.includes(mechanicalToken),
		false,
		'Control cannot distinguish raw from projected input',
	);
}

export async function observeMechanicalFilter(
	observer: RulebookProcess,
	root: string,
	evidenceRoot: string,
): Promise<void> {
	configureEndpoint(root, observer.endpoint);
	observer.respond = ({ body }) =>
		Promise.resolve({ status: 200, body: filterReply(body, 'judged') });
	const checked = await observer.run({
		root,
		name: 'mechanical-filter-check',
		arguments: ['lint', '--config', configName, '--check'],
		expectedExit: 0,
	});
	equal(checked.captures.length, 0, 'Mechanical filter check dispatched');
	const saved = path.join(evidenceRoot, 'mechanical-filter-report.json');
	const run = await observer.run({
		root,
		name: 'mechanical-filter',
		arguments: ['lint', '--config', configName, '--all', '--report', saved],
		credential: true,
		expectedExit: 0,
	});
	equal(run.captures.length, 1, 'Mechanical filter required another model call');
	const sent = run.captures[0]?.body;
	requireCondition(sent, 'Filter request missing');
	requireCondition(
		!('opening' in objectValue(sent['state'])) && !JSON.stringify(sent).includes(mechanicalToken),
		'Model-unused mechanical field leaked to HTTP',
	);
	const report = readObject(saved);
	const filters = arrayValue(report['filter_answers']).map(objectValue);
	equal(filters.length, 1, 'Mechanical filter missing or duplicated');
	const filter = filters[0];
	requireCondition(filter, 'Filter absent');
	equal(filter['label'], 'eligible', 'Mechanical raw input did not reach eligible');
	equal(filter['path'], ['scope', 'style', 'content', 'matcher'], 'Mechanical route lost');
	const id = numberValue(filter['id']);
	const evidence = objectValue(objectValue(report['piece_evidence'])[String(id)]);
	requireMechanicalFilterEvidence(evidence);
	const record = objectValue(arrayValue(report['requests'])[0]);
	equal(
		objectValue(record['request'])['state'],
		sent['state'],
		'Saved filter state differs from HTTP',
	);
	equal(
		objectValue(record['request'])['questions'],
		sent['questions'],
		'Saved filter questions differ from HTTP',
	);
	const details: JsonObject[] = [];
	await runSequence(['before', 'after'] as const, async (phase) => {
		if (phase === 'after')
			for (const file of [
				configName,
				rulebookName,
				rulesName,
				`${includedFolder}/overview.md`,
				'evidence/rules/marker.yml',
			])
				filesystem.unlinkSync(path.join(root, file));
		const shown = await observer.run({
			root,
			name: `mechanical-filter-show-${phase}`,
			arguments: ['lint', 'show', saved, String(id)],
			expectedExit: 0,
		});
		equal(shown.captures.length, 0, 'Saved mechanical filter show dispatched');
		const detail = parseObject(shown.stdout.toString('utf8'));
		equal(detail['result'], filter, 'Show selected another filter');
		equal(detail['evidence'], evidence, 'Show lost saved raw filter evidence');
		equal(detail['rewalkable'], true, 'Completed mechanical filter cannot be re-walked');
		details.push(detail);
	});
	equal(details[0], details[1], 'Saved filter depends on current context/rulebook files');
}
