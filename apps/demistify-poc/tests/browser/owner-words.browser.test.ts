import { afterEach, expect, test } from 'vitest';
import { page, userEvent } from 'vitest/browser';

import { browserScreenshotPath } from './browser-evidence.ts';
import { fixtureLocator, openFixture, requiredElement } from './fixture-page.ts';

afterEach(async (): Promise<void> => {
	document.body.replaceChildren();
	await page.viewport(1600, 1000);
});

test.each([1400, 390].flatMap((width) => ['light', 'dark'].map((theme) => ({ width, theme }))))(
	'owner wording shows its attribution and exact source at $width px in $theme',
	async ({ width, theme }): Promise<void> => {
		await page.viewport(width, 1000);
		const { document: fixture, frame } = await openFixture('ui', '', 'owner-words.html');
		fixture.documentElement.dataset['theme'] = theme;

		async function verifyDisclosure(
			selector: string,
			label: string,
			shown: string,
			excerpt: string,
			href: string,
		): Promise<void> {
			const record = requiredElement(fixture, selector);
			const face = fixtureLocator(frame, record);
			await expect.element(face.getByText(label, { exact: true })).toBeVisible();
			const link = face.getByRole('link', { name: shown, exact: true });
			await expect.element(link).toHaveAttribute('href', href);

			const disclosure = requiredElement(record, 'details:has(> summary)');
			const summaryElement = requiredElement(disclosure, 'summary');
			const summary = fixtureLocator(frame, summaryElement);
			const exactWords = requiredElement(disclosure, 'p');
			expect(summaryElement.textContent).toBe('Your exact words');
			expect(exactWords.textContent).toBe(excerpt);
			expect(disclosure.hasAttribute('open')).toBe(false);

			record.querySelector<HTMLElement>(`a[href="${href}"]`)?.focus();
			await expect.element(link).toHaveFocus();
			await userEvent.keyboard('{Tab}');
			if (fixture.activeElement?.hasAttribute('data-comment-control'))
				await userEvent.keyboard('{Tab}');
			await expect.element(summary).toHaveFocus();
			await userEvent.keyboard('{Enter}');
			await expect.element(fixtureLocator(frame, disclosure)).toHaveAttribute('open', '');
			await expect.element(fixtureLocator(frame, exactWords)).toBeVisible();
			expect(fixture.documentElement.scrollWidth).toBeLessThanOrEqual(width);
			disclosure.scrollIntoView({ block: 'center', behavior: 'instant' });
			await page.screenshot({
				path: browserScreenshotPath(`owner-words-${selector.slice(1)}-${width}-${theme}.png`),
			});

			await userEvent.keyboard(' ');
			expect(disclosure.hasAttribute('open')).toBe(false);
		}

		await verifyDisclosure(
			'#D1',
			'Your words, tidied',
			'Allow at most three retries, please.',
			'3  retrys max\npls',
			'#retry-source',
		);
		await expect.element(frame.getByText('Which queue do we use?', { exact: true })).toBeVisible();
		await verifyDisclosure(
			'#D2',
			'Wording you accepted, tidied',
			'Keep the same queue.',
			'keep the same queu',
			'#queue-source',
		);
		expect(requiredElement(fixture, '#D2--details').closest('#D2')?.id).toBe('D2');

		const originalChoice = requiredElement(fixture, '#D3--details');
		expect(originalChoice.localName).toBe('p');
		expect(originalChoice.textContent).toBe('Wording you accepted your answer');
		expect(originalChoice.querySelector('a')?.getAttribute('href')).toBe('#history-source');
		expect(fixture.querySelector('#D3 summary')).toBeNull();

		const originalRuling = requiredElement(fixture, '#D4 strong');
		expect(originalRuling.parentElement?.localName).toBe('p');
		expect(originalRuling.parentElement?.textContent).toBe('Your words one reciept is fine');
		expect(fixture.querySelector('#D4 summary')).toBeNull();
	},
);

test.each([
	{ width: 1400, theme: 'dark' },
	{ width: 390, theme: 'light' },
])(
	'typed records share one reading edge at $width px in $theme',
	async ({ width, theme }): Promise<void> => {
		await page.viewport(width, 1000);
		const { document: fixture, frame } = await openFixture('ui', '', 'owner-words.html');
		fixture.documentElement.dataset['theme'] = theme;
		await expect.element(frame.getByText('Retry policy', { exact: true })).toBeVisible();

		const view = fixture.defaultView;
		if (!view) throw new Error('Missing preview window.');
		const openSection = requiredElement(fixture, '#open-decisions');
		const heading = requiredElement(openSection, '.ex-section-head');
		requiredElement(heading, '.ex-tone-icon');
		const choice = requiredElement(openSection, '#D5');
		const item = requiredElement(openSection, 'ex-section-item[record="D5"]');
		expect(view.getComputedStyle(heading, '::after').backgroundColor).toBe(
			view.getComputedStyle(item, '::before').backgroundColor,
		);
		const context = view.getComputedStyle(choice, '::before');
		const ruling = requiredElement(fixture, '#D1 > .ex-record-surface');
		expect(context.backgroundColor).toBe(view.getComputedStyle(ruling).backgroundColor);
		expect(requiredElement(choice, 'fieldset').getAttribute('aria-label')).toBe(
			'How should a slow request be retried?',
		);

		const layout = requiredElement(choice, '.ex-choice-detail-layout');
		const spacing = view.getComputedStyle(layout);
		expect([spacing.marginBlockStart, spacing.marginBlockEnd]).toEqual(['36px', '12px']);
		const [first, second] = choice.querySelectorAll('ex-option.ex-choice-option');
		if (!first || !second) throw new Error('Expected two open options.');
		expect(first.getBoundingClientRect().width).toBeCloseTo(Number.parseFloat(context.width));
		expect(second.getBoundingClientRect().top - first.getBoundingClientRect().bottom).toBeCloseTo(
			12,
		);
		expect(requiredElement(first, '.ex-record-option-metadata').getBoundingClientRect().left).toBe(
			requiredElement(first, '.ex-choice-label').getBoundingClientRect().left,
		);
		expect(first.querySelector('.ex-choice-details-button')).toBeNull();
		expect(requiredElement(choice, 'table').getBoundingClientRect().left).toBeCloseTo(
			choice.getBoundingClientRect().left + 16,
		);
		expect(fixture.documentElement.scrollWidth).toBeLessThanOrEqual(width);

		const chosen = requiredElement(fixture, '#D2 .ex-choice-detail-options > .ex-record-chosen');
		const settled = view.getComputedStyle(chosen);
		expect(settled.backgroundColor).toBe('rgba(0, 0, 0, 0)');
		expect(settled.borderInlineStartWidth).toBe('0px');
	},
);
