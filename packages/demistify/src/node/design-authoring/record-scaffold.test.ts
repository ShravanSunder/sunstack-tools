import { execFile } from 'node:child_process';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

import { expect, it } from 'vitest';
import { z } from 'zod';

const run = promisify(execFile);
const cli = fileURLToPath(new URL('../../../bin/demistify.ts', import.meta.url));

const decisionsSource = `export default [
  {
    "id": "D2",
    "kind": "ruling",
    "statement": "Keep a time floor.",
    "authority": { "state": "agent-decided", "by": { "alias": "lead", "role": "lead" }, "sourceKind": "author", "rationale": "Clocks move.", "decidedOn": "2026-11-20", "weight": "design" }
  }
];
`;

const page = `<!doctype html><html lang="en"><head><title>Shared decisions</title>
<meta name="demistify-records" content="records.generated.js"></head><body><ex-document id="content">
<h1>Shared decisions</h1>
<ex-section id="open" title="For you to decide" holds="needs-owner"></ex-section>
<ex-section id="agent" title="Decided without you" holds="decided-by-agent"></ex-section>
<ex-section-item record="initiative/D2"><p>A floor.</p></ex-section-item>
<ex-section id="settled" title="Settled" holds="settled"></ex-section>
<ex-section id="assumed" title="Assumed" holds="assumed"></ex-section>
</ex-document></body></html>`;

it('scaffolds a record, its section item and the owner words in one step', async (): Promise<void> => {
	const root = await mkdtemp(join(tmpdir(), 'demistify-initiative-add-'));
	const pageFile = join(root, 'decisions.html');
	const add = (...args: string[]): ReturnType<typeof run> =>
		run(process.execPath, [
			cli,
			'initiative',
			'add',
			...args.slice(0, 1),
			root,
			'--page',
			pageFile,
			...args.slice(1),
		]);
	const decisionsFile = join(root, 'records/initiative-decisions.ts');
	try {
		await mkdir(join(root, 'records'));
		await Promise.all([
			writeFile(decisionsFile, decisionsSource),
			writeFile(join(root, 'records/initiative-assumptions.ts'), 'export default [];\n'),
			writeFile(
				join(root, 'initiative.ts'),
				`import decisions from './records/initiative-decisions.ts';
import assumptions from './records/initiative-assumptions.ts';
export default { decisions, assumptions, pages: ['decisions.html'] };`,
			),
			writeFile(pageFile, page),
		]);

		const preview = await add('choice', '--namespace', 'initiative', '--words', 'pick one <soon>');
		expect(preview.stdout).toContain('initiative/D3 · new choice from the owner’s words');
		expect(await readFile(decisionsFile, 'utf8')).toBe(decisionsSource);

		await writeFile(
			pageFile,
			page.replace('<ex-section id="assumed" title="Assumed" holds="assumed"></ex-section>', ''),
		);
		await expect(
			add('assumption', '--namespace', 'initiative', '--author', 'lead'),
		).rejects.toThrow('has no <ex-section holds="assumed">');
		await writeFile(pageFile, page);

		await add('choice', '--namespace', 'initiative', '--words', 'pick one <soon>', '--write');
		const withChoice = await readFile(pageFile, 'utf8');
		expect(withChoice).toContain(
			'holds="needs-owner"></ex-section>\n<ex-section-item record="initiative/D3"><ex-choice><ex-option data-option="option-a">',
		);
		expect(withChoice).toMatch(
			/<p id="initiative\/D3-source">initiative\/D3\. Your words, \d{4}-\d{2}-\d{2}: "pick one &lt;soon&gt;"<\/p>\n<\/ex-document>/u,
		);
		const decisions = await readFile(decisionsFile, 'utf8');
		expect(decisions).toContain(`  },
  {
    "id": "D3",
    "title": "TODO: a short title for D3",
    "sources": [
      {
        "href": "../decisions.html#initiative%2FD3-source",
        "excerpt": "pick one <soon>",
        "verbatim": true
      }
    ],
    "kind": "choice",`);

		await add('ruling', '--namespace', 'initiative', '--author', 'lead', '--write');
		expect(await readFile(pageFile, 'utf8')).toContain(
			'holds="decided-by-agent"></ex-section>\n<ex-section-item record="initiative/D4"><p>TODO: explain initiative/D4 here.</p></ex-section-item>',
		);

		await expect(add('assumption', '--namespace', 'initiative')).rejects.toThrow(
			'needs --author <alias>',
		);
		await add('assumption', '--namespace', 'initiative', '--author', 'lead', '--write');
		expect(await readFile(join(root, 'records/initiative-assumptions.ts'), 'utf8')).toContain(
			"[\n  {\n    id: 'A1',",
		);

		const view = z
			.object({ records: z.array(z.object({ id: z.string() })) })
			.parse(JSON.parse(await readFile(join(root, 'initiative.generated.json'), 'utf8')));
		expect(view.records.map(({ id }) => id).toSorted()).toEqual([
			'initiative/A1',
			'initiative/D2',
			'initiative/D3',
			'initiative/D4',
		]);

		const current = await readFile(pageFile, 'utf8');
		await writeFile(
			pageFile,
			current.replace('<ex-section id="assumed" title="Assumed" holds="assumed"></ex-section>', ''),
		);
		await expect(
			add('assumption', '--namespace', 'initiative', '--author', 'lead'),
		).rejects.toThrow('holds="assumed"');
	} finally {
		await rm(root, { recursive: true, force: true });
	}
}, 30_000);
