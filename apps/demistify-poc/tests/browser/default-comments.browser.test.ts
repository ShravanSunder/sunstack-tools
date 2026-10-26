import { afterEach, expect, test } from 'vitest';
import { page } from 'vitest/browser';
import * as Zod from 'zod';

import { fixtureLocator, openFixture, requiredElement } from './fixture-page.ts';
import { readCurrentReview } from './review-fixture.ts';

const sourceStatusSchema = Zod.object({
	markdown: Zod.number(),
	graph: Zod.number(),
	markdownSent: Zod.boolean(),
	graphSent: Zod.boolean(),
});
const heldCases: string[] = [];
async function sourceControl(props: {
	readonly caseId: string;
	readonly action: string;
	readonly mode?: string;
	readonly variant?: string;
}): Promise<Zod.infer<typeof sourceStatusSchema>> {
	const response = await fetch(
		`/__demistify/default-comments/control?${new URLSearchParams({ case: props.caseId, action: props.action, mode: props.mode ?? '', variant: props.variant ?? '' })}`,
	);
	if (!response.ok) throw new Error(`Source control failed: ${response.status}`);
	return sourceStatusSchema.parse(await response.json());
}
async function prepareHeld(mode = '', variant = '', existingCase?: string): Promise<string> {
	const caseId = existingCase ?? crypto.randomUUID();
	heldCases.push(caseId);
	await sourceControl({ caseId, action: 'prepare', mode, variant });
	return caseId;
}
async function openHeld(caseId: string): ReturnType<typeof openFixture> {
	const opened = await openFixture('default-comments', '', `${caseId}/index.html`);
	await expect
		.poll(async (): Promise<number> => {
			const status = await sourceControl({ caseId, action: 'status' });
			return status.markdown + status.graph;
		})
		.toBe(2);
	return opened;
}
async function completeHeld(caseId: string, fixture: Document): Promise<string> {
	await sourceControl({ caseId, action: 'release' });
	await expect.poll((): number => fixture.querySelectorAll('.ex-comments-trigger').length).toBe(1);
	const status = await sourceControl({ caseId, action: 'status' });
	expect(status.markdown).toBe(1);
	expect(status.graph).toBe(1);
	return readCurrentReview(fixture).documentRevision;
}

afterEach(async (): Promise<void> => {
	await Promise.all(
		heldCases
			.splice(0)
			.map((caseId): ReturnType<typeof sourceControl> =>
				sourceControl({ caseId, action: 'release' }),
			),
	);
	document.body.replaceChildren();
	await page.viewport(1600, 1000);
});

test('fixed initial source inventory waits for both HTTP loads in either order, fingerprints payload changes and fetches only once', async (): Promise<void> => {
	const firstCase = await prepareHeld();
	const first = await openHeld(firstCase);
	await sourceControl({ caseId: firstCase, action: 'markdown' });
	await expect
		.poll((): boolean => !!first.document.querySelector('ex-markdown[prepared]'))
		.toBe(true);
	expect(first.document.querySelector('ex-document')?.hasAttribute('revision')).toBe(false);
	const baseline = await completeHeld(firstCase, first.document);
	document.body.replaceChildren();
	const secondCase = await prepareHeld();
	const second = await openHeld(secondCase);
	await sourceControl({ caseId: secondCase, action: 'graph' });
	await expect
		.poll((): boolean => !!second.document.querySelector('.ex-explorer-shell'))
		.toBe(true);
	expect(second.document.querySelector('ex-document')?.hasAttribute('revision')).toBe(false);
	expect(await completeHeld(secondCase, second.document)).toBe(baseline);
});

test.each(['markup', 'markdown', 'graph'])(
	'changed %s content changes revision while other inputs stay fixed',
	async (variant): Promise<void> => {
		const baselineCase = await prepareHeld();
		const baseline = await openHeld(baselineCase);
		const originalRevision = await completeHeld(baselineCase, baseline.document);
		document.body.replaceChildren();
		const changedCase = await prepareHeld('', variant);
		const changed = await openHeld(changedCase);
		expect(await completeHeld(changedCase, changed.document)).not.toBe(originalRevision);
		expect(changed.document.querySelector('ex-explorer .ex-entity-comment')).toBeNull();
	},
);

