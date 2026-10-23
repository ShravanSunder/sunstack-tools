import { afterEach, expect, test } from 'vitest';
import { page } from 'vitest/browser';

import { openFixture, requiredElement } from './fixture-page.ts';

afterEach(async (): Promise<void> => {
	document.body.replaceChildren();
	await page.viewport(1600, 1000);
});

test('compact C timing retains the selected two-line presentation', async (): Promise<void> => {
	const { document: fixture } = await openFixture('ui', '', 'small-parts.html');
	const breakdown = requiredElement(fixture, '#share-breakdown');
	await expect.poll((): boolean => breakdown.hasAttribute('prepared')).toBe(true);
	const fixtureWindow = fixture.defaultView;
	if (!fixtureWindow) throw new Error('Fixture has no browser window.');
	const track = requiredElement(breakdown, '.ex-breakdown-track');

	// C's reference has one 16px bar, a 4px gap and one line of exact values.
	expect(breakdown.getBoundingClientRect().height).toBeLessThanOrEqual(44);
	expect(track.getBoundingClientRect().height).toBe(16);
	expect(fixtureWindow.getComputedStyle(track).borderRadius).toBe('4px');
	for (const segment of track.querySelectorAll('.ex-breakdown-segment'))
		expect(fixtureWindow.getComputedStyle(segment).borderRadius).toBe('0px');
	for (const repeatedText of breakdown.querySelectorAll('.ex-breakdown-label, .ex-breakdown-scale'))
		expect(repeatedText.getBoundingClientRect().height).toBe(0);
	expect(requiredElement(breakdown, '.ex-breakdown-values').textContent).toBe('28 / 156 / 44 ms');
	const accessibleDescription = track.getAttribute('aria-label');
	expect(accessibleDescription).toContain('share of total, 100 percent');
	for (const fact of ['Connect 28 ms', 'Work 156 ms', 'Transfer 44 ms'])
		expect(accessibleDescription).toContain(fact);

	const records = requiredElement(fixture, '#selected-c-records');
	expect(records.querySelectorAll('tbody tr')).toHaveLength(6);
	for (const fact of [
		'Asha Patel',
		'Noor Chen',
		'Luis Diaz',
		'North workspace',
		'West workspace',
		'A-148',
		'A-149',
		'A-150',
		'A-151',
		'A-152',
		'A-153',
	])
		expect(records.textContent).toContain(fact);
	for (const cell of records.querySelectorAll('td'))
		expect(fixtureWindow.getComputedStyle(cell).verticalAlign).toBe('middle');
	for (const bar of records.querySelectorAll('ex-breakdown'))
		expect(bar.getBoundingClientRect().height).toBeLessThanOrEqual(44);
});

