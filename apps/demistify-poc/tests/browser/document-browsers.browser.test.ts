import { afterEach, expect, test } from 'vitest';
import { commands, page } from 'vitest/browser';

import { browserScreenshotPath, writeBrowserEvidence } from './browser-evidence.ts';
import {
	capturePreparedAuthor,
	captureReaderReachability,
	captureReaderInspectorHelp,
} from './document-reader-observations.ts';
import { fixtureLocator, openFixture, requiredElement } from './fixture-page.ts';
import {
	capturePublicStyleParity,
	captureFixedPublicPartStyles,
} from './public-part-style-observations.ts';

const modalRule =
	'While the menu is modal, pointer activation and keyboard shortcuts aimed at lower-scope session actions produce no background action.';

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

afterEach(async (): Promise<void> => {
	document.body.replaceChildren();
	await commands.setReducedMotion('no-preference');
});

test.each(
	[1600, 390].flatMap((width) =>
		['light', 'dark'].flatMap((theme) =>
			['no-preference', 'reduce'].map((motion) => ({ width, theme, motion })),
		),
	),
)(
	'public part styles retain selected baseline: $width $theme $motion',
	captureFixedPublicPartStyles,
);

test.each(
	[1600, 390].flatMap((width) =>
		['light', 'dark'].flatMap((theme) =>
			['no-preference', 'reduce'].map((motion) => ({ width, theme, motion })),
		),
	),
)(
	'reader inspector preserves generated Help and authored disclosure: $width $theme $motion',
	captureReaderInspectorHelp,
);

test.each(
	[1600, 390].flatMap((width) =>
		['light', 'dark'].flatMap((theme) =>
			['no-preference', 'reduce'].map((motion) => ({ width, theme, motion })),
		),
	),
)('class-free public reader style and inspector: $width $theme $motion', capturePublicStyleParity);

test.each(
	[1600, 390].flatMap((width) =>
		['light', 'dark'].flatMap((theme) =>
			['no-preference', 'reduce'].map((motion) => ({ width, theme, motion })),
		),
	),
)(
	'selected document surfaces and controls: $width $theme $motion',
	async ({ width, theme, motion }): Promise<void> => {
		await page.viewport(width, width === 390 ? 844 : 1000);

		await commands.setReducedMotion(motion === 'reduce' ? 'reduce' : 'no-preference');
		const { document: fixture, frame } = await openFixture(
			'understanding',
			'',
			'requirements.html?need=U2&stage=program-design#program-design-U2',
		);
		fixture.documentElement.dataset['theme'] = theme;
		await expect
			.poll(() => fixture.querySelector('.ex-design-document-subject small')?.textContent)
			.toBe('U2 · Input ownership');
		await expect.poll(() => fixture.querySelector('.ex-design-stage-record')).not.toBeNull();
		await expect
			.poll(() => fixture.querySelector('.ex-design-stage-record ex-mermaid svg')?.textContent)
			.toContain('InputOwner');
		await expect
			.poll(
				() =>
					fixture.querySelector('.ex-design-stage-record ex-mermaid svg')?.getBoundingClientRect()
						.height ?? 0,
			)
			.toBeGreaterThan(20);
		requiredElement(fixture, '.ex-design-document-header').scrollIntoView({
			block: 'start',
			behavior: 'instant',
		});
		async function captureView(view: 'program' | 'documents' | 'calls'): Promise<void> {
			if (view === 'documents')
				await frame.getByRole('button', { name: 'View together', exact: true }).click();
			if (view === 'calls')
				await frame.getByRole('button', { name: 'Open call stack', exact: true }).last().click();
			if (view === 'documents')
				await expect
					.poll(() => fixture.querySelector('.ex-design-requirement-expanded'))
					.not.toBeNull();
			if (view === 'documents') {
				requiredElement(fixture, '.ex-design-requirement-expanded ex-mermaid').scrollIntoView({
					block: 'center',
					behavior: 'instant',
				});
				await expect
					.poll(
						() =>
							fixture.querySelector('.ex-design-requirement-expanded ex-mermaid svg')?.textContent,
					)
					.toContain('InputOwner');
				requiredElement(fixture, '.ex-design-related-content').scrollTop = 0;
			}
			if (view === 'calls')
				await expect.poll(() => fixture.querySelector('button[data-frame="0"]')).not.toBeNull();
			await Promise.all(
				fixture.getAnimations().map((animation) => animation.finished.catch(() => undefined)),
			);
			if (view !== 'program')
				await expect
					.poll(
						() =>
							getComputedStyle(requiredElement(fixture, 'ex-drawer.ex-design-related-drawer'))
								.opacity,
					)
					.toBe('1');
			const metrics = [
				'.ex-design-document-header',
				'.ex-design-document-panel',
				'.ex-design-document-switcher',
				'ex-drawer.ex-design-related-drawer',
			].flatMap((selector) => {
				const element = fixture.querySelector(selector);
				if (!element) return [];
				const rect = element.getBoundingClientRect();
				const style = getComputedStyle(element);
				return [
					{
						selector,
						x: rect.x,
						y: rect.y,
						width: rect.width,
						height: rect.height,
						background: style.backgroundColor,
						fontSize: style.fontSize,
						lineHeight: style.lineHeight,
					},
				];
			});
			await writeBrowserEvidence(
				`metrics-${width}-${theme}-${motion}-${view}.json`,
				JSON.stringify({ width, theme, motion, view, metrics }, null, 2),
			);
			await page.screenshot({
				path: browserScreenshotPath(`documents-${width}-${theme}-${motion}-${view}.png`),
			});
		}
		await captureView('program');
		await captureView('documents');
		await captureView('calls');
	},
);