test('failed initial source and recovery on the same address use separate content revisions', async (): Promise<void> => {
	const caseId = await prepareHeld('', 'failed');
	const failed = await openHeld(caseId);
	const failedRevision = await completeHeld(caseId, failed.document);
	expect(requiredElement(failed.document, 'ex-markdown[role="alert"]').textContent).toContain(
		'HTTP 503',
	);
	const documentId = readCurrentReview(failed.document).documentId;
	document.body.replaceChildren();
	await prepareHeld('', '', caseId);
	const recovered = await openHeld(caseId);
	expect(await completeHeld(caseId, recovered.document)).not.toBe(failedRevision);
	expect(readCurrentReview(recovered.document).documentId).toBe(documentId);
});

test.each(['explicit', 'identity', 'revision'])(
	'author metadata stays authoritative: %s',
	async (mode): Promise<void> => {
		const caseId = await prepareHeld(mode);
		const { document: fixture } = await openHeld(caseId);
		await sourceControl({ caseId, action: 'release' });
		await expect.poll((): boolean => !!fixture.querySelector('ex-markdown[prepared]')).toBe(true);
		await expect
			.poll((): number => fixture.querySelectorAll('.ex-comments-trigger').length)
			.toBe(1);
		const review = readCurrentReview(fixture);
		expect(review.documentId).toBe(
			mode === 'revision'
				? `${location.origin}/__demistify/default-comments/${caseId}/index.html`
				: 'authored-page',
		);
		if (mode !== 'identity') expect(review.documentRevision).toBe('authored-r1');
		else expect(review.documentRevision).toMatch(/^sha256:[a-f0-9]{64}$/);
	},
);

test.each(['off', 'manual-off'])(
	'the removed disabling flag cannot suppress Comments: %s',
	async (mode): Promise<void> => {
		const caseId = await prepareHeld(mode);
		const { document: fixture } = await openHeld(caseId);
		await completeHeld(caseId, fixture);
		expect(fixture.querySelectorAll('.ex-comments-trigger')).toHaveLength(1);
		expect(fixture.querySelectorAll('h1 > .ex-entity-comment')).toHaveLength(1);
	},
);

test.each(['remove', 'reinsert', 'double'])(
	'pending identity does not install stale or duplicate controllers: %s',
	async (mode): Promise<void> => {
		const caseId = await prepareHeld(mode === 'double' ? mode : '');
		const { document: fixture } = await openHeld(caseId);
		const comments = requiredElement(fixture, 'ex-comments');
		if (mode !== 'double') {
			comments.remove();
			if (mode === 'reinsert') fixture.body.append(comments);
		}
		await sourceControl({ caseId, action: 'release' });
		await expect
			.poll(
				(): boolean =>
					!!fixture.querySelector('ex-markdown[prepared]') &&
					!!fixture.querySelector('.ex-explorer-shell'),
			)
			.toBe(true);
		if (mode === 'remove') {
			// Identity still settles, but the detached connection must install no UI.
			await expect.poll((): boolean => !!fixture.querySelector('ex-document[revision]')).toBe(true);
			expect(fixture.querySelector('.ex-comments-trigger,.ex-entity-comment')).toBeNull();
		} else {
			await expect
				.poll((): number => fixture.querySelectorAll('.ex-comments-trigger').length)
				.toBe(1);
			expect(
				requiredElement(fixture, 'ex-document > p').querySelectorAll(':scope > .ex-entity-comment'),
			).toHaveLength(1);
			if (mode === 'double')
				expect(requiredElement(fixture, 'ex-comments [role="alert"]').textContent).toContain(
					'only one Comments surface',
				);
		}
	},
);

