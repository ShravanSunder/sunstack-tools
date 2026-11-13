import { afterEach, expect, test } from 'vitest';
import { page, userEvent } from 'vitest/browser';

import { browserScreenshotPath, writeBrowserEvidence } from './browser-evidence.ts';
import { fixtureLocator, openFixture, requiredElement } from './fixture-page.ts';
afterEach(async (): Promise<void> => {
	document.body.replaceChildren();
	await page.viewport(1600, 1000);
});
test.each([1600, 390].flatMap((width) => ['light', 'dark'].map((theme) => ({ width, theme }))))(
	'call reader inspector keeps generated Help state: $width $theme',
	async ({ width, theme }): Promise<void> => {
		await page.viewport(width, width === 390 ? 844 : 1000);
		const { document: fixture, frame } = await openFixture(
			'document-browsers',
			'',
			'public-style/inspector-calls.html',
		);
		fixture.documentElement.dataset['theme'] = theme;
		await expect.poll(() => fixture.querySelector('ex-call-stack[data-ready]')).not.toBeNull();
		const reader = requiredElement(fixture, 'ex-call-stack');
		const help = reader.querySelector<HTMLDetailsElement>('details.ex-design-help');
		if (!help) throw new Error('Missing generated call Help.');
		expect(help.open).toBe(false);
		await fixtureLocator(
			frame,
			requiredElement(reader, ':scope > ex-design-call-view > p'),
		).hover();
		await frame.getByRole('button', { name: 'Expand Call reader', exact: true }).click();
		expect(requiredElement(fixture, '.ex-inspect-dialog ex-call-stack')).toBe(reader);
		expect(help.open).toBe(false);
		await page.screenshot({ path: browserScreenshotPath(`call-inspector-${width}-${theme}.png`) });
		await fixtureLocator(frame, requiredElement(help, 'summary')).click();
		expect(help.open).toBe(true);
		await fixtureLocator(frame, requiredElement(help, 'summary')).click();
		expect(help.open).toBe(false);
		await frame
			.getByRole('dialog', { name: 'Call reader', exact: true })
			.getByRole('button', { name: 'Close', exact: true })
			.click();
		expect(help.open).toBe(false);
		expect(fixture.querySelector('ex-call-stack')).toBe(reader);
	},
);

async function readerRevision(
	file: string,
	outcome: 'loaded' | 'failed' = 'loaded',
): Promise<string> {
	document.body.replaceChildren();
	const { document: fixture, frame } = await openFixture('document-browsers', '', file);
	await expect
		.poll(() => fixture.querySelector('ex-call-stack[data-ready],ex-call-stack [role="alert"]'))
		.not.toBeNull();
	if (outcome === 'failed') {
		expect(fixture.querySelector('ex-call-stack[data-ready]')).toBeNull();
		expect(fixture.querySelector('ex-call-stack [role="alert"]')).not.toBeNull();
	}
	await fixtureLocator(frame, requiredElement(fixture, '.ex-comments-trigger')).click();
	const root = requiredElement(fixture, 'ex-document');
	await expect.poll(() => root.getAttribute('revision')).toMatch(/^sha256:/);
	expect(fixture.body.textContent).not.toContain('source load was not registered');
	return root.getAttribute('revision') ?? '';
}

test('Comments revisions follow whole out-of-root bodies and both local Markdown revisions', async (): Promise<void> => {
	const original = await readerRevision('revision-base/requirements.html');
	const unchanged = await readerRevision('revision-base/requirements.html');
	const bodyChanged = await readerRevision('revision-body/requirements.html');
	const markdownFirst = await readerRevision('revision-md-first/requirements.html');
	const markdownSecond = await readerRevision('revision-md-second/requirements.html');
	expect(unchanged).toBe(original);
	expect(new Set([original, bodyChanged, markdownFirst, markdownSecond]).size).toBe(4);
});

test.each(['unprepared.html', 'invalid.html', 'empty.html'])(
	'failed input keeps Comments usable: %s',
	async (file): Promise<void> => {
		const revision = await readerRevision(file, 'failed');
		expect(revision).toMatch(/^sha256:/);
	},
);

test('a standalone record body renders its prepared Markdown and diagram', async (): Promise<void> => {
	const { document: fixture } = await openFixture(
		'document-browsers',
		'',
		'full/program-design.html?need=DraftNeed',
	);
	await expect.poll(() => fixture.querySelector('ex-markdown strong')?.textContent).toBe('editor');
	await expect
		.poll(() => fixture.querySelector('ex-mermaid svg')?.textContent)
		.toContain('Refusal');
	expect(requiredElement(fixture, 'ex-markdown').textContent).toContain('draft-before-save');
});

