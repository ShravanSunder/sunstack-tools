import { expect } from 'vitest';
import { commands, page } from 'vitest/browser';

import { collectionSchema } from '../../../../packages/demistify/src/design-documents/collection.ts';
import { browserScreenshotPath } from './browser-evidence.ts';
import { fixtureLocator, openFixture, requiredElement } from './fixture-page.ts';

function publicStyleSamples(scope: Element, stage: string): readonly object[] {
	const selectors = [
		'[data-ex-part="sketch"]',
		'[data-ex-part="sketch"] > figcaption',
		'[data-ex-part="sketch"] ~ details:has(> .ex-table-scroll)',
		'[data-ex-part="cases"]',
		'[data-ex-part="cases"] > figcaption',
		'[data-ex-part="procedure"]',
		'[data-ex-part="procedure"] summary',
		'.ex-design-example-outcome > small + p',
	];
	return selectors.flatMap((selector) =>
		[...scope.querySelectorAll(selector)].map((element) => {
			const style = getComputedStyle(element);
			return {
				stage,
				selector,
				display: style.display,
				margin: style.margin,
				padding: style.padding,
				color: style.color,
				background: style.backgroundColor,
				font: style.font,
				radius: style.borderRadius,
				width: element.getBoundingClientRect().width,
			};
		}),
	);
}

/** Same-content fixtures separate the public recipe from legacy author classes. */
export async function capturePublicStyleParity(props: {
	readonly width: number;
	readonly theme: string;
	readonly motion: string;
}): Promise<void> {
	const { width, theme, motion } = props;
	await page.viewport(width, width === 390 ? 844 : 1000);
	await commands.setReducedMotion(motion === 'reduce' ? 'reduce' : 'no-preference');
	const samples: string[] = [];
	await ['understanding', 'document-browsers'].reduce(async (previous, name): Promise<void> => {
		await previous;
		document.body.replaceChildren();
		const file = `${name === 'understanding' ? '' : 'public-style/'}requirements.html?need=U2&stage=program-design`;
		const { document: fixture, frame } = await openFixture(
			name === 'understanding' ? 'understanding' : 'document-browsers',
			'',
			file,
		);
		fixture.documentElement.dataset['theme'] = theme;
		await expect
			.poll(() => fixture.querySelector('.ex-design-stage-record ex-mermaid svg'))
			.not.toBeNull();
		const wrapper = requiredElement(fixture, 'ex-zoom:has(> ex-requirements)');
		const sidebar = requiredElement(wrapper, ':scope > .ex-inspect-controls .ex-inspect-sidebar');
		expect(getComputedStyle(sidebar).display).toBe('none');
		const styles: object[] = [];
		await ['Program Design', 'Specification', 'Verification'].reduce(
			async (prior, stage): Promise<void> => {
				await prior;
				await frame.getByRole('button', { name: stage, exact: true }).click();
				await Promise.all(
					fixture.getAnimations().map((animation) => animation.finished.catch(() => undefined)),
				);
				const panel = requiredElement(fixture, '.ex-design-document-panel');
				styles.push(...publicStyleSamples(panel, stage));
				await page.screenshot({
					path: browserScreenshotPath(
						`public-parity-${width}-${theme}-${motion}-${name}-${stage}.png`,
					),
				});
			},
			Promise.resolve(),
		);
		await frame.getByRole('button', { name: 'View together', exact: true }).click();
		const workspace = requiredElement(fixture, 'ex-drawer.ex-design-related-drawer');
		await expect.poll(() => workspace.matches(':popover-open')).toBe(true);
		await Promise.all(
			fixture.getAnimations().map((animation) => animation.finished.catch(() => undefined)),
		);
		styles.push(...publicStyleSamples(workspace, 'Related'));
		await page.screenshot({
			path: browserScreenshotPath(`public-parity-${width}-${theme}-${motion}-${name}-related.png`),
		});
		await frame.getByRole('button', { name: 'Close related documents', exact: true }).click();
		samples.push(JSON.stringify(styles));
		await expect(
			JSON.stringify({ width, theme, motion, name, styles }, null, 2),
		).toMatchFileSnapshot(
			browserScreenshotPath(`public-parity-${width}-${theme}-${motion}-${name}.json`),
		);
		if (name === 'document-browsers') {
			const reader = requiredElement(fixture, 'ex-requirements');
			const expand = requiredElement(
				wrapper,
				':scope > .ex-inspect-controls button:not(.ex-inspect-sidebar)',
			);
			await fixtureLocator(frame, expand).click();
			const dialog = requiredElement(fixture, '.ex-inspect-dialog');
			await expect.poll(() => dialog.matches(':modal')).toBe(true);
			expect(requiredElement(dialog, 'ex-requirements')).toBe(reader);
			if (width === 390) {
				const detail = requiredElement(dialog, '.ex-design-requirement-detail');
				expect(getComputedStyle(detail).borderTopWidth).toBe('0px');
				expect(detail.getBoundingClientRect().right).toBeLessThanOrEqual(width);
			}
			await page.screenshot({
				path: browserScreenshotPath(`public-inspector-${width}-${theme}-${motion}.png`),
			});
			await frame
				.getByRole('dialog', { name: 'User requirements', exact: true })
				.getByRole('button', { name: /^Close/ })
				.click();
			await expect.poll(() => fixture.querySelector('.ex-inspect-dialog')).toBeNull();
			expect(wrapper.querySelector('ex-requirements')).toBe(reader);
			await expect.poll(() => fixture.activeElement).toBe(expand);
			await expect
				.poll(() => fixture.querySelector('.ex-design-example-outcome p [data-comment-control]'))
				.not.toBeNull();
			await fixtureLocator(
				frame,
				requiredElement(fixture, '.ex-design-example-outcome p [data-comment-control]'),
			).click();
			await frame
				.getByRole('textbox', { name: 'Your comment', exact: true })
				.fill('Check the public reader surface.');
			expect(fixture.activeElement?.getAttribute('aria-label')).toBe('Your comment');
			await frame.getByRole('textbox', { name: 'Your comment', exact: true }).fill('');
			await frame.getByRole('button', { name: 'Close comments', exact: true }).click();
		}
	}, Promise.resolve());
	expect(samples[1]).toBe(samples[0]);
}