test('compact evidence parts retain exact facts, scale and slot color in native contexts', async (): Promise<void> => {
	const { document: fixture } = await openFixture('ui', '', 'small-parts.html');
	await expect
		.poll((): boolean =>
			[
				'#long-neutral-badge',
				'#trace-neutral-badge',
				'#trace-planned-badge',
				'#complete-progress',
				'#zero-progress',
				'#partial-progress',
				'#unknown-progress',
				'#unknown-with-total',
				'#empty-progress',
				'#share-breakdown',
				'#reordered-breakdown',
				'#zero-share-breakdown',
				'#zero-elapsed-breakdown',
				'#trace-breakdown',
				'#fractional-elapsed-breakdown',
			].every((selector): boolean => requiredElement(fixture, selector).hasAttribute('prepared')),
		)
		.toBe(true);

	for (const [id, text] of [
		['#zero-progress', '0/6'],
		['#partial-progress', '2/6'],
		['#complete-progress', '6/6'],
	] as const)
		expect(requiredElement(fixture, id).textContent).toContain(text);
	for (const id of ['#unknown-progress', '#unknown-with-total']) {
		const progress = requiredElement(fixture, id);
		expect(progress.textContent).toContain('Not reported');
		expect(progress.querySelector('.ex-progress-gauge')).toBeNull();
	}
	const empty = requiredElement(fixture, '#empty-progress');
	expect(empty.textContent).toContain('0/0');
	expect(empty.textContent?.toLowerCase()).toContain('empty');
	expect(empty.querySelector('.ex-progress-gauge')).toBeNull();

	const partialFill = requiredElement(
		requiredElement(fixture, '#partial-progress'),
		'.ex-progress-fill',
	);
	const partialDashArray = Number(partialFill.getAttribute('stroke-dasharray'));
	const partialDashOffset = Number(partialFill.getAttribute('stroke-dashoffset'));
	expect(partialDashArray).toBeCloseTo(2 * Math.PI * 42, 0);
	expect(1 - partialDashOffset / partialDashArray).toBeCloseTo(2 / 6, 3);
	const emptyFill = requiredElement(
		requiredElement(fixture, '#zero-progress'),
		'.ex-progress-fill',
	);
	expect(
		Number(emptyFill.getAttribute('stroke-dashoffset')) /
			Number(emptyFill.getAttribute('stroke-dasharray')),
	).toBe(1);
	expect(
		Number(
			requiredElement(
				requiredElement(fixture, '#complete-progress'),
				'.ex-progress-fill',
			).getAttribute('stroke-dashoffset'),
		),
	).toBe(0);

	const share = requiredElement(fixture, '#share-breakdown');
	const shareTrack = requiredElement(share, '.ex-breakdown-track');
	const shareSegments = [...share.querySelectorAll<HTMLElement>('.ex-breakdown-segment')];
	const trackWidth = shareTrack.getBoundingClientRect().width;
	expect(shareSegments).toHaveLength(3);
	for (const [segment, expected] of [
		[shareSegments[0], 28 / 228],
		[shareSegments[1], 156 / 228],
		[shareSegments[2], 44 / 228],
	] as const) {
		if (!segment) throw new Error('Missing share segment.');
		expect(segment.getBoundingClientRect().width / trackWidth).toBeCloseTo(expected, 2);
	}
	expect(share.textContent).toContain('28 / 156 / 44 ms');
	expect(shareTrack.getAttribute('aria-label')).toContain('100 percent');

	const elapsed = requiredElement(fixture, '#reordered-breakdown');
	const elapsedTrack = requiredElement(elapsed, '.ex-breakdown-track');
	const elapsedSegments = [...elapsed.querySelectorAll<HTMLElement>('.ex-breakdown-segment')];
	expect(elapsed.textContent).toContain('44 / 28 / 156 ms');
	expect(elapsed.querySelector('.ex-breakdown-scale')?.textContent).toContain('228/1240 ms');
	expect(
		elapsedSegments.reduce(
			(total, segment): number => total + segment.getBoundingClientRect().width,
			0,
		) / elapsedTrack.getBoundingClientRect().width,
	).toBeCloseTo(228 / 1240, 2);

	const expectedColors = {
		light: ['rgb(61, 121, 174)', 'rgb(168, 120, 36)', 'rgb(43, 129, 117)'],
		dark: ['rgb(91, 155, 213)', 'rgb(216, 166, 87)', 'rgb(96, 181, 166)'],
	} as const;
	const expectedNeutralPaint = {
		light: { foreground: 'rgb(74, 85, 102)', background: 'rgb(255, 255, 255)' },
		dark: { foreground: 'rgb(165, 175, 189)', background: 'rgb(23, 29, 38)' },
	} as const;
	const expectedPlannedPaint = {
		light: { foreground: 'rgb(91, 102, 120)', background: 'rgb(236, 239, 243)' },
		dark: { foreground: 'rgb(168, 178, 194)', background: 'rgb(35, 43, 55)' },
	} as const;
	const fixtureWindow = fixture.defaultView;
	if (!fixtureWindow) throw new Error('Fixture has no browser window.');
	for (const theme of ['light', 'dark'] as const) {
		fixture.documentElement.dataset['theme'] = theme;
		const reorderedSeries = new Map(
			[...elapsed.querySelectorAll<HTMLElement>('.ex-breakdown-segment')].map(
				(segment): [string, string] => [
					segment.dataset['series'] ?? '',
					fixture.defaultView?.getComputedStyle(segment).backgroundColor ?? '',
				],
			),
		);
		const shareColors = [...share.querySelectorAll<HTMLElement>('.ex-breakdown-segment')].map(
			(segment): string => fixture.defaultView?.getComputedStyle(segment).backgroundColor ?? '',
		);
		const legendColors = [
			...requiredElement(fixture, '#prose-breakdown').querySelectorAll<HTMLElement>(
				'.ex-breakdown-marker',
			),
		].map((marker): string => fixture.defaultView?.getComputedStyle(marker).backgroundColor ?? '');
		expect(shareColors).toEqual(expectedColors[theme]);
		expect(legendColors).toEqual(expectedColors[theme]);
		expect(reorderedSeries.get('1')).toBe(expectedColors[theme][0]);
		expect(reorderedSeries.get('2')).toBe(expectedColors[theme][1]);
		expect(reorderedSeries.get('3')).toBe(expectedColors[theme][2]);

		const outsideNeutral = requiredElement(fixture, '#long-neutral-badge');
		const nestedNeutral = requiredElement(fixture, '#trace-neutral-badge');
		const nestedPlanned = requiredElement(fixture, '#trace-planned-badge');
		const outsideStyles = fixtureWindow.getComputedStyle(outsideNeutral);
		const nestedStyles = fixtureWindow.getComputedStyle(nestedNeutral);
		const outsideNeutralPaint = {
			foreground: outsideStyles.color,
			background: outsideStyles.backgroundColor,
			border: outsideStyles.borderTopColor,
			marker: fixtureWindow.getComputedStyle(outsideNeutral, '::before').backgroundColor,
		};
		const nestedNeutralPaint = {
			foreground: nestedStyles.color,
			background: nestedStyles.backgroundColor,
			border: nestedStyles.borderTopColor,
			marker: fixtureWindow.getComputedStyle(nestedNeutral, '::before').backgroundColor,
		};
		expect(outsideNeutralPaint).toMatchObject(expectedNeutralPaint[theme]);
		expect(nestedNeutralPaint).toEqual(outsideNeutralPaint);
		const plannedStyles = fixtureWindow.getComputedStyle(nestedPlanned);
		expect({
			foreground: plannedStyles.color,
			background: plannedStyles.backgroundColor,
		}).toEqual(expectedPlannedPaint[theme]);
	}

	const fractionalElapsed = requiredElement(fixture, '#fractional-elapsed-breakdown');
	const fractionalTrack = requiredElement(fractionalElapsed, '.ex-breakdown-track');
	const fractionalSegments = [
		...fractionalElapsed.querySelectorAll<HTMLElement>('.ex-breakdown-segment'),
	];
	expect(fractionalElapsed.querySelector('.ex-breakdown-unavailable')).toBeNull();
	expect(fractionalElapsed.querySelector('.ex-breakdown-scale')?.textContent).toBe(
		'Elapsed · 0.3/0.3 s',
	);
	expect(fractionalElapsed.textContent).toContain('0.1 s');
	expect(fractionalElapsed.textContent).toContain('0.2 s');
	expect(fractionalSegments).toHaveLength(2);
	expect(
		fractionalSegments.reduce(
			(width, segment): number => width + segment.getBoundingClientRect().width,
			0,
		) / fractionalTrack.getBoundingClientRect().width,
	).toBeCloseTo(1, 2);

	for (const id of ['#zero-share-breakdown', '#zero-elapsed-breakdown']) {
		const zero = requiredElement(fixture, id);
		expect(zero.querySelector('.ex-breakdown-track')).toBeNull();
		expect(zero.textContent).toContain('0 ms');
	}
	expect(requiredElement(fixture, '#zero-share-breakdown').textContent).toContain('Share');
	expect(requiredElement(fixture, '#zero-elapsed-breakdown').textContent).toContain('0/1240 ms');

	const fullProgress = requiredElement(fixture, '#complete-progress');
	const delayed = requiredElement(fixture, '#delayed-status');
	expect(fullProgress.hasAttribute('tone')).toBe(false);
	expect(delayed.getAttribute('tone')).toBe('risk');
	expect(fixture.querySelector('#trace-breakdown .ex-breakdown-track')).not.toBeNull();
	expect(requiredElement(fixture, '#compact-prose').textContent).toContain('Supplied observation');

	const inconsistent = fixture.createElement('ex-progress');
	inconsistent.setAttribute('label', 'Checks');
	inconsistent.setAttribute('value', '7');
	inconsistent.setAttribute('max', '6');
	fixture.querySelector('ex-document')?.append(inconsistent);
	await expect.poll((): boolean => inconsistent.hasAttribute('prepared')).toBe(true);
	expect(inconsistent.textContent).toContain('7/6');
	expect(inconsistent.textContent?.toLowerCase()).toContain('inconsistent');
	expect(inconsistent.querySelector('.ex-progress-gauge')).toBeNull();

	const invalidElapsed = fixture.createElement('ex-breakdown');
	invalidElapsed.setAttribute('label', 'Request phases');
	invalidElapsed.setAttribute('unit', 'ms');
	invalidElapsed.setAttribute('scale', 'elapsed');
	invalidElapsed.setAttribute('max', '0');
	const invalidPhase = fixture.createElement('data');
	invalidPhase.setAttribute('value', '0');
	invalidPhase.setAttribute('series', '1');
	invalidPhase.textContent = 'Connect';
	invalidElapsed.append(invalidPhase);
	fixture.querySelector('ex-document')?.append(invalidElapsed);
	await expect.poll((): boolean => invalidElapsed.hasAttribute('prepared')).toBe(true);
	expect(invalidElapsed.textContent).toContain('Connect');
	expect(invalidElapsed.textContent).toContain('0');
	expect(invalidElapsed.querySelector('.ex-breakdown-track')).toBeNull();

	await page.viewport(390, 1000);
	const badge = requiredElement(fixture, '#long-neutral-badge');
	expect(badge.getBoundingClientRect().right).toBeLessThanOrEqual(390);
	expect(badge.scrollWidth).toBeLessThanOrEqual(badge.clientWidth + 1);
	const table = requiredElement(fixture, '#evidence-table');
	expect(table.scrollWidth).toBeGreaterThan(table.clientWidth);
	expect(fixture.documentElement.scrollWidth).toBeLessThanOrEqual(390);
	const grid = requiredElement(fixture, '#nested-grid-control');
	expect(grid.getBoundingClientRect().right).toBeLessThanOrEqual(390);
});
