import assert from 'node:assert/strict';
import fileSystem from 'node:fs';

import { lintReportSchema } from './release-channel-contracts.ts';

interface StopCapture {
	readonly stdout: Buffer;
	readonly observation: {
		readonly error: string | null;
		readonly survivors_before_cleanup: readonly unknown[] | null;
		readonly sockets_before_cleanup: number;
		readonly root_close?: {
			readonly code: number | null;
			readonly signal: NodeJS.Signals | null;
		} | null;
		readonly admitted_request?: { readonly state: unknown } | null;
	};
}
interface StopExpectation {
	readonly route: string;
	readonly signal: string;
	readonly destination: string;
	readonly platform?: string;
}
export function stopExpectation({
	route,
	signal,
	destination,
	platform = process.platform,
}: StopExpectation): string {
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
interface ReportComparison {
	readonly actual: unknown;
	readonly reference: Record<string, unknown>;
	readonly version: string;
	readonly configPath: string;
}
export function assertReportMatches({
	actual,
	reference,
	version,
	configPath,
}: ReportComparison): void {
	assert.deepEqual(
		actual,
		{ ...reference, version, config: fileSystem.realpathSync(configPath) },
		'full lint report',
	);
}
interface StopGrading {
	readonly result: StopCapture;
	readonly expected: string;
	readonly reference: Record<string, unknown>;
	readonly version: string;
	readonly configPath: string;
}
export function gradeStop({
	result,
	expected,
	reference,
	version,
	configPath,
}: StopGrading): string {
	const facts = result.observation;
	assert.equal(facts.error, null, facts.error?.length ? facts.error : 'capture failed');
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
		const actual = lintReportSchema.parse(JSON.parse(result.stdout.toString('utf8')));
		assertReportMatches({
			actual,
			reference,
			version,
			configPath,
		});
		assert.ok(actual.evidence_seen);
		assert.ok(facts.admitted_request);
		assert.deepEqual(
			Object.values(actual.evidence_seen),
			[facts.admitted_request.state],
			'report state equals the actual sent state',
		);
	}
	return 'pass';
}