test('an ordinary document gets one closed Comments surface and semantic targets without author boilerplate', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('review', '', 'defaults.html');
	await expect.poll((): number => fixture.querySelectorAll('.ex-comments-trigger').length).toBe(1);
	const root = requiredElement(fixture, 'ex-document');
	expect(requiredElement(fixture, '.ex-comments-trigger').getAttribute('aria-expanded')).toBe(
		'false',
	);
	expect(root.getAttribute('document-id')).toBe(
		`${location.origin}/__demistify/review/defaults.html`,
	);
	expect(root.getAttribute('revision')).toMatch(/^sha256:[a-f0-9]{64}$/);
	await expect.poll((): number => root.querySelectorAll('ex-markdown p').length).toBe(1);
	await Promise.all(
		[
			'h1',
			'h2',
			':scope > p',
			':scope > ul > li',
			'ex-callout',
			'ex-card',
			'ex-tree',
			'ex-markdown p',
		].map(async (selector): Promise<void> => {
			const target = requiredElement(root, selector);
			await expect
				.poll((): number => target.querySelectorAll(':scope > .ex-entity-comment').length)
				.toBe(1);
			expect(target.id).not.toBe('');
		}),
	);
	const passage = requiredElement(root, ':scope > p');
	await fixtureLocator(frame, passage).hover();
	await fixtureLocator(frame, requiredElement(passage, ':scope > .ex-entity-comment')).click();
	await frame
		.getByRole('textbox', { name: 'Your comment', exact: true })
		.fill('Keep this exact handoff.');
	await frame.getByRole('button', { name: 'Add comment', exact: true }).click();
	const saved = readCurrentReview(fixture);
	expect(saved.comments[0]?.anchor.targetId).toBe(passage.id);
	document.body.replaceChildren();
	const reopened = await openFixture('review', '?reader=canvas#ready-for-review', 'defaults.html');
	await expect
		.poll((): number => reopened.document.querySelectorAll('.ex-comments-trigger').length)
		.toBe(1);
	expect(readCurrentReview(reopened.document)).toEqual(saved);
	expect(requiredElement(reopened.document, 'ex-document > p').id).toBe(passage.id);
});

test.each([1600, 390])(
	'automatic controls preserve heading names and readable target gutters at width %i',
	async (width): Promise<void> => {
		await page.viewport(width, 1000);
		const { document: fixture, frame } = await openFixture('review', '', 'defaults.html');
		await expect
			.poll((): number => fixture.querySelectorAll('.ex-comments-trigger').length)
			.toBe(1);
		await expect
			.element(frame.getByRole('heading', { name: 'Review the pilot', exact: true }))
			.toBeVisible();
		const root = requiredElement(fixture, 'ex-document');
		expect(root.querySelector(':scope > .ex-entity-comment')).toBeNull();
		expect(fixture.documentElement.scrollWidth).toBeLessThanOrEqual(width + 1);
		const paragraph = requiredElement(root, ':scope > p');
		const text = paragraph.firstChild;
		if (!text || text.nodeType !== Node.TEXT_NODE)
			throw new Error('Expected authored paragraph text.');
		const range = fixture.createRange();
		range.selectNodeContents(text);
		const button = requiredElement(paragraph, ':scope > .ex-entity-comment');
		for (const rectangle of range.getClientRects())
			expect(rectangle.right).toBeLessThanOrEqual(button.getBoundingClientRect().left + 1);
		await fixtureLocator(frame, requiredElement(root, 'h1')).hover();
		await fixtureLocator(frame, requiredElement(root, 'h1 > .ex-entity-comment')).click();
		await expect
			.element(frame.getByRole('textbox', { name: 'Your comment', exact: true }))
			.toBeVisible();
		await frame.getByRole('button', { name: 'Close comments', exact: true }).click();
	},
);

test('an authored named target inside an interactive island survives generated-UI exclusion', async (): Promise<void> => {
	const caseId = await prepareHeld('named-intro');
	const { document: fixture, frame } = await openHeld(caseId);
	await completeHeld(caseId, fixture);
	const intro = requiredElement(fixture, '#graph-intro');
	expect(intro.querySelectorAll(':scope > .ex-entity-comment')).toHaveLength(1);
	expect(fixture.querySelector('ex-explorer h1 .ex-entity-comment')).toBeNull();
	await fixtureLocator(frame, requiredElement(intro, ':scope > .ex-entity-comment')).click();
	await frame.getByRole('textbox', { name: 'Your comment', exact: true }).fill('Graph is checked.');
	await frame.getByRole('button', { name: 'Add comment', exact: true }).click();
	expect(readCurrentReview(fixture).comments[0]?.anchor).toEqual({
		kind: 'entity',
		targetId: 'graph-intro',
		label: 'Graph introduction',
	});
});
