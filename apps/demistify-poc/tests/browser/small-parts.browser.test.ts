import { afterEach, expect, test } from 'vitest';
import { page } from 'vitest/browser';

import { openFixture, requiredElement } from './fixture-page.ts';

function resolveThemeColor(fixture: Document, token: string): string {
	const fixtureWindow = fixture.defaultView;
	if (!fixtureWindow) throw new Error('Fixture has no browser window.');
	const probe = fixture.createElement('span');
	probe.style.color = `var(${token})`;
	fixture.body.append(probe);
	const color = fixtureWindow.getComputedStyle(probe).color;
	probe.remove();
	return color;
}

function resolveThemeBackground(fixture: Document, token: string): string {
	const fixtureWindow = fixture.defaultView;
	if (!fixtureWindow) throw new Error('Fixture has no browser window.');
	const probe = fixture.createElement('span');
	probe.style.backgroundColor = `var(${token})`;
	fixture.body.append(probe);
	const background = fixtureWindow.getComputedStyle(probe).backgroundColor;
	probe.remove();
	return background;
}

afterEach(async (): Promise<void> => {
	document.body.replaceChildren();
	await page.viewport(1600, 1000);
});

test('progress count is centered inside its actual ring and fits a long value', async (): Promise<void> => {
	const { document: fixture } = await openFixture('ui', '', 'small-parts.html');
	const progress = requiredElement(fixture, '#partial-progress');
	await expect.poll((): boolean => progress.hasAttribute('prepared')).toBe(true);
	const gauge = requiredElement(progress, '.ex-progress-gauge');
	const count = requiredElement(progress, '.ex-progress-count');
	const gaugeBounds = gauge.getBoundingClientRect();
	const countBounds = count.getBoundingClientRect();
	expect(
		Math.abs(countBounds.left + countBounds.width / 2 - (gaugeBounds.left + gaugeBounds.width / 2)),
	).toBeLessThanOrEqual(1);
	expect(
		Math.abs(countBounds.top + countBounds.height / 2 - (gaugeBounds.top + gaugeBounds.height / 2)),
	).toBeLessThanOrEqual(1);
	expect(gaugeBounds.width).toBeGreaterThanOrEqual(40);
	expect(gaugeBounds.height).toBeGreaterThanOrEqual(40);
	const subject = requiredElement(progress, '.ex-progress-label');
	expect(subject.getBoundingClientRect().left).toBeGreaterThanOrEqual(gaugeBounds.right - 1);

	const largeProgress = requiredElement(fixture, '#large-count-progress');
	await expect.poll((): boolean => largeProgress.hasAttribute('prepared')).toBe(true);
	const largeGauge = requiredElement(largeProgress, '.ex-progress-gauge').getBoundingClientRect();
	const largeCount = requiredElement(largeProgress, '.ex-progress-count');
	const largeCountBounds = largeCount.getBoundingClientRect();
	expect(largeCount.textContent).toBe('1000/1000');
	expect(largeGauge.width).toBeGreaterThan(40);
	expect(largeGauge.height).toBeGreaterThanOrEqual(40);
	expect(largeCountBounds.left).toBeGreaterThanOrEqual(largeGauge.left);
	expect(largeCountBounds.right).toBeLessThanOrEqual(largeGauge.right);
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

	const records = requiredElement(fixture, '#selected-combined-records');
	const rows = [...records.querySelectorAll<HTMLTableRowElement>('tbody tr')];
	expect(rows).toHaveLength(6);
	await expect
		.poll((): boolean =>
			[...records.querySelectorAll('ex-progress, ex-breakdown')].every((part) =>
				part.hasAttribute('prepared'),
			),
		)
		.toBe(true);
	const rowFacts = [
		{
			id: 'assessment-a148',
			facts: [
				'Asha Patel',
				'North workspace',
				'Document import',
				'A-148',
				'Complete',
				'6/6',
				'228 ms',
				'28 / 156 / 44 ms',
			],
			initials: 'AP',
		},
		{
			id: 'assessment-a149',
			facts: [
				'Noor Chen',
				'North workspace',
				'Address assessment',
				'A-149',
				'Running',
				'2/6',
				'371 ms',
				'42 / 264 / 65 ms',
			],
			initials: 'NC',
		},
		{
			id: 'assessment-a150',
			facts: [
				'Luis Diaz',
				'West workspace',
				'Photo review',
				'A-150',
				'Failed',
				'1/6',
				'512 ms',
				'34 / 418 / 60 ms',
			],
			initials: 'LD',
		},
		{
			id: 'assessment-a151',
			facts: [
				'Asha Patel',
				'North workspace',
				'Identity lookup',
				'A-151',
				'Complete',
				'6/6',
				'96 ms',
				'18 / 55 / 23 ms',
			],
			initials: 'AP',
		},
		{
			id: 'assessment-a152',
			facts: [
				'Noor Chen',
				'West workspace',
				'Delivery reconciliation',
				'A-152',
				'Queued',
				'0/6',
				'0 ms',
				'Not started',
			],
			initials: 'NC',
		},
		{
			id: 'assessment-a153',
			facts: [
				'Luis Diaz',
				'West workspace',
				'A deliberately long document classification label',
				'A-153',
				'Running',
				'4/6',
				'1240 ms',
				'120 / 830 / 290 ms',
			],
			initials: 'LD',
		},
	] as const;
	for (const rowFact of rowFacts) {
		const row = requiredElement(records, `#${rowFact.id}`);
		for (const fact of rowFact.facts) expect(row.textContent).toContain(fact);
		const identity = requiredElement(row, '.ex-identity');
		const avatar = requiredElement(identity, '.ex-avatar');
		expect(avatar.textContent).toBe(rowFact.initials);
		expect(avatar.getAttribute('aria-hidden')).toBe('true');
		const avatarStyles = fixtureWindow.getComputedStyle(avatar);
		expect(avatar.getBoundingClientRect().width).toBe(36);
		expect(avatar.getBoundingClientRect().height).toBe(36);
		expect(avatarStyles.borderRadius).toBe('50%');
	}
	expect(records.querySelectorAll('tbody tr ex-breakdown')).toHaveLength(5);
	const queuedRow = requiredElement(records, '#assessment-a152');
	expect(requiredElement(queuedRow, '#status-a152').hasAttribute('tone')).toBe(false);
	expect(queuedRow.querySelector('ex-breakdown')).toBeNull();
	const queuedGauge = requiredElement(
		requiredElement(queuedRow, 'ex-progress'),
		'.ex-progress-gauge',
	);
	expect(queuedGauge.querySelector('.ex-progress-track')).not.toBeNull();
	expect(queuedGauge.querySelector('.ex-progress-fill')).toBeNull();
	const identityDetail = requiredElement(fixture, '#prose-identity .ex-identity-detail');
	expect(fixture.querySelector('#identity-prose')?.textContent).toContain(
		'North workspace · deliberately long authored context for this wrapping example',
	);
	expect(requiredElement(fixture, '#trace-identity').textContent).toContain('Trace example');
	await page.viewport(390, 1000);
	expect(identityDetail.getBoundingClientRect().height).toBeGreaterThan(18);
	expect(identityDetail.scrollWidth).toBeLessThanOrEqual(identityDetail.clientWidth + 1);
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
				'#large-count-progress',
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
				'#status-a148',
				'#status-a149',
				'#status-a150',
				'#status-a151',
				'#status-a152',
				'#status-a153',
				'#nonbadge-done-tone',
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
	const fixtureWindow = fixture.defaultView;
	if (!fixtureWindow) throw new Error('Fixture has no browser window.');
	const zeroGauge = requiredElement(
		requiredElement(fixture, '#zero-progress'),
		'.ex-progress-gauge',
	);
	expect(zeroGauge.getAttribute('aria-label')).toBe('0 of 6 Checks');
	expect(zeroGauge.querySelector('.ex-progress-track')).not.toBeNull();
	expect(zeroGauge.querySelector('.ex-progress-fill')).toBeNull();

	const partialFill = requiredElement(
		requiredElement(fixture, '#partial-progress'),
		'.ex-progress-fill',
	);
	const partialDashArray = Number(partialFill.getAttribute('stroke-dasharray'));
	const partialDashOffset = Number(partialFill.getAttribute('stroke-dashoffset'));
	expect(partialDashArray).toBeCloseTo(2 * Math.PI * 42, 0);
	expect(1 - partialDashOffset / partialDashArray).toBeCloseTo(2 / 6, 3);
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
	const pillRadius = fixtureWindow
		.getComputedStyle(fixture.documentElement)
		.getPropertyValue('--radius-pill')
		.trim();
	for (const theme of ['light', 'dark'] as const) {
		fixture.documentElement.dataset['theme'] = theme;
		expect(
			fixtureWindow.getComputedStyle(requiredElement(zeroGauge, '.ex-progress-track')).stroke,
		).not.toBe('none');
		for (const id of ['#partial-progress', '#complete-progress']) {
			const fill = requiredElement(requiredElement(fixture, id), '.ex-progress-fill');
			const styles = fixtureWindow.getComputedStyle(fill);
			expect(styles.stroke).toBe(resolveThemeColor(fixture, '--accent'));
			expect(styles.strokeLinecap).toBe('round');
		}
		for (const [id, foregroundToken, backgroundToken] of [
			['#status-a148', '--tone-added', '--tone-added-surface'],
			['#status-a149', '--accent', '--tone-note-surface'],
			['#status-a150', '--tone-risk', '--tone-risk-surface'],
		] as const) {
			const badge = requiredElement(fixture, id);
			const styles = fixtureWindow.getComputedStyle(badge);
			expect(styles.color).toBe(resolveThemeColor(fixture, foregroundToken));
			expect(styles.backgroundColor).toBe(resolveThemeBackground(fixture, backgroundToken));
			expect(styles.borderTopLeftRadius).toBe(pillRadius);
		}
		const queuedBadge = fixtureWindow.getComputedStyle(requiredElement(fixture, '#status-a152'));
		expect(queuedBadge.color).toBe(resolveThemeColor(fixture, '--ink-soft'));
		expect(queuedBadge.backgroundColor).toBe(resolveThemeBackground(fixture, '--surface-content'));
		expect(queuedBadge.borderTopLeftRadius).toBe(pillRadius);
		const nonBadgeDone = fixtureWindow.getComputedStyle(
			requiredElement(fixture, '#nonbadge-done-tone'),
		);
		expect(nonBadgeDone.borderLeftColor).toBe(resolveThemeColor(fixture, '--tone-done'));
		expect(nonBadgeDone.borderRadius).toBe(
			fixtureWindow.getComputedStyle(fixture.documentElement).getPropertyValue('--radius-sm'),
		);
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
