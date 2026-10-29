import assert from 'node:assert/strict';
import fileSystem from 'node:fs';

import { reportSchema, type LintReport, type ProcessObservation } from './release-records.ts';
type GradedProcess = {
	stdout: Buffer;
	observation: Pick<ProcessObservation, 'error' | 'sockets_before_cleanup'> & {
		root_close: { code: number | null; signal: NodeJS.Signals | null } | null;
		survivors_before_cleanup: { pid: number }[] | null;
		admitted_request?: { state: unknown } | null;
	};
};

export function stopExpectation(
	route: string,
	signal: string,
	destination: string,
	platform: string = process.platform,
): string {
	if (destination === 'group')
		return route === 'npx' && platform === 'linux' ? 'cancelled-native/npx-int' : 'cancel4';
	if (
		(route === 'npx' && platform === 'linux') ||
		(signal === 'SIGHUP' && ['exec-script', 'npx', 'exec-npx'].includes(route))
	)
		return 'no-cancelled-report';
	if (signal === 'SIGHUP')
		return route === 'pnpm-dlx' ? 'native-ended' : 'native-ended/caller-sighup';
	return 'cancel4';
}

export function assertReportMatches(
	actual: LintReport,
	reference: LintReport,
	version: string,
	configPath: string,
): void {
	assert.deepEqual(
		actual,
		{ ...reference, version, config: fileSystem.realpathSync(configPath) },
		'full lint report',
	);
}

export function gradeStop(
	result: GradedProcess,
	expected: string,
	reference: LintReport,
	version: string,
	configPath: string,
): string {
	const facts = result.observation;
	assert.equal(
		facts.error,
		null,
		facts.error && facts.error.length > 0 ? facts.error : 'capture failed',
	);
	if (expected === 'no-cancelled-report') {
		assert.doesNotMatch(result.stdout.toString(), /"kind"\s*:\s*"cancelled"/);
		return 'limit';
	}
	assert.deepEqual(facts.survivors_before_cleanup, [], 'survivors before cleanup');
	assert.equal(facts.sockets_before_cleanup, 0, 'held socket before cleanup');
	assert.ok(facts.root_close, 'caller did not close');
	if (expected === 'native-ended/caller-sighup') {
		assert.equal(facts.root_close.signal, 'SIGHUP');
		assert.equal(facts.root_close.code, null);
	} else if (expected !== 'native-ended') {
		if (expected === 'cancelled-native/npx-int')
			assert.ok(facts.root_close.signal === 'SIGINT' || facts.root_close.code === 130);
		else {
			assert.equal(facts.root_close.code, 4);
			assert.equal(facts.root_close.signal, null);
		}
		const actual = reportSchema.parse(JSON.parse(result.stdout.toString()));
		assertReportMatches(actual, reference, version, configPath);
		assert.deepEqual(
			Object.values(actual.evidence_seen ?? {}),
			[facts.admitted_request?.state],
			'report state equals the actual sent state',
		);
	}
	return 'pass';
}