test.each(['requirements.html', 'trailing-note.html'])(
	'the compact S2 rule is its authored conditions paragraph: %s',
	async (file): Promise<void> => {
		const { document: fixture, frame } = await openFixture('understanding', '', `${file}?need=U2`);
		await expect
			.poll(() => fixture.querySelector('.ex-design-document-subject small'))
			.not.toBeNull();
		await frame.getByRole('button', { name: 'View together', exact: true }).click();
		await expect.poll(() => fixture.querySelector('.ex-design-rule-layout')).not.toBeNull();
		const rule = requiredElement(fixture, '.ex-design-rule-layout > div > p');
		expect(rule.textContent?.replace(/\s+/g, ' ').trim()).toBe(modalRule);
	},
);

test.each(['needs', 'rules', 'design', 'full'])(
	'standalone public reader accepts the %s collection without call records',
	async (name): Promise<void> => {
		const { document: fixture, frame } = await openFixture(
			'document-browsers',
			'',
			`${name}/requirements.html?need=DraftNeed&stage=verification`,
		);
		await expect.poll(() => fixture.querySelector('ex-requirements[data-ready]')).not.toBeNull();
		expect(fixture.querySelector('[role="alert"]')).toBeNull();
		expect(requiredElement(fixture, '.ex-design-document-subject').textContent).toContain(
			'DraftNeed',
		);
		if (name !== 'full')
			expect(requiredElement(fixture, '.ex-design-stage-detail').textContent).toContain(
				'Not yet written',
			);
		else {
			expect(requiredElement(fixture, '.ex-design-stage-detail').textContent).toContain(
				'Observed: the draft was cleared after refusal.',
			);
			expect(requiredElement(fixture, '.ex-design-stage-detail').textContent).toContain('Not run');
			expect(requiredElement(fixture, '.ex-design-stage-detail').textContent).not.toContain(
				'Example only',
			);
		}
		await frame.getByRole('button', { name: 'View together', exact: true }).click();
		if (name !== 'full')
			expect(requiredElement(fixture, '.ex-design-expanded-grid').textContent).toContain(
				'Not yet written',
			);
		await frame.getByRole('button', { name: 'Open call stack', exact: true }).last().click();
		await expect.poll(() => fixture.querySelector('ex-call-stack[data-ready]')).not.toBeNull();
		expect(requiredElement(fixture, 'ex-call-stack').textContent).toContain(
			'No call paths are supplied',
		);
		expect(
			[...fixture.scripts]
				.filter((script) => script.src)
				.map((script) => new URL(script.src).pathname),
		).toEqual(['/__demistify/library/runtime.js']);
		expect(
			[...fixture.querySelectorAll<HTMLLinkElement>('link[rel="stylesheet"]')].map(
				(link) => new URL(link.href).pathname,
			),
		).toEqual(['/__demistify/library/theme.css']);
		const loadedAssets =
			fixture.defaultView?.performance
				.getEntriesByType('resource')
				.map((entry) => entry.name)
				.filter((assetUrl) => /\.(?:js|css)(?:[?#]|$)/.test(assetUrl)) ?? [];
		expect(loadedAssets.length).toBeGreaterThan(2);
		expect(
			loadedAssets.every((assetUrl) =>
				new URL(assetUrl).pathname.startsWith('/__demistify/library/'),
			),
		).toBe(true);
	},
);

test('two rich projections retain their actual local href, label and ARIA targets', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture(
		'document-browsers',
		'',
		'full/requirements.html?need=DraftNeed&stage=specification',
	);
	await expect.poll(() => fixture.querySelector('ex-requirements[data-ready]')).not.toBeNull();
	await frame.getByRole('button', { name: 'View together', exact: true }).click();
	const rule = requiredElement(fixture, '.ex-design-rule-layout');
	expect(rule.textContent).toContain('After a refused save, the same draft text remains editable.');
	expect(rule.querySelector(':scope > div')?.textContent).not.toContain(
		'A trailing implementation note',
	);
	await frame.getByText('Source details · DraftRule', { exact: true }).click();
	const projections = [
		requiredElement(rule, '[data-ex-part="rule"]'),
		requiredElement(rule, '.ex-design-record-source [data-ex-part="rule"]'),
	];
	expect(projections[0]?.querySelector('input')?.id).not.toBe(
		projections[1]?.querySelector('input')?.id,
	);
	for (const projection of projections) {
		const input = requiredElement(projection, 'input');
		const label = requiredElement(projection, 'label');
		expect(label.getAttribute('for')).toBe(input.id);
		const aria = input.getAttribute('aria-labelledby')?.split(' ') ?? [];
		expect(aria).toHaveLength(2);
		for (const id of aria) expect(projection.querySelector(`#${CSS.escape(id)}`)).not.toBeNull();
		const help = input.getAttribute('aria-describedby');
		expect(help).toBeTruthy();
		expect(projection.querySelector(`#${CSS.escape(help ?? '')}`)?.textContent).toBe(
			'The input remains editable.',
		);
		const link = requiredElement(projection, 'a[href^="#"]');
		expect(projection.querySelector(link.getAttribute('href') ?? 'missing')).not.toBeNull();
		expect(requiredElement(projection, 'a[href^="https:"]').getAttribute('href')).toBe(
			'https://example.org/reference#outside',
		);
	}
});

async function readerRevision(
	file: string,
	outcome: 'loaded' | 'failed' = 'loaded',
): Promise<string> {
	document.body.replaceChildren();
	const { document: fixture, frame } = await openFixture('document-browsers', '', file);
	await expect
		.poll(() => fixture.querySelector('ex-requirements[data-ready],ex-requirements [role="alert"]'))
		.not.toBeNull();
	if (outcome === 'failed') {
		expect(fixture.querySelector('ex-requirements[data-ready]')).toBeNull();
		expect(fixture.querySelector('ex-requirements [role="alert"]')).not.toBeNull();
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

/** Generic observations only: the independent author oracle judges their factual fidelity. */
test.each([1600, 390])(
	'capture external prepared author reader at %ipx',
	capturePreparedAuthor,
	120_000,
);

test('the illustrative result caption displays its observation identity once', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture(
		'understanding',
		'',
		'requirements.html?need=U2',
	);
	await expect.poll(() => fixture.querySelector('ex-requirements[data-ready]')).not.toBeNull();
	await frame.getByRole('button', { name: 'View together', exact: true }).click();
	const observation = requiredElement(fixture, '.ex-design-example-outcome[data-record-id="O2"]');
	expect((observation.textContent ?? '').split('O2').length - 1).toBe(1);
	expect(observation.textContent).toContain('Example only');
	expect(observation.textContent).toContain('target not supplied');
	expect(observation.textContent).not.toContain('supplied.O2');
	const qualification = [...observation.querySelectorAll('p')].find((element) =>
		element.textContent
			?.replace(/\s+/g, ' ')
			.includes('A target build, setup and real-path check are still needed for product proof.'),
	);
	expect(qualification).toBeDefined();
	if (!qualification) throw new Error('Missing visible product-proof qualification');
	expect(qualification.closest('details:not([open])')).toBeNull();
	await expect.element(fixtureLocator(frame, qualification)).toBeVisible();
});

test.each([1600, 390])(
	'list, overview and detail preserve every same-scenario observation outcome at %ipx',
	async (width): Promise<void> => {
		await page.viewport(width, 1000);
		const { document: fixture, frame } = await openFixture(
			'document-browsers',
			'',
			'multiple/requirements.html',
		);
		await expect.poll(() => fixture.querySelector('ex-requirements[data-ready]')).not.toBeNull();
		expect(
			readOutcomeBadges(
				requiredElement(fixture, '.ex-design-requirement-list [data-need="DraftNeed"]'),
			),
		).toEqual(['Pass', 'Fail', 'Not run']);
		await page.screenshot({ path: browserScreenshotPath(`multiple-outcomes-${width}-list.png`) });
		await fixtureLocator(
			frame,
			requiredElement(fixture, '.ex-design-requirement-list [data-need="DraftNeed"]'),
		).click();
		expect(readOutcomeBadges(requiredElement(fixture, '#verification-DraftNeed'))).toEqual([
			'Pass',
			'Fail',
			'Not run',
		]);
		await frame.getByRole('button', { name: 'Verification', exact: true }).click();
		expect(readOutcomeBadges(requiredElement(fixture, '.ex-design-stage-detail'))).toEqual([
			'Pass',
			'Fail',
			'Not run',
		]);
	},
);

function readOutcomeBadges(scope: Element): readonly string[] {
	return [...scope.querySelectorAll('ex-badge')].map((badge) => badge.textContent?.trim() ?? '');
}

test('a wide Specification table stays local while its phone heading and rule remain readable', async (): Promise<void> => {
	// Arrange: ordinary authored columns have an intrinsic width larger than the phone.
	await page.viewport(390, 844);
	const { document: fixture, frame } = await openFixture(
		'document-browsers',
		'',
		'wide-table/requirements.html?need=DraftNeed&stage=specification',
	);
	await expect.poll(() => fixture.querySelector('.ex-design-stage-record')).not.toBeNull();
	const selectors = [
		'.ex-design-document-panel',
		'.ex-design-stage-detail',
		'.ex-design-stage-record',
		'.ex-design-stage-record-heading',
		'.ex-design-stage-record [data-ex-part="rule"]',
		'.ex-design-stage-record .ex-table-scroll',
	];
	const geometry = selectors.map((selector) => {
		const element = requiredElement(fixture, selector);
		const style = getComputedStyle(element);
		return {
			selector,
			bounds: element.getBoundingClientRect(),
			clientWidth: element.clientWidth,
			scrollWidth: element.scrollWidth,
			minWidth: style.minWidth,
			gridColumns: style.gridTemplateColumns,
			overflowX: style.overflowX,
		};
	});
	await writeBrowserEvidence('wide-table-phone-geometry.json', JSON.stringify(geometry, null, 2));
	await page.screenshot({ path: browserScreenshotPath('wide-table-phone.png') });
	// Assert: the viewport is the independent boundary, not a copy of a CSS declaration.
	for (const selector of selectors) {
		const bounds = requiredElement(fixture, selector).getBoundingClientRect();
		expect(bounds.left, selector).toBeGreaterThanOrEqual(0);
		expect(bounds.right, selector).toBeLessThanOrEqual(390);
	}
	expect(fixture.documentElement.scrollWidth).toBeLessThanOrEqual(390);
	await captureReaderReachability({
		fixture,
		frame,
		scope: requiredElement(fixture, '.ex-design-document-panel'),
		label: 'wide-table-phone',
	});
});

test('a 390px Program diagram reaches both local endpoints with native horizontal wheel input', async (): Promise<void> => {
	await page.viewport(390, 844);
	const { document: fixture, frame } = await openFixture(
		'understanding',
		'',
		'requirements.html?need=U2&stage=program-design#program-design-U2',
	);
	fixture.documentElement.dataset['theme'] = 'dark';
	const selector = '.ex-design-stage-record ex-mermaid';
	await expect
		.poll(() => fixture.querySelector(`${selector} svg`)?.textContent)
		.toContain('InputOwner');
	const diagram = requiredElement(fixture, selector);
	await fixtureLocator(frame, diagram).hover();
	const nodes = [...diagram.querySelectorAll('g.node')].toSorted(
		(left, right) => left.getBoundingClientRect().left - right.getBoundingClientRect().left,
	);
	const first = nodes[0],
		last = nodes.at(-1);
	if (!first || !last) throw new Error('Missing real diagram endpoints');
	expect(diagram.scrollWidth).toBeGreaterThan(diagram.clientWidth);
	expect(diagram.scrollLeft).toBeCloseTo(0, 0);
	expect(first.getBoundingClientRect().left).toBeGreaterThanOrEqual(
		diagram.getBoundingClientRect().left - 1,
	);
	expect(first.getBoundingClientRect().right).toBeLessThanOrEqual(
		diagram.getBoundingClientRect().right + 1,
	);
	const start = {
		scrollLeft: diagram.scrollLeft,
		clientWidth: diagram.clientWidth,
		scrollWidth: diagram.scrollWidth,
		first: first.getBoundingClientRect(),
		last: last.getBoundingClientRect(),
	};
	await commands.wheelLocalContent(selector, 2000);
	await expect
		.poll(() => diagram.scrollLeft)
		.toBeCloseTo(diagram.scrollWidth - diagram.clientWidth, 0);
	expect(last.getBoundingClientRect().left).toBeGreaterThanOrEqual(
		diagram.getBoundingClientRect().left - 1,
	);
	expect(last.getBoundingClientRect().right).toBeLessThanOrEqual(
		diagram.getBoundingClientRect().right + 1,
	);
	expect(fixture.documentElement.scrollWidth).toBeLessThanOrEqual(390);
	const end = {
		scrollLeft: diagram.scrollLeft,
		first: first.getBoundingClientRect(),
		last: last.getBoundingClientRect(),
	};
	await page.screenshot({ path: browserScreenshotPath('program-local-scroll-390-end.png') });
	await commands.wheelLocalContent(selector, -2000);
	await expect.poll(() => diagram.scrollLeft).toBeCloseTo(0, 0);
	expect(first.getBoundingClientRect().left).toBeGreaterThanOrEqual(
		diagram.getBoundingClientRect().left - 1,
	);
	expect(fixture.documentElement.scrollWidth).toBeLessThanOrEqual(390);
	await writeBrowserEvidence(
		'program-local-scroll-390.json',
		JSON.stringify(
			{
				start,
				end,
				returnedScrollLeft: diagram.scrollLeft,
				pageWidth: fixture.documentElement.scrollWidth,
			},
			null,
			2,
		),
	);
});
