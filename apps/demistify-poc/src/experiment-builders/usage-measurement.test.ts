import { expect, it } from 'vitest';

import { measureUsageFiles } from './usage-measurement.ts';

it('counts complete authored files while refusing library assets', (): void => {
	const measurement = measureUsageFiles([
		{ path: 'experiments/graph-diff/planning.md', source: 'hello world' },
		{ path: 'experiments/graph-diff/planning-changes.json', source: 'hello world' },
	]);
	expect(measurement.usageTokens).toBe(4);
	expect(measurement.usageBytes).toBe(22);
	expect(measurement.authorFiles).toHaveLength(2);
	expect(() =>
		measureUsageFiles([{ path: 'src/html-islands/runtime.ts', source: 'hello world' }]),
	).toThrow('usage');
	expect(() =>
		measureUsageFiles([
			{ path: 'experiments/quick-json/upstream/base.css', source: 'hello world' },
		]),
	).toThrow('usage');
});

it('treats special-token-looking text as document content', (): void => {
	const measurement = measureUsageFiles([
		{ path: 'experiments/html-islands/planning.html', source: '<|endoftext|>' },
	]);
	expect(measurement.usageTokens).toBeGreaterThan(1);
	expect(measurement.tokenEncoding).toBe('o200k_base');
});

it('refuses empty manifests and duplicate files that would distort the total', (): void => {
	expect(() => measureUsageFiles([])).toThrow('author');
	const authorFile = { path: 'experiments/html-islands/planning.html', source: 'hello world' };
	expect(() => measureUsageFiles([authorFile, authorFile])).toThrow('Duplicate');
});