interface ReaderSurface {
	readonly fixture: Document;
	readonly frame: ReturnType<typeof page.frameLocator>;
	readonly scope: Element;
	readonly label: string;
}

interface EndpointPosition {
	readonly left: number;
	readonly bounds: DOMRect;
	readonly first: DOMRect;
	readonly last: DOMRect;
}

interface HorizontalObservation {
	readonly tag: string;
	readonly start: EndpointPosition & {
		readonly clientWidth: number;
		readonly scrollWidth: number;
	};
	readonly end: EndpointPosition;
	readonly returnedLeft: number;
}

/** Native input and independently measured endpoints distinguish local scrolling from clipping. */
export async function captureReaderReachability(props: ReaderSurface): Promise<void> {
	const { fixture, frame, scope, label } = props;
	const view = fixture.defaultView;
	if (!view) throw new Error('Missing reader window');
	const surrounding = [
		...scope.querySelectorAll('.ex-design-stage-record,.ex-design-stage-record-heading'),
	].map((element) => ({ className: element.className, bounds: element.getBoundingClientRect() }));
	for (const element of surrounding) {
		expect(element.bounds.left).toBeGreaterThanOrEqual(0);
		expect(element.bounds.right).toBeLessThanOrEqual(view.innerWidth);
	}
	const surfaces = [...scope.querySelectorAll('.ex-table-scroll,ex-mermaid')].filter(
		(element) => !element.closest('details:not([open])') && element.getClientRects().length > 0,
	);
	const observations: HorizontalObservation[] = [];
	await surfaces.reduce(async (previous, scroller, index): Promise<void> => {
		await previous;
		await fixtureLocator(frame, scroller).hover();
		const endpoints = [
			...scroller.querySelectorAll(
				scroller.matches('ex-mermaid') ? 'g.node' : 'thead tr:first-child > *',
			),
		].toSorted(
			(left, right) => left.getBoundingClientRect().left - right.getBoundingClientRect().left,
		);
		const first = endpoints[0],
			last = endpoints.at(-1);
		if (!first || !last) throw new Error('Reader surface has no observable endpoints');
		const marker = crypto.randomUUID();
		scroller.setAttribute('data-reachability-wheel', marker);
		const selector = `[data-reachability-wheel="${marker}"]`;
		const delta = scroller.scrollWidth + scroller.clientWidth;
		await commands.wheelLocalContent(selector, -delta);
		await expect.poll(() => scroller.scrollLeft).toBeCloseTo(0, 0);
		const start = {
			left: scroller.scrollLeft,
			bounds: scroller.getBoundingClientRect(),
			clientWidth: scroller.clientWidth,
			scrollWidth: scroller.scrollWidth,
			first: first.getBoundingClientRect(),
			last: last.getBoundingClientRect(),
		};
		expect(start.first.left).toBeGreaterThanOrEqual(start.bounds.left - 1);
		expect(start.first.right).toBeLessThanOrEqual(start.bounds.right + 1);
		await page.screenshot({ path: browserScreenshotPath(`${label}-surface-${index}-start.png`) });
		await commands.wheelLocalContent(selector, delta);
		const firstGesture = {
			left: scroller.scrollLeft,
			clientWidth: scroller.clientWidth,
			scrollWidth: scroller.scrollWidth,
			bounds: scroller.getBoundingClientRect(),
			last: last.getBoundingClientRect(),
		};
		await commands.wheelLocalContent(selector, delta);
		await expect(
			JSON.stringify(
				{
					firstGesture,
					secondGesture: {
						left: scroller.scrollLeft,
						clientWidth: scroller.clientWidth,
						scrollWidth: scroller.scrollWidth,
						bounds: scroller.getBoundingClientRect(),
						last: last.getBoundingClientRect(),
					},
				},
				null,
				2,
			),
		).toMatchFileSnapshot(browserScreenshotPath(`${label}-surface-${index}-wheel-diagnostic.json`));
		await expect
			.poll(() => scroller.scrollLeft)
			.toBeCloseTo(scroller.scrollWidth - scroller.clientWidth, 0);
		const maximumLeft = scroller.scrollLeft;
		const viewport = scroller.getBoundingClientRect();
		const endpoint = last.getBoundingClientRect();
		// Trailing diagram space can move a fully readable final node past the left edge at max.
		if (
			endpoint.width <= viewport.width &&
			(endpoint.left < viewport.left || endpoint.right > viewport.right)
		) {
			await commands.wheelLocalContent(
				selector,
				endpoint.left - viewport.left - (viewport.width - endpoint.width) / 2,
			);
			await expect
				.poll(() => {
					const bounds = last.getBoundingClientRect();
					return bounds.left >= viewport.left - 1 && bounds.right <= viewport.right + 1;
				})
				.toBe(true);
		}
		const end = {
			left: scroller.scrollLeft,
			bounds: scroller.getBoundingClientRect(),
			first: first.getBoundingClientRect(),
			last: last.getBoundingClientRect(),
			maximumLeft,
		};
		expect(end.last.left).toBeGreaterThanOrEqual(end.bounds.left - 1);
		expect(end.last.right).toBeLessThanOrEqual(end.bounds.right + 1);
		expect(fixture.documentElement.scrollWidth).toBeLessThanOrEqual(view.innerWidth);
		await page.screenshot({ path: browserScreenshotPath(`${label}-surface-${index}-end.png`) });
		await commands.wheelLocalContent(selector, -delta);
		await expect.poll(() => scroller.scrollLeft).toBeCloseTo(0, 0);
		observations.push({ tag: scroller.tagName, start, end, returnedLeft: scroller.scrollLeft });
	}, Promise.resolve());
	await expect(
		JSON.stringify(
			{ label, surrounding, observations, pageWidth: fixture.documentElement.scrollWidth },
			null,
			2,
		),
	).toMatchFileSnapshot(browserScreenshotPath(`${label}-reachability.json`));
}

