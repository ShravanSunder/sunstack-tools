import { afterEach, expect, test } from 'vitest';
import { commands, page } from 'vitest/browser';

import { browserScreenshotPath } from './browser-evidence.ts';
import { fixtureLocator, openFixture, requiredElement } from './fixture-page.ts';

const modalRule =
	'While the menu is modal, pointer activation and keyboard shortcuts aimed at lower-scope session actions produce no background action.';

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
			await expect(
				JSON.stringify({ width, theme, motion, view, metrics }, null, 2),
			).toMatchFileSnapshot(
				browserScreenshotPath(`metrics-${width}-${theme}-${motion}-${view}.json`),
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

interface ReaderObservation {
	readonly html: string;
	readonly text: string;
	readonly diagrams: readonly {
		readonly html: string;
		readonly text: string;
		readonly width: number;
		readonly height: number;
	}[];
	readonly markdown: readonly { readonly html: string; readonly text: string }[];
}

async function observeReader(scope: Element): Promise<ReaderObservation> {
	const visibleDiagrams = [...scope.querySelectorAll('ex-mermaid')].filter(
		(element) => !element.closest('details:not([open])') && element.getClientRects().length > 0,
	);
	await visibleDiagrams.reduce(async (previous, element): Promise<void> => {
		await previous;
		element.scrollIntoView({ block: 'center', behavior: 'instant' });
		await expect
			.poll(() => element.querySelector('svg')?.textContent?.trim().length ?? 0)
			.toBeGreaterThan(0);
	}, Promise.resolve());
	await expect
		.poll(() =>
			[...scope.querySelectorAll('ex-markdown')]
				.filter(
					(element) =>
						!element.closest('details:not([open])') && element.getClientRects().length > 0,
				)
				.every((element) => element.hasAttribute('prepared')),
		)
		.toBe(true);
	const diagrams = [...scope.querySelectorAll('ex-mermaid svg')].map((svg) => ({
		html: svg.outerHTML,
		text: svg.textContent ?? '',
		width: svg.getBoundingClientRect().width,
		height: svg.getBoundingClientRect().height,
	}));
	const markdown = [...scope.querySelectorAll('ex-markdown[prepared]')].map((element) => ({
		html: element.innerHTML,
		text: element.textContent ?? '',
	}));
	return { html: scope.innerHTML, text: scope.textContent ?? '', diagrams, markdown };
}

/** Generic observations only: the independent author oracle judges their factual fidelity. */
test.each([1600, 390])(
	'capture external prepared author reader at %ipx',
	async (width): Promise<void> => {
		await page.viewport(width, width === 390 ? 844 : 1000);
		const { document: fixture, frame } = await openFixture(
			'document-author',
			'',
			'requirements.html',
		);
		await expect.poll(() => fixture.querySelector('ex-requirements[data-ready]')).not.toBeNull();
		expect(fixture.querySelector('ex-requirements [role="alert"]')).toBeNull();
		const scripts = [...fixture.scripts].filter((script) => script.src).map((script) => script.src);
		const styles = [...fixture.querySelectorAll<HTMLLinkElement>('link[rel="stylesheet"]')].map(
			(link) => link.href,
		);
		expect(scripts.length).toBeGreaterThan(0);
		expect(scripts.every((address) => new URL(address).pathname.endsWith('/runtime.js'))).toBe(
			true,
		);
		expect(styles.length).toBeGreaterThan(0);
		expect(styles.every((address) => new URL(address).pathname.endsWith('/theme.css'))).toBe(true);
		const needs = [
			...fixture.querySelectorAll<HTMLElement>('.ex-design-requirement-list button[data-need]'),
		].map((button) => button.dataset['need'] ?? '');
		expect(needs.length).toBeGreaterThan(0);
		await needs.reduce(async (previous, needId): Promise<void> => {
			await previous;
			const back = fixture.querySelector<HTMLButtonElement>('[data-action="back-to-needs"]');
			if (back && back.getClientRects().length > 0) await fixtureLocator(frame, back).click();
			await fixtureLocator(
				frame,
				requiredElement(
					fixture,
					`.ex-design-requirement-list button[data-need="${CSS.escape(needId)}"]`,
				),
			).click();
			await ['Overview', 'Specification', 'Program Design', 'Verification'].reduce(
				async (previousStage, stage): Promise<void> => {
					await previousStage;
					await frame.getByRole('button', { name: stage, exact: true }).click();
					const panel = requiredElement(fixture, '.ex-design-document-panel');
					const observation = await observeReader(panel);
					expect(panel.querySelector('[role="alert"]')).toBeNull();
					requiredElement(fixture, '.ex-design-document-header').scrollIntoView({
						block: 'start',
						behavior: 'instant',
					});
					await expect(
						JSON.stringify({ width, needId, stage, scripts, styles, observation }, null, 2),
					).toMatchFileSnapshot(browserScreenshotPath(`author-${width}-${needId}-${stage}.json`));
					await page.screenshot({
						path: browserScreenshotPath(`author-${width}-${needId}-${stage}.png`),
					});
				},
				Promise.resolve(),
			);
			await frame.getByRole('button', { name: 'View together', exact: true }).click();
			const workspace = requiredElement(fixture, 'ex-drawer.ex-design-related-drawer');
			const related = await observeReader(requiredElement(workspace, '.ex-design-related-content'));
			await Promise.all(
				fixture.getAnimations().map((animation) => animation.finished.catch(() => undefined)),
			);
			requiredElement(workspace, '.ex-design-related-content').scrollTop = 0;
			await expect(
				JSON.stringify({ width, needId, view: 'related', observation: related }, null, 2),
			).toMatchFileSnapshot(browserScreenshotPath(`author-${width}-${needId}-related.json`));
			await page.screenshot({
				path: browserScreenshotPath(`author-${width}-${needId}-related.png`),
			});
			await frame.getByRole('button', { name: 'Open call stack', exact: true }).last().click();
			await expect.poll(() => workspace.querySelector('ex-call-stack[data-ready]')).not.toBeNull();
			const call = workspace.querySelector<HTMLButtonElement>('button[data-frame]');
			if (call) await fixtureLocator(frame, call).click();
			const calls = await observeReader(workspace);
			await Promise.all(
				fixture.getAnimations().map((animation) => animation.finished.catch(() => undefined)),
			);
			await expect(
				JSON.stringify({ width, needId, view: 'calls', observation: calls }, null, 2),
			).toMatchFileSnapshot(browserScreenshotPath(`author-${width}-${needId}-calls.json`));
			await page.screenshot({ path: browserScreenshotPath(`author-${width}-${needId}-calls.png`) });
			const closeDetails = workspace.querySelector<HTMLButtonElement>(
				'[data-action="close-call-details"]',
			);
			if (closeDetails && closeDetails.getClientRects().length > 0)
				await fixtureLocator(frame, closeDetails).click();
			await frame.getByRole('button', { name: 'Back to related documents', exact: true }).click();
			await frame.getByRole('button', { name: 'Close related documents', exact: true }).click();
		}, Promise.resolve());
		const resources =
			fixture.defaultView?.performance.getEntriesByType('resource').map((entry) => ({
				name: entry.name,
				initiatorType:
					'initiatorType' in entry && typeof entry.initiatorType === 'string'
						? entry.initiatorType
						: 'unknown',
				entryType: entry.entryType,
			})) ?? [];
		expect(resources.length).toBeGreaterThan(0);
		await expect(
			JSON.stringify(
				{
					width,
					scripts,
					styles,
					resources,
					limits:
						'Document PerformanceResourceTiming only. Worker-internal, cached or browser-internal requests absent from these entries are not a complete request graph.',
				},
				null,
				2,
			),
		).toMatchFileSnapshot(browserScreenshotPath(`author-${width}-resources.json`));
	},
	120_000,
);

test('public labels name both reader groups while unlabeled readers keep their defaults', async (): Promise<void> => {
	const { document: fixture } = await openFixture('document-browsers', '', 'labeled.html');
	await expect
		.poll(
			() =>
				fixture.querySelectorAll('ex-requirements[data-ready],ex-call-stack[data-ready]').length,
		)
		.toBe(2);
	expect(
		requiredElement(fixture, 'ex-requirements [role="group"]').getAttribute('aria-label'),
	).toBe('Draft design needs');
	expect(requiredElement(fixture, 'ex-call-stack [role="group"]').getAttribute('aria-label')).toBe(
		'Draft source calls',
	);
	document.body.replaceChildren();
	const { document: defaultNeeds } = await openFixture('understanding', '', 'requirements.html');
	await expect.poll(() => defaultNeeds.querySelector('ex-requirements[data-ready]')).not.toBeNull();
	expect(
		requiredElement(defaultNeeds, 'ex-requirements [role="group"]').getAttribute('aria-label'),
	).toBe('Requirements browser');
	document.body.replaceChildren();
	const { document: defaultCalls } = await openFixture('understanding', '', 'execution.html');
	await expect.poll(() => defaultCalls.querySelector('ex-call-stack[data-ready]')).not.toBeNull();
	expect(
		requiredElement(defaultCalls, 'ex-call-stack [role="group"]').getAttribute('aria-label'),
	).toBe('Call stack browser');
});
