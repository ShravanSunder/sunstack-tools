/** O1/O2/O4 through the pinned fresh installed wrapper and actual native addon. */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import path from 'node:path';

import { requireCaseAccounting } from '../case-dispatch/case-dispatch-checks.ts';
import { runSequence } from '../rulebook/rulebook-fixture-support.ts';
import { type ReleasedTarget, type RulebookProcess } from '../rulebook/rulebook-process.ts';
import { boundedSdkWait } from '../sdk/sdk-controlled-backend.ts';
import { saveSdkEvidence, type SdkExports } from '../sdk/sdk-target.ts';
import {
	integer,
	record,
	rows,
	requireLocalParity,
	requireProviderFailure,
	requireRequestParity,
	requireUnavailableCounts,
	requireUnavailableResults,
	requireUnavailableSelection,
	requireZeroDispatch,
	status,
} from './missing-checks.ts';
import {
	judgedNames,
	localNames,
	sourceDigest,
	sourceFile,
	sourceText,
	writeFixture,
} from './missing-fixtures.ts';
import { type MissingCredentialBackend } from './missing-http.ts';

export interface SdkQualification {
	readonly root: string;
	readonly reports: readonly { readonly name: string; readonly report: Record<string, unknown> }[];
}
export async function observeMissingSdk(options: {
	readonly sdk: SdkExports;
	readonly backend: MissingCredentialBackend;
	readonly evidence: string;
	readonly observer: RulebookProcess;
	readonly reference: ReleasedTarget;
}): Promise<SdkQualification> {
	const { sdk, backend, evidence } = options;
	assert.ok(sdk.inspect, 'Installed SDK lacks inspect');
	const inspect = sdk.inspect;
	const root = path.join(evidence, 'sdk-fixture');
	const reports: { name: string; report: Record<string, unknown> }[] = [];
	const inspectCall = async (
		name: string,
		input: unknown,
		extra: Record<string, unknown> = {},
	): Promise<Record<string, unknown>> => {
		const controller = new AbortController();
		const pending = inspect(input, {
			root,
			scope: { kind: 'all' },
			signal: controller.signal,
			...extra,
		});
		try {
			return record(await boundedSdkWait(pending, name));
		} finally {
			controller.abort();
			await boundedSdkWait(Promise.allSettled([pending]), `${name} settlement`);
		}
	};
	const load = async (): Promise<unknown> => {
		const loaded = record(await boundedSdkWait(sdk.load({ directory: root }), 'native load'));
		assert.equal(loaded['kind'], 'loaded');
		return loaded['value'];
	};
	const run = async (
		name: string,
		configuration: unknown,
		expected: string,
		extra: Record<string, unknown> = {},
	): Promise<Record<string, unknown>> => {
		const cancellation = new AbortController();
		const pending = sdk.run(configuration, {
			root,
			scope: { kind: 'all' },
			signal: cancellation.signal,
			...extra,
		});
		try {
			const outcome = record(await boundedSdkWait(pending, name));
			saveSdkEvidence(path.join(evidence, `${name}.json`), outcome);
			assert.equal(outcome['kind'], expected, `${name}: wrong native outcome`);
			return outcome;
		} finally {
			cancellation.abort();
			await boundedSdkWait(Promise.allSettled([pending]), `${name} settlement`);
		}
	};
	writeFixture(root, backend.endpoint, 'mixed');
	let configuration = await load();
	process.env['OPENROUTER_API_KEY'] = 'SYNTHETIC_MISSING_CREDENTIAL_LOOPBACK_ONLY';
	let before = backend.captures.length;
	const keyed = await run('sdk-keyed-witness', configuration, 'completed');
	const witness = backend.captures.slice(before);
	assert.ok(witness.length > 0, 'No keyed native HTTP witness');
	assert.ok(integer(record(record(keyed['report'])['usage'])['model_requests']) > 0n);
	before = backend.captures.length;
	await options.observer.run({
		target: options.reference,
		root,
		name: 'sdk-keyed-released-reference',
		arguments: ['lint', '--config', 'sunstack.lint.json', '--all', '--format', 'json'],
		expectedExit: 1,
		credential: true,
	});
	requireRequestParity(witness, backend.captures.slice(before));
	delete process.env['OPENROUTER_API_KEY'];
	writeFixture(root, backend.endpoint, 'mechanical');
	const mechanical = record((await run('sdk-mechanical', await load(), 'completed'))['report']);
	writeFixture(root, backend.endpoint, 'mixed');
	configuration = await load();
	before = backend.captures.length;
	const mixed = record((await run('sdk-mixed-keyless', configuration, 'incomplete'))['report']);
	assert.equal(status(mixed), 'incomplete');
	requireLocalParity({
		mechanical,
		mixed,
		names: localNames,
		expectedFindings: 3,
		expectedSha256: sourceDigest,
	});
	requireUnavailableResults(
		mixed,
		new Map(judgedNames.map((name) => [name, 'credential_unavailable'])),
	);
	requireUnavailableCounts(mixed);
	requireZeroDispatch(mixed, backend.captures.slice(before));
	reports.push({ name: 'mixed', report: mixed });
	await runSequence(['filter', 'selection'] as const, async (mode) => {
		writeFixture(root, backend.endpoint, mode);
		before = backend.captures.length;
		const report = record((await run(`sdk-${mode}`, await load(), 'incomplete'))['report']);
		requireUnavailableCounts(report);
		requireZeroDispatch(report, backend.captures.slice(before));
		const expected = new Map<string, string>(
			judgedNames.map((name) => [name, 'credential_unavailable']),
		);
		expected.set(
			mode === 'filter' ? 'gated_local' : 'needed_field',
			mode === 'filter' ? 'filter_unavailable' : 'selected_field_unavailable',
		);
		requireUnavailableResults(report, expected);
		if (mode === 'selection') {
			requireUnavailableSelection(report);
			assert.ok(!rows(report['results']).some((row) => row['name'] === 'selected_child'));
			assert.equal(
				rows(report['results']).find((row) => row['name'] === 'unused_field')?.['outcome'],
				'finding',
			);
			assert.ok(
				rows(report['incomplete']).some(
					(row) => record(row['reason'])['kind'] === 'selection_unavailable',
				),
			);
			const selected = record(
				(
					await run('sdk-unknown-child-case', await load(), 'incomplete', {
						case_selection: [
							{
								id: 'unknown-child',
								kind: 'rule',
								rule: 'selected_child',
								file: sourceFile,
								lines: [1n, 1n],
								text_sha256: createHash('sha256').update(sourceText).digest('hex'),
							},
						],
					})
				)['report'],
			);
			const unknown = rows(record(selected['case_selection'])['bindings']);
			assert.equal(unknown.length, 1);
			assert.equal(record(unknown[0]?.['outcome'])['kind'], 'unavailable');
			assert.equal(
				rows(selected['results']).filter((row) => row['name'] === 'selected_child').length,
				0,
			);
			reports.push({ name: 'unknown-child-case', report: selected });
		}
		reports.push({ name: mode, report });
	});
	writeFixture(root, backend.endpoint, 'mixed');
	configuration = await load();
	const binding = {
		kind: 'rule',
		rule: 'judged_first',
		file: sourceFile,
		lines: [1n, 1n],
		text_sha256: createHash('sha256').update(sourceText).digest('hex'),
	};
	const selected = record(
		(
			await run('sdk-selected-cases', configuration, 'incomplete', {
				case_selection: ['known-a', 'known-b'].map((id) => Object.assign({ id }, binding)),
			})
		)['report'],
	);
	const bindings = rows(record(selected['case_selection'])['bindings']);
	requireCaseAccounting({
		selectedIds: ['known-a', 'known-b'],
		dispositions: bindings.map((row) => ({
			id: String(record(row['binding'])['id']),
			kind: String(record(row['outcome'])['kind']),
		})),
		expectedKinds: new Map([
			['known-a', 'unavailable'],
			['known-b', 'unavailable'],
		]),
	});
	assert.equal(
		rows(selected['results']).filter((row) => row['name'] === 'judged_first').length,
		1,
		'Two case IDs must share one real subject',
	);
	assert.equal(
		rows(selected['results']).filter((row) => row['name'] === 'judged_second').length,
		0,
		'Excluded case owner executed',
	);
	reports.push({ name: 'selected-cases', report: selected });
	const empty = record(
		(await run('sdk-empty-cases', configuration, 'completed', { case_selection: [] }))['report'],
	);
	assert.deepEqual(empty['results'], []);
	assert.equal(status(empty), 'nothing_selected');

	before = backend.captures.length;
	assert.equal(
		record(await boundedSdkWait(sdk.check(configuration), 'static check'))['kind'],
		'valid',
	);
	const staticInspection = await inspectCall('static inspection', configuration);
	saveSdkEvidence(path.join(evidence, 'sdk-static-inspection.json'), staticInspection);
	assert.equal(staticInspection['kind'], 'inspected');
	assert.ok(rows(record(staticInspection['report'])['pieces']).length > 0);
	assert.equal(backend.captures.length, before);
	writeFixture(root, backend.endpoint, 'selection');
	const selectionConfiguration = await load();
	const missing = await inspectCall('model inspection missing key', selectionConfiguration);
	saveSdkEvidence(path.join(evidence, 'sdk-model-inspection-keyless.json'), missing);
	assert.equal(missing['kind'], 'invalid');
	assert.ok(!('report' in missing));
	assert.equal(rows(missing['diagnostics'])[0]?.['code'], 'credential_missing');
	const preAborted = new AbortController();
	preAborted.abort();
	const cancelledInspection = await inspectCall(
		'pre-aborted model inspection',
		selectionConfiguration,
		{ signal: preAborted.signal },
	);
	assert.equal(cancelledInspection['kind'], 'cancelled');
	assert.equal(cancelledInspection['report'], null);
	const relativeInspection = await inspectCall('relative-root inspection', selectionConfiguration, {
		root: 'relative',
	});
	assert.equal(relativeInspection['kind'], 'invalid');
	const capture = await inspectCall('unavailable definition capture', {
		id: 'missing-capture',
		base_directory: root,
		marker_prefix: 'K',
		check: { files: [sourceFile] },
		decision_model: { model: 'jev-latest' },
		rulebooks: [{ kind: 'files', markdown: 'missing.md', entries: 'missing.json' }],
		evidence: { kind: 'project', config_path: 'evidence/sgconfig.yml', kinds: {} },
	});
	assert.equal(capture['kind'], 'unavailable');
	assert.ok(!('report' in capture));
	saveSdkEvidence(path.join(evidence, 'sdk-inspection-preserved-boundaries.json'), {
		cancelledInspection,
		relativeInspection,
		capture,
	});
	process.env['OPENROUTER_API_KEY'] = 'SYNTHETIC_MISSING_CREDENTIAL_LOOPBACK_ONLY';
	before = backend.captures.length;
	const inspected = await inspectCall('keyed model inspection', selectionConfiguration);
	saveSdkEvidence(path.join(evidence, 'sdk-model-inspection-keyed.json'), inspected);
	assert.equal(inspected['kind'], 'inspected');
	assert.ok(backend.captures.length > before);
	for (const raw of backend.captures.slice(before))
		assert.ok(
			!raw.includes('Does this synthetic paragraph comply?'),
			'Inspection evaluated rule questions',
		);
	writeFixture(root, backend.endpoint, 'mixed');
	configuration = await load();
	await runSequence([401, 400, 503], async (code) => {
		backend.status = code;
		before = backend.captures.length;
		const report = record((await run(`sdk-http-${code}`, configuration, 'incomplete'))['report']);
		requireProviderFailure(report, code, backend.captures.length - before);
		assert.ok(backend.captures.length > before);
		assert.ok(rows(report['requests']).length > 0);
		assert.equal(
			rows(report['results']).filter((row) => row['outcome'] === 'unavailable').length,
			0,
		);
	});
	backend.status = 200;
	backend.hold = true;
	const cancellation = new AbortController();
	before = backend.captures.length;
	const pending = sdk.run(configuration, {
		root,
		scope: { kind: 'all' },
		signal: cancellation.signal,
	});
	try {
		await backend.waitForRequests(before + 1);
		cancellation.abort();
		const cancelled = record(await boundedSdkWait(pending, 'acknowledged native cancellation'));
		saveSdkEvidence(path.join(evidence, 'sdk-held-cancel.json'), cancelled);
		assert.equal(cancelled['kind'], 'cancelled');
	} finally {
		cancellation.abort();
		backend.release();
		await boundedSdkWait(Promise.allSettled([pending]), 'cancelled native settlement');
	}
	delete process.env['OPENROUTER_API_KEY'];
	const aborted = new AbortController();
	aborted.abort();
	const preAbort = record(
		await boundedSdkWait(
			sdk.run(configuration, { root, signal: aborted.signal }),
			'pre-aborted run',
		),
	);
	assert.equal(preAbort['kind'], 'cancelled');
	const relative = record(
		await boundedSdkWait(sdk.run(configuration, { root: 'relative' }), 'relative run root'),
	);
	assert.equal(relative['kind'], 'invalid');
	writeFixture(root, backend.endpoint, 'invalid');
	const invalidLoad = record(
		await boundedSdkWait(sdk.load({ directory: root }), 'invalid configuration load'),
	);
	saveSdkEvidence(path.join(evidence, 'sdk-invalid-load.json'), invalidLoad);
	assert.equal(invalidLoad['kind'], 'invalid');
	const captured = record(record(configuration)['configuration']);
	const invalidConfiguration = { ...captured, check: { ...record(captured['check']), files: [] } };
	await run('sdk-invalid-global', invalidConfiguration, 'invalid');
	return { root, reports };
}
