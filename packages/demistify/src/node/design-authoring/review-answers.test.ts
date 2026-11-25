import { execFile } from 'node:child_process';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

import { expect, it } from 'vitest';
import { z } from 'zod';

import { formatReview, reviewEnvelopeSchema } from '../../core/review-model.ts';

const run = promisify(execFile);
const cli = fileURLToPath(new URL('../../../bin/demistify.ts', import.meta.url));

const decisionsSource = `export default [
  {
    id: 'D2',
    kind: 'choice',
    question: 'When can a request pick up leftover work?',
    options: [
      { id: 'wait', title: 'Wait for the next mark' },
      { id: 'resume', title: 'Resume overdue work' },
    ],
    recommended: 'none',
    authority: { state: 'owner-open' },
  },
  {
    id: 'D3',
    kind: 'choice',
    question: 'Where do retries log?',
    options: [
      { id: 'journal', title: 'In the journal' },
      { id: 'none', title: 'Nowhere' },
    ],
    recommended: 'none',
    authority: { state: 'owner-open' },
  },
];
`;

function choiceItem(id: string, options: readonly string[]): string {
	const markup = options
		.map(
			(option) =>
				`<ex-option data-option="${option}"><template slot="summary">${option}.</template></ex-option>`,
		)
		.join('');
	return `<ex-section-item record="initiative/${id}"><ex-choice>${markup}</ex-choice></ex-section-item>`;
}

const decisionsPage = `<!doctype html><html lang="en"><head><title>Shared decisions</title>
<meta name="demistify-records" content="records.generated.js"></head><body><ex-document id="content">
<h1>Shared decisions</h1>
<ex-section id="open" title="For you to decide" holds="needs-owner"></ex-section>
${choiceItem('D2', ['wait', 'resume'])}
${choiceItem('D3', ['journal', 'none'])}
<ex-section id="agent" title="Decided without you" holds="decided-by-agent"></ex-section>
<ex-section id="settled" title="Settled" holds="settled"></ex-section>
</ex-document></body></html>`;

function copiedReview(
	answers: readonly { id: string; option: string; label: string }[],
	comments: readonly { target: string; body: string }[],
): string {
	const time = '2026-11-25T12:00:00.000Z';
	return formatReview(
		reviewEnvelopeSchema.parse({
			documentId: 'decisions',
			documentRevision: 'r1',
			format: 'demistify-review-1',
			reviewId: 'review-1',
			comments: comments.map(({ target, body }, index) => ({
				commentId: `c${index}`,
				anchor: { kind: 'entity', targetId: target, label: target },
				body,
				createdAt: time,
				updatedAt: time,
			})),
			answers: answers.map(({ id, option, label }) => ({
				questionId: `initiative/${id}`,
				decisionId: `initiative/${id}`,
				question: id,
				selected: [{ optionId: `initiative/${id}--${option}`, label }],
			})),
		}),
	);
}

it('applies a copied review answer to its open choice, with the owner words on the page', async (): Promise<void> => {
	const root = await mkdtemp(join(tmpdir(), 'demistify-initiative-answer-'));
	await mkdir(join(root, 'records'));
	const answer = (review: string, ...flags: string[]): ReturnType<typeof run> =>
		run(process.execPath, [
			cli,
			'initiative',
			'answer',
			review,
			root,
			'--decided-on',
			'2026-11-25',
			...flags,
		]);
	try {
		await Promise.all([
			writeFile(join(root, 'records/initiative-decisions.ts'), decisionsSource),
			writeFile(
				join(root, 'initiative.ts'),
				`import decisions from './records/initiative-decisions.ts';\nexport default { decisions, pages: ['decisions.html'] };`,
			),
			writeFile(join(root, 'decisions.html'), decisionsPage),
			writeFile(
				join(root, 'worded.md'),
				copiedReview(
					[{ id: 'D2', option: 'resume', label: 'Resume overdue work' }],
					[{ target: 'initiative/D2', body: 'yes resume it, <and> log each one' }],
				),
			),
			writeFile(
				join(root, 'clicked.md'),
				copiedReview([{ id: 'D3', option: 'journal', label: 'In the journal' }], []),
			),
		]);
		await run(process.execPath, [cli, 'initiative', 'check', root, '--write']);

		const preview = await answer(join(root, 'worded.md'));
		expect(preview.stdout).toContain('initiative/D2 · When can a request pick up leftover work?');
		expect(preview.stdout).toContain('Preview only; nothing written.');
		expect(await readFile(join(root, 'records/initiative-decisions.ts'), 'utf8')).toBe(
			decisionsSource,
		);

		const applied = await answer(join(root, 'worded.md'), '--write');
		expect(applied.stdout).toContain('Applied.');
		const page = await readFile(join(root, 'decisions.html'), 'utf8');
		expect(page).toContain(
			'<p id="initiative/D2-answer">Decision initiative/D2. Your words, 2026-11-25, in review review-1: "yes resume it, &lt;and&gt; log each one"</p>\n</ex-document>',
		);
		const decided = await readFile(join(root, 'records/initiative-decisions.ts'), 'utf8');
		expect(decided).toContain(`    authority: {
      state: 'owner-decided',
      sourceKind: 'owner-words',
      decidedOn: '2026-11-25',
      chosen: 'resume',
      decision: 'Resume overdue work',
      source: {
        href: '../decisions.html#initiative%2FD2-answer',
        excerpt: 'yes resume it, <and> log each one',
        verbatim: true
      }
    },`);
		expect(decided).toContain("authority: { state: 'owner-open' },");

		const clashingPage = page.replace('<h1>', "<span id='initiative/D3-answer'></span><h1>");
		await Promise.all([
			writeFile(join(root, 'decisions.html'), clashingPage),
			writeFile(
				join(root, 'clashing.md'),
				copiedReview(
					[{ id: 'D3', option: 'journal', label: 'In the journal' }],
					[{ target: 'initiative/D3--journal', body: 'journal it' }],
				),
			),
		]);
		await expect(answer(join(root, 'clashing.md'), '--write')).rejects.toThrow(
			'every changed file was restored',
		);
		expect(await readFile(join(root, 'decisions.html'), 'utf8')).toBe(clashingPage);
		expect(await readFile(join(root, 'records/initiative-decisions.ts'), 'utf8')).toBe(decided);
		await writeFile(join(root, 'decisions.html'), page);

		await answer(join(root, 'clicked.md'), '--write');
		expect(await readFile(join(root, 'records/initiative-decisions.ts'), 'utf8')).toContain(
			"sourceKind: 'owner-accepted'",
		);
		const view = z
			.object({
				records: z.array(
					z.object({
						id: z.string(),
						authority: z.object({ chosen: z.string().optional() }).optional(),
					}),
				),
			})
			.parse(JSON.parse(await readFile(join(root, 'initiative.generated.json'), 'utf8')));
		expect(
			view.records
				.filter(({ id }) => id === 'initiative/D2' || id === 'initiative/D3')
				.map(({ authority }) => authority?.chosen),
		).toEqual(['resume', 'journal']);

		await expect(answer(join(root, 'worded.md'))).rejects.toThrow(
			'initiative/D2: already decided (owner-decided); nothing to apply.',
		);
	} finally {
		await rm(root, { recursive: true, force: true });
	}
}, 30_000);
