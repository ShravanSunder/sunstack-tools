import { execFile } from 'node:child_process';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

import { expect, it } from 'vitest';

import { formatReview, reviewEnvelopeSchema } from '../../core/review-model.ts';

const run = promisify(execFile);
const cli = fileURLToPath(new URL('../../../bin/demistify.ts', import.meta.url));

const decisions = `export default [
  {
    id: 'D2',
    kind: 'choice',
    question: 'Should an expired row stay visible until the reader scrolls?',
    options: [
      { id: 'hide-at-once', title: 'Hide at once' },
      { id: 'grey-until-scroll', title: 'Grey until scroll' },
    ],
    recommended: 'none',
    authority: { state: 'owner-open' },
  },
];
`;

const page = `<!doctype html><html lang="en"><head><title>Design</title>
<link rel="stylesheet" href="https://demistify.getagentstudio.dev/v1/theme.css">
<script type="module" src="https://demistify.getagentstudio.dev/v1/runtime.js"></script></head><body>
<ex-nav for="content" src="./navigation.generated.json"></ex-nav><ex-document id="content"><h1>Design</h1>
<ex-section id="open" title="For you to decide" holds="needs-owner"></ex-section>
<ex-section-item record="D2"><ex-choice><ex-option data-option="hide-at-once"><template slot="summary">Hide.</template></ex-option><ex-option data-option="grey-until-scroll"><template slot="summary">Grey.</template></ex-option></ex-choice></ex-section-item>
<ex-section id="agent" title="Decided without you" holds="decided-by-agent"></ex-section>
<ex-section id="settled" title="Settled" holds="settled"></ex-section>
</ex-document></body></html>`;

function review(option: string, label: string): string {
	return formatReview(
		reviewEnvelopeSchema.parse({
			documentId: 'design',
			documentRevision: 'r1',
			format: 'demistify-review-1',
			reviewId: 'review-1',
			comments: [],
			answers: [
				{
					questionId: 'D2',
					decisionId: 'D2',
					question: 'D2',
					selected: [{ optionId: `D2--${option}`, label }],
				},
			],
		}),
	);
}

it('design add and design answer edit a standalone design set by its local IDs', async (): Promise<void> => {
	const root = await mkdtemp(join(tmpdir(), 'demistify-design-edits-'));
	const project = join(root, 'design');
	const pageFile = join(project, 'index.html');
	const decisionsFile = join(project, 'design-decisions.ts');
	try {
		await run(process.execPath, [cli, 'design', 'init', project]);
		await Promise.all([writeFile(decisionsFile, decisions), writeFile(pageFile, page)]);
		await run(process.execPath, [cli, 'design', 'check', project, '--write']);

		const added = await run(process.execPath, [
			cli,
			'design',
			'add',
			'ruling',
			project,
			'--page',
			pageFile,
			'--author',
			'lead',
			'--write',
		]);
		const reviewFile = join(root, 'review.md');
		await writeFile(reviewFile, review('grey-until-scroll', 'Grey until scroll'));
		const answered = await run(process.execPath, [
			cli,
			'design',
			'answer',
			reviewFile,
			project,
			'--decided-on',
			'2026-11-26',
			'--write',
		]);

		expect(added.stdout).toContain('D3 · new ruling');
		expect(added.stdout).toContain('Warning: D3: still holds a TODO: placeholder');
		const recordText = await readFile(decisionsFile, 'utf8');
		expect(recordText).toContain("id: 'D3'");
		expect(recordText).toContain("chosen: 'grey-until-scroll'");
		expect(await readFile(pageFile, 'utf8')).toContain('<ex-section-item record="D3">');
		expect(answered.stdout).toContain(
			'chosen: grey-until-scroll (Grey until scroll) · owner-accepted',
		);
		expect(answered.stdout).toContain('Applied.');
		await expect(
			run(process.execPath, [
				cli,
				'design',
				'add',
				'choice',
				project,
				'--namespace',
				'initiative',
				'--page',
				pageFile,
			]),
		).rejects.toThrow(
			'A standalone design set has no namespaces; leave out --namespace initiative.',
		);
	} finally {
		await rm(root, { recursive: true, force: true });
	}
}, 60_000);
