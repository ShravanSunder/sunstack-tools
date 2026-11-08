import { expect } from 'vitest';
import { commands, page, userEvent } from 'vitest/browser';

import { collectionSchema } from '../../../../packages/demistify/src/design-documents/collection.ts';
import { browserScreenshotPath, writeBrowserEvidence } from './browser-evidence.ts';
import { fixtureLocator, openFixture, requiredElement } from './fixture-page.ts';

function expectNativeHit(fixture: Document, element: Element): void {
	const bounds = element.getBoundingClientRect();
	const hit = fixture.elementFromPoint(
		bounds.left + bounds.width / 2,
		bounds.top + bounds.height / 2,
	);
	expect(hit === element || element.contains(hit), `Uncovered ${element.textContent}`).toBe(true);
}

/** Enlargement preserves generated Help while normal authored disclosures retain reveal/restore. */
export async function captureReaderInspectorHelp(props: {
	readonly width: number;
	readonly theme: string;
	readonly motion: string;
}): Promise<void> {
	await page.viewport(props.width, props.width === 390 ? 844 : 1000);
	await commands.setReducedMotion(props.motion === 'reduce' ? 'reduce' : 'no-preference');
	const { document: fixture, frame } = await openFixture(
		'document-browsers',
		'',
		'public-style/inspector-controls.html?need=U2&stage=program-design',
	);
	fixture.documentElement.dataset['theme'] = props.theme;
	await expect
		.poll(() => fixture.querySelector('.ex-design-stage-record ex-mermaid svg'))
		.not.toBeNull();
	const reader = requiredElement(fixture, 'ex-requirements');
	const wrapper = requiredElement(fixture, 'ex-zoom:has(> ex-requirements)');
	const expand = requiredElement(
		wrapper,
		':scope > .ex-inspect-controls button:not(.ex-inspect-sidebar)',
	);
	const help = reader.querySelector<HTMLDetailsElement>('details.ex-design-help');
	const authored = reader.querySelector<HTMLDetailsElement>('.ex-design-stage-record details');
	if (!help || !authored) throw new Error('Generated Help and authored disclosure required.');
	expect(help.open).toBe(false);
	expect(authored.open).toBe(false);
	await fixtureLocator(frame, requiredElement(reader, '.ex-design-document-subject')).hover();
	expand.scrollIntoView({ block: 'center', behavior: 'instant' });
	await fixtureLocator(frame, expand).click();
	const dialog = requiredElement(fixture, '.ex-inspect-dialog');
	await expect.poll(() => dialog.matches(':modal')).toBe(true);
	await Promise.all(
		fixture.getAnimations().map((animation) => animation.finished.catch(() => undefined)),
	);
	expect(requiredElement(dialog, 'ex-requirements')).toBe(reader);
	expect(help.open, 'Closed generated Help must not be forced over reader controls').toBe(false);
	expect(authored.open).toBe(true);
	const title = requiredElement(reader, '.ex-design-document-subject');
	title.scrollIntoView({ block: 'center', behavior: 'instant' });
	expectNativeHit(fixture, title);
	await ['Overview', 'Specification', 'Program Design', 'Verification'].reduce(
		async (previous, name): Promise<void> => {
			await previous;
			const button = requiredElement(
				reader,
				`.ex-design-document-switcher button[data-document="${name === 'Program Design' ? 'program-design' : name === 'Overview' ? 'requirement' : name.toLowerCase()}"]`,
			);
			button.scrollIntoView({ block: 'nearest', behavior: 'instant' });
			expectNativeHit(fixture, button);
		},
		Promise.resolve(),
	);
	await page.screenshot({
		path: browserScreenshotPath(`reader-help-${props.width}-${props.theme}-${props.motion}.png`),
	});
	requiredElement(help, 'summary').scrollIntoView({ block: 'center', behavior: 'instant' });
	await fixtureLocator(frame, requiredElement(help, 'summary')).click();
	expect(help.open).toBe(true);
	await userEvent.keyboard('{Enter}');
	expect(help.open).toBe(false);
	await frame
		.getByRole('dialog', { name: 'User requirements', exact: true })
		.getByRole('button', { name: 'Close', exact: true })
		.click();
	await expect.poll(() => fixture.querySelector('.ex-inspect-dialog')).toBeNull();
	expect(wrapper.querySelector('ex-requirements')).toBe(reader);
	expect(authored.open).toBe(false);
	expect(help.open).toBe(false);
	await expect.poll(() => fixture.activeElement).toBe(expand);
	{
		await fixtureLocator(frame, requiredElement(help, 'summary')).click();
		expect(help.open).toBe(true);
		const hoverTarget = requiredElement(
			reader,
			props.width === 1600 ? '.ex-design-requirement-list' : '.ex-design-document-subject',
		);
		hoverTarget.scrollIntoView({
			block: 'center',
			behavior: 'instant',
		});
		await fixtureLocator(frame, hoverTarget).hover();
		expand.scrollIntoView({ block: 'center', behavior: 'instant' });
		await fixtureLocator(frame, expand).click();
		expect(help.open).toBe(true);
		await frame
			.getByRole('dialog', { name: 'User requirements', exact: true })
			.getByRole('button', { name: 'Close', exact: true })
			.click();
		expect(help.open).toBe(true);
		await fixtureLocator(frame, requiredElement(help, 'summary')).click();
	}
	await frame.getByRole('button', { name: 'Specification', exact: true }).click();
	expect(
		requiredElement(reader, '.ex-design-document-switcher button[aria-pressed="true"]').textContent,
	).toContain('Specification');
	const unrelated = requiredElement(fixture, 'ex-zoom[label="Unrelated disclosure"]');
	const ordinary = unrelated.querySelector<HTMLDetailsElement>('details');
	if (!ordinary) throw new Error('Missing ordinary authored disclosure.');
	expect(ordinary.open).toBe(false);
	await fixtureLocator(frame, unrelated).hover();
	await frame.getByRole('button', { name: 'Expand Unrelated disclosure', exact: true }).click();
	expect(ordinary.open).toBe(true);
	await frame
		.getByRole('dialog', { name: 'Unrelated disclosure', exact: true })
		.getByRole('button', { name: 'Close', exact: true })
		.click();
	expect(ordinary.open).toBe(false);
	if (props.width === 1600) {
		document.body.replaceChildren();
		const copied = await openFixture(
			'document-browsers',
			'',
			'public-style/inspector-copy.html?need=U2&stage=program-design',
		);
		await expect.poll(() => copied.document.querySelector('details.ex-design-help')).not.toBeNull();
		const original = requiredElement(copied.document, 'ex-requirements');
		await fixtureLocator(
			copied.frame,
			requiredElement(original, '.ex-design-document-subject'),
		).hover();
		await copied.frame
			.getByRole('button', { name: 'Open User requirements in a sidebar', exact: true })
			.click();
		const drawer = requiredElement(copied.document, '.ex-drawer');
		await expect.poll(() => drawer.querySelector('ex-requirements[aria-busy]')).toBeNull();
		const copyHelp = drawer.querySelector<HTMLDetailsElement>('details.ex-design-help');
		if (!copyHelp) throw new Error('Missing copied reader Help.');
		expect(requiredElement(drawer, 'ex-requirements')).not.toBe(original);
		expect(copyHelp.open).toBe(false);
		await fixtureLocator(copied.frame, requiredElement(copyHelp, 'summary')).click();
		expect(copyHelp.open).toBe(true);
		await copied.frame.getByRole('button', { name: 'Close', exact: true }).click();
		expect(copied.document.querySelector('ex-requirements')).toBe(original);
	}
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
		await writeBrowserEvidence(
			`${label}-surface-${index}-wheel-diagnostic.json`,
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
		);
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
	await writeBrowserEvidence(
		`${label}-reachability.json`,
		JSON.stringify(
			{ label, surrounding, observations, pageWidth: fixture.documentElement.scrollWidth },
			null,
			2,
		),
	);
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
			await writeBrowserEvidence(
				`${label}-${stack.id}-${frameId}-detail.json`,
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
			);
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
				await writeBrowserEvidence(
					`author-${width}-${needId}-${stage}.json`,
					JSON.stringify({ width, needId, stage, scripts, styles, observation }, null, 2),
				);
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
		await writeBrowserEvidence(
			`author-${width}-${needId}-related.json`,
			JSON.stringify({ width, needId, view: 'related', observation: related }, null, 2),
		);
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
		await writeBrowserEvidence(
			`author-${width}-${needId}-calls.json`,
			JSON.stringify({ width, needId, view: 'calls', observation: calls }, null, 2),
		);
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
	await writeBrowserEvidence(
		`author-${width}-resources.json`,
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
	);
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
		await writeBrowserEvidence(
			`author-${width}-standalone-entry.json`,
			JSON.stringify(
				{
					url: cold.document.URL,
					entry: await observeReader(requiredElement(cold.document, 'ex-call-stack')),
				},
				null,
				2,
			),
		);
		await page.screenshot({ path: browserScreenshotPath(`author-${width}-standalone-entry.png`) });
		await captureEveryCall({
			fixture: cold.document,
			frame: cold.frame,
			scope: requiredElement(cold.document, 'ex-call-stack'),
			label: `author-${width}-standalone`,
		});
	}
}