async function captureEveryCall(props: ReaderSurface): Promise<void> {
	const { fixture, frame, scope, label } = props;
	const collection = collectionSchema.parse(
		JSON.parse(requiredElement(fixture, 'script[data-ex-design-collection]').textContent ?? 'null'),
	);
	await collection.records.reduce(async (previousStack, { record: stack }): Promise<void> => {
		await previousStack;
		if (stack.kind !== 'stack') return;
		await fixtureLocator(frame, requiredElement(scope, 'ex-design-picker summary')).click();
		await fixtureLocator(
			frame,
			requiredElement(scope, `button[data-picker-value="${CSS.escape(stack.id)}"]`),
		).click();
		await stack.frames.reduce(async (previousFrame, frameId, index): Promise<void> => {
			await previousFrame;
			const expected = collection.records.find(({ record }) => record.id === frameId)?.record;
			if (!expected || expected.kind !== 'frame') throw new Error('Missing supplied call frame');
			const button = requiredElement(scope, `button[data-frame="${index}"]`);
			const supporting = button.closest('details');
			if (supporting && !supporting.open) {
				await fixtureLocator(frame, requiredElement(supporting, 'summary')).click();
				await expect.poll(() => supporting.open).toBe(true);
			}
			await fixtureLocator(frame, button).click();
			const detail = requiredElement(scope, '.ex-design-frame-detail');
			await expect
				.poll(() => detail.querySelector('pre code')?.textContent)
				.toBe(expected.statement);
			expect(requiredElement(detail, '.ex-design-local-heading').textContent).toBe(expected.name);
			expect(requiredElement(detail, '.ex-design-detail-description').textContent).toBe(
				expected.responsibility,
			);
			expect(requiredElement(detail, '.ex-design-source-caption').textContent).toContain(
				expected.source,
			);
			expect(requiredElement(detail, 'small').textContent).toContain(expected.category);
			if (expected.branch)
				expect(requiredElement(detail, '.ex-design-frame-branch').textContent).toContain(
					expected.branch,
				);
			const requirement = detail.querySelector<HTMLAnchorElement>('.ex-design-frame-requirement a');
			if (expected.requirement)
				expect(new URL(requirement?.href ?? fixture.URL).searchParams.get('need')).toBe(
					expected.requirement,
				);
			else expect(requirement).toBeNull();
			await Promise.all(
				fixture.getAnimations().map((animation) => animation.finished.catch(() => undefined)),
			);
			await expect.element(fixtureLocator(frame, detail)).toBeVisible();
			await expect(
				JSON.stringify(
					{
						stackId: stack.id,
						frameId,
						supportingOpen: supporting?.open ?? null,
						detail: await observeReader(detail),
						bounds: detail.getBoundingClientRect(),
					},
					null,
					2,
				),
			).toMatchFileSnapshot(browserScreenshotPath(`${label}-${stack.id}-${frameId}-detail.json`));
			await page.screenshot({
				path: browserScreenshotPath(`${label}-${stack.id}-${frameId}-detail.png`),
			});
			const close = scope.querySelector('[data-action="close-call-details"]');
			if (close && close.getClientRects().length > 0) await fixtureLocator(frame, close).click();
		}, Promise.resolve());
	}, Promise.resolve());
}

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

