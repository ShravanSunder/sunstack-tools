import { describe, expect, test } from 'vitest';

import { requireSdkMissingAddonExampleResult } from './sdk-examples-unavailable.ts';

describe('missing-addon example observation', () => {
	test('accepts a visible typed before-dispatch refusal', () => {
		const result = {
			name: 'example',
			status: 1,
			stdout: '',
			stderr: "phase: 'before_dispatch', reason: 'addon_missing'",
			error: undefined,
		};
		expect(() => requireSdkMissingAddonExampleResult(result)).not.toThrow();
	});
	test('rejects a successful exit, an unrelated error and a missing phase', () => {
		const result = {
			name: 'example',
			status: 1,
			stdout: '',
			stderr: "phase: 'before_dispatch', reason: 'addon_missing'",
			error: undefined,
		};
		expect(() => requireSdkMissingAddonExampleResult({ ...result, status: 0 })).toThrow('exit 1');
		expect(() =>
			requireSdkMissingAddonExampleResult({ ...result, stderr: 'configuration invalid' }),
		).toThrow('phase/reason');
		expect(() =>
			requireSdkMissingAddonExampleResult({ ...result, stderr: 'addon_missing' }),
		).toThrow('phase/reason');
	});
});