test.each([1600, 390].flatMap((width) => ['light', 'dark'].map((theme) => ({ width, theme }))))(
	'standalone calls retain selection and Comments: $width $theme',
	async ({ width, theme }): Promise<void> => {
		await page.viewport(width, width === 390 ? 844 : 1000);
		const { document: fixture, frame } = await openFixture('understanding', '', 'execution.html');
		fixture.documentElement.dataset['theme'] = theme;
		await expect.poll(() => fixture.querySelector('ex-call-stack[data-ready]')).not.toBeNull();
		const owner = requiredElement(fixture, 'ex-call-stack');
		const call = requiredElement(owner, 'button[data-frame="0"]');
		await fixtureLocator(frame, call).click();
		if (width === 390)
			await frame.getByRole('button', { name: 'Close call details', exact: true }).click();
		await userEvent.keyboard('{ArrowDown}');
		expect(requiredElement(owner, 'button[data-frame="1"]').getAttribute('aria-pressed')).toBe(
			'true',
		);
		expect(new URL(fixture.URL).searchParams.get('call')).toBe('input-owner-route');
		await fixtureLocator(
			frame,
			requiredElement(owner, '[data-call-index="1"] [data-comment-control]'),
		).click();
		await frame
			.getByRole('textbox', { name: 'Your comment', exact: true })
			.fill('Inspect this routing boundary.');
		await Promise.all(
			fixture.getAnimations().map((animation) => animation.finished.catch(() => undefined)),
		);
		await page.screenshot({ path: browserScreenshotPath(`call-comments-${width}-${theme}.png`) });
		expect(fixture.activeElement?.getAttribute('aria-label')).toBe('Your comment');
		const addressWhileTyping = fixture.URL;
		await userEvent.keyboard('{ArrowDown}');
		expect(fixture.URL).toBe(addressWhileTyping);
		expect(requiredElement(owner, 'button[data-frame="1"]').getAttribute('aria-pressed')).toBe(
			'true',
		);
		await frame.getByRole('button', { name: 'Close comments', exact: true }).click();
		expect(requiredElement(owner, 'button[data-frame="1"]').getAttribute('aria-pressed')).toBe(
			'true',
		);
		expect(fixture.documentElement.scrollWidth).toBeLessThanOrEqual(width);
		await writeBrowserEvidence(
			`retained-calls-${width}-${theme}.json`,
			JSON.stringify({
				width,
				theme,
				address: fixture.URL,
				selected: owner.querySelector('[aria-pressed="true"]')?.textContent,
				pageWidth: fixture.documentElement.scrollWidth,
			}),
		);
		await page.screenshot({ path: browserScreenshotPath(`retained-calls-${width}-${theme}.png`) });
	},
);

test('standalone call labels use the authored accessible name', async (): Promise<void> => {
	const { document: fixture } = await openFixture('document-browsers', '', 'labeled.html');
	await expect.poll(() => fixture.querySelector('ex-call-stack[data-ready]')).not.toBeNull();
	expect(requiredElement(fixture, 'ex-call-stack [role="group"]').getAttribute('aria-label')).toBe(
		'Draft source calls',
	);
});

test('call reconnect preserves the selected frame and Comments source revision', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('understanding', '', 'execution.html');
	await expect.poll(() => fixture.querySelector('ex-call-stack[data-ready]')).not.toBeNull();
	const owner = requiredElement(fixture, 'ex-call-stack');
	const view = requiredElement(owner, 'ex-design-call-view');
	const root = requiredElement(fixture, 'ex-document');
	await fixtureLocator(frame, requiredElement(fixture, '.ex-comments-trigger')).click();
	await expect.poll(() => root.getAttribute('revision')).toMatch(/^sha256:/);
	const revision = root.getAttribute('revision');
	await frame.getByRole('button', { name: 'Close comments', exact: true }).click();
	await fixtureLocator(frame, requiredElement(owner, 'button[data-frame="1"]')).click();
	owner.remove();
	root.append(owner);
	expect(requiredElement(owner, 'ex-design-call-view')).toBe(view);
	expect(requiredElement(owner, 'button[data-frame="1"]').getAttribute('aria-pressed')).toBe(
		'true',
	);
	expect(new URL(fixture.URL).searchParams.get('call')).toBe('input-owner-route');
	await fixtureLocator(frame, requiredElement(fixture, '.ex-comments-trigger')).click();
	await expect
		.element(frame.getByRole('button', { name: 'Close comments', exact: true }))
		.toBeVisible();
	expect(root.getAttribute('revision')).toBe(revision);
	expect(fixture.body.textContent).not.toContain('source load was not registered');
	expect(fixture.querySelector('[role="alert"]')).toBeNull();
});