/** Capture generic public-reader output; factual scoring belongs to the independent oracle. */
export async function capturePreparedAuthor(width: number): Promise<void> {
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
	expect(scripts.every((address) => new URL(address).pathname.endsWith('/runtime.js'))).toBe(true);
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
				await captureReaderReachability({
					fixture,
					frame,
					scope: panel,
					label: `author-${width}-${needId}-${stage}`,
				});
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
		await captureReaderReachability({
			fixture,
			frame,
			scope: requiredElement(workspace, '.ex-design-related-content'),
			label: `author-${width}-${needId}-related`,
		});
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
		await captureEveryCall({
			fixture,
			frame,
			scope: workspace,
			label: `author-${width}-${needId}-calls`,
		});
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
	const collection = collectionSchema.parse(
		JSON.parse(requiredElement(fixture, 'script[data-ex-design-collection]').textContent ?? 'null'),
	);
	if (collection.records.some(({ record }) => record.kind === 'stack')) {
		document.body.replaceChildren();
		const cold = await openFixture('document-author', '', 'execution.html');
		await expect
			.poll(() => cold.document.querySelector('ex-call-stack[data-ready]'))
			.not.toBeNull();
		expect(new URL(cold.document.URL).search).toBe('');
		expect(
			requiredElement(
				cold.document,
				'ex-call-stack button[data-frame][aria-pressed="true"]',
			).getAttribute('data-frame'),
		).toBe('0');
		await expect(
			JSON.stringify(
				{
					url: cold.document.URL,
					entry: await observeReader(requiredElement(cold.document, 'ex-call-stack')),
				},
				null,
				2,
			),
		).toMatchFileSnapshot(browserScreenshotPath(`author-${width}-standalone-entry.json`));
		await page.screenshot({ path: browserScreenshotPath(`author-${width}-standalone-entry.png`) });
		await captureEveryCall({
			fixture: cold.document,
			frame: cold.frame,
			scope: requiredElement(cold.document, 'ex-call-stack'),
			label: `author-${width}-standalone`,
		});
	}
}
