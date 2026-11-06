import { expect, test } from 'vitest';

import { checkHtml, type Finding } from './check.ts';

async function findingsFor(content: string): Promise<Finding[]> {
	return checkHtml(
		`<!doctype html><html><head><link rel="stylesheet" href="/theme.css"><script type="module" src="/runtime.js"></script></head><body><ex-document>${content}</ex-document></body></html>`,
		{ diagrams: false },
	);
}

test('the checker accepts tab sections with a label and a curated subject icon', async (): Promise<void> => {
	const findings = await findingsFor(`
		<ex-tabs label="Reconnect explanations">
			<section id="normal-path" label="Normal path" icon="path"><p>The registry keeps the session.</p></section>
			<section id="missing-path" label="Missing session" icon="database"><p>A replacement connection starts again.</p></section>
		</ex-tabs>
	`);
	expect(findings).toEqual([]);
});

test('the checker reports a tab section icon that is not a curated subject icon', async (): Promise<void> => {
	const findings = await findingsFor(`
		<ex-tabs label="Reconnect explanations">
			<section id="normal-path" label="Normal path" icon="zebra-unicorn"><p>The registry keeps the session.</p></section>
		</ex-tabs>
	`);
	const problem = findings.find((finding): boolean => finding.rule === 'attribute');
	expect(problem).toMatchObject({ severity: 'error', rule: 'attribute' });
	expect(problem?.message).toContain('icon="zebra-unicorn"');
});
