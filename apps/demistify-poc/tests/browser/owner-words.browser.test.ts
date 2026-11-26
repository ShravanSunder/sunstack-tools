import { afterEach, expect, test } from 'vitest';
import { page, userEvent } from 'vitest/browser';

import {
	readReviewTargets,
	resolveCommentAnchor,
} from '../../../../packages/demistify/src/core/review-anchors.ts';
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
		expect(
			view.getComputedStyle(requiredElement(first, '.ex-choice-summary')).marginBlockStart,
		).toBe('8px');
		expect(
			view.getComputedStyle(requiredElement(second, '.ex-choice-summary')).marginBlockStart,
		).toBe('4px');
		expect(requiredElement(choice, 'table').getBoundingClientRect().left).toBeCloseTo(
			choice.getBoundingClientRect().left + 16,
		);
		expect(fixture.documentElement.scrollWidth).toBeLessThanOrEqual(width);

		const settledChoice = requiredElement(fixture, '#D2');
		const wrapper = requiredElement(fixture, 'ex-section-item[record="D2"] > .ex-record-surface');
		expect(view.getComputedStyle(wrapper).display).toBe('contents');
		const chosen = requiredElement(settledChoice, '.ex-choice-detail-options > .ex-record-chosen');
		const alternative = requiredElement(settledChoice, '.ex-record-alternative');
		const settledContext = view.getComputedStyle(settledChoice, '::before');
		expect(settledContext.backgroundColor).toBe(context.backgroundColor);
		expect(chosen.getBoundingClientRect().width).toBeCloseTo(
			Number.parseFloat(settledContext.width),
		);
		expect(view.getComputedStyle(chosen).borderRadius).toBe('8px');
		expect(view.getComputedStyle(alternative).borderRadius).toBe('8px');
		expect(
			view.getComputedStyle(requiredElement(chosen, '.ex-choice-summary')).marginBlockStart,
		).toBe('8px');
		expect(requiredElement(chosen, 'ex-badge').textContent).toBe('Chosen');
		expect(requiredElement(alternative, 'ex-badge').textContent).toBe('Not chosen');
	},
);

test('legacy title selections still resolve to the visible question and statement', (): void => {
	const question = 'How should a slow request be retried?';
	const statement = 'Limit retries to three attempts.';
	const root = document.createElement('ex-document');
	root.id = 'previous-record-layout';
	root.innerHTML = `<ex-section-item record="D5">
		<ex-choice id="D5"><fieldset><legend data-record-display-title>Retry policy</legend><div>D5</div><div data-record-statement>${question}</div></fieldset></ex-choice>
	</ex-section-item><ex-section-item record="D1" id="D1">
		<article class="ex-record-surface"><h4 data-record-display-title>Retry limit</h4><div>D1</div><div data-record-statement>${statement}</div><p>Apply the limit on every retry.</p></article>
	</ex-section-item>`;
	document.body.append(root);
	const targets = readReviewTargets(root);
	const ruling = requiredElement(root, '#D1 h4');
	const anchor = {
		kind: 'text',
		targetId: 'D5',
		label: 'Retry policy',
		selectedText: question,
		sourceText: question,
		start: 0,
		end: question.length,
		prefix: '',
		suffix: 'D5',
	} as const;
	const choice = resolveCommentAnchor(anchor, targets);
	expect(choice.kind === 'resolved' && choice.range?.toString()).toBe(question);
	const priorRuling = resolveCommentAnchor(
		{
			...anchor,
			targetId: ruling.id,
			label: statement,
			selectedText: statement,
			sourceText: statement,
			end: statement.length,
			suffix: '',
		},
		targets,
	);
	expect(priorRuling.kind === 'resolved' && priorRuling.range?.toString()).toBe(statement);
});
