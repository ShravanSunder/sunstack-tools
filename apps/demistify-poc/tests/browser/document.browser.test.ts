import { afterEach, expect, test } from 'vitest';
import { commands, page, userEvent } from 'vitest/browser';

import notesSource from '../../experiments/agent-written/navigation-markdown-fix/notes.md?raw';
import { browserScreenshotPath } from './browser-evidence.ts';
import { fixtureLocator, insideViewport, openFixture, requiredElement } from './fixture-page.ts';

afterEach(async (): Promise<void> => {
	document.body.replaceChildren();
	await page.viewport(1600, 1000);
});

test.each([
	[1600, '#inspection-picture', 'img'],
	[390, '#inspection-picture', 'img'],
])(
	'visual click at %ipx opens %s fullscreen and restores its original',
	async (width, ownerSelector, visualSelector): Promise<void> => {
		await page.viewport(width, 900);
		const { document: fixture, frame } = await openFixture('layout');
		const owner = requiredElement(fixture, ownerSelector);
		owner.scrollIntoView();
		await expect.poll((): boolean => !!owner.querySelector(visualSelector)).toBe(true);
		const original = requiredElement(owner, visualSelector);
		await fixtureLocator(frame, original).click();
		await expect.element(frame.getByRole('dialog')).toBeVisible();
		expect(original.closest('dialog')).not.toBeNull();
		expect(fixture.querySelector('.ex-drawer')).toBeNull();
		await frame.getByRole('dialog').getByRole('button', { name: 'Close', exact: true }).click();
		expect(owner.contains(original)).toBe(true);
		expect(original.closest('ex-document')).not.toBeNull();
	},
);

async function verifyMarkdownRendering(variant: 'navigation' | 'embedded'): Promise<void> {
	const { document: fixture, frame } = await openFixture(variant);
	await expect
		.element(frame.getByRole('heading', { name: 'Session ownership', exact: true }))
		.toBeVisible();
	await expect
		.element(frame.getByRole('cell', { name: 'Session remains available', exact: true }))
		.toBeVisible();
	await expect
		.element(frame.getByText('The literal <registry> stays prose.', { exact: false }))
		.toBeVisible();
	requiredElement(fixture, '#notes ex-mermaid').scrollIntoView();
	await expect
		.poll(
			(): string =>
				requiredElement(fixture, '#notes').querySelector('ex-mermaid svg')?.textContent ?? '',
		)
		.toContain('Registry');
	const diagram = requiredElement(fixture, '#notes ex-mermaid svg');
	expect(diagram.textContent).toContain('Client');
	expect(diagram.textContent).toContain('Session');
	expect(
		fixture.defaultView?.getComputedStyle(requiredElement(diagram, 'text')).fontFamily,
	).toContain('Source Sans 3');
	expect(diagram.querySelector('path, polyline')?.getBoundingClientRect().width).toBeGreaterThan(0);
	if (variant === 'embedded') {
		const requests = fixture.defaultView?.performance.getEntriesByType('resource') ?? [];
		expect(requests.some((entry): boolean => entry.name.endsWith('/notes.md'))).toBe(false);
		expect((await fetch('/__demistify/embedded/notes.md')).status).toBe(404);
	}
	document.body.replaceChildren();
}

async function verifyMarkdownSource(variant: 'navigation' | 'embedded'): Promise<void> {
	const { document: fixture, frame } = await openFixture(variant);
	const heading = frame.getByRole('heading', { name: 'Session ownership', exact: true });
	await expect.element(heading).toBeVisible();
	await heading.hover();
	await fixtureLocator(
		frame,
		requiredElement(fixture, '#notes button[data-markdown-source]'),
	).click();
	await expect
		.element(frame.getByRole('complementary', { name: 'notes.md', exact: true }))
		.toBeVisible();
	expect(requiredElement(fixture, '.ex-drawer pre code').textContent).toBe(notesSource);
	await expect.element(heading).toBeVisible();
	await expect
		.element(frame.getByRole('cell', { name: 'Session remains available', exact: true }))
		.toBeVisible();
	await frame
		.getByRole('complementary', { name: 'notes.md', exact: true })
		.getByRole('button', { name: 'Close', exact: true })
		.click();
	await expect.element(heading).toBeVisible();
	document.body.replaceChildren();
}

test('external and embedded Markdown render prose, a real table and Mermaid instead of literal source', async (): Promise<void> => {
	await verifyMarkdownRendering('navigation');
	await verifyMarkdownRendering('embedded');
});

test('Show Markdown source displays the exact file while main content stays visible', async (): Promise<void> => {
	await verifyMarkdownSource('navigation');
	await verifyMarkdownSource('embedded');
});

test('presentation menu starts a named path while existing content and outline remain', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('navigation');
	await expect
		.element(frame.getByRole('heading', { name: 'Session ownership', exact: true }))
		.toBeVisible();
	await frame.getByRole('button', { name: 'Browse walkthroughs', exact: true }).click();
	const menu = frame.getByRole('dialog', { name: 'Search documents', exact: true });
	await expect
		.element(menu.getByText('The proposed path and its two delivery phases.', { exact: true }))
		.toBeVisible();
	await menu.getByRole('option', { name: /Implementation plan/ }).click();
	await expect
		.poll((): boolean => requiredElement(fixture, '#plan').classList.contains('ex-route-focus'))
		.toBe(true);
	expect(requiredElement(fixture, '#architecture').hasAttribute('hidden')).toBe(false);
	expect(requiredElement(fixture, '#review').hasAttribute('hidden')).toBe(false);
	await expect
		.element(frame.getByRole('region', { name: 'Walkthrough player', exact: true }))
		.toBeVisible();
});

async function verifyInitialFragment(
	fragment: '#notes--recovery' | '#plan-diagram',
	label: string,
): Promise<void> {
	const { document: fixture, frame } = await openFixture('navigation', fragment);
	if (fragment === '#notes--recovery') {
		await expect.element(frame.getByRole('heading', { name: label, exact: true })).toBeVisible();
		await expect.poll((): boolean => insideViewport(requiredElement(fixture, fragment))).toBe(true);
	} else {
		await expect
			.element(frame.getByRole('tab', { name: 'Diagram', exact: true }))
			.toHaveAttribute('aria-selected', 'true');
		await expect
			.poll(
				(): string =>
					requiredElement(fixture, '#plan-diagram').querySelector('svg')?.textContent ?? '',
			)
			.toContain(label);
		await expect
			.poll((): boolean => insideViewport(requiredElement(fixture, '#plan-diagram svg')))
			.toBe(true);
	}
	document.body.replaceChildren();
}

test('initial fragments reveal the containing presentation and inactive tab before any click', async (): Promise<void> => {
	await verifyInitialFragment('#notes--recovery', 'Recovery');
	await verifyInitialFragment('#plan-diagram', 'Registry');
});

test.each([1600, 390])(
	'Mermaid offers only its fullscreen icon at %ipx and leaves drawing clicks alone',
	async (width): Promise<void> => {
		await page.viewport(width, 900);
		const { document: fixture, frame } = await openFixture('layout');
		const diagram = requiredElement(fixture, '#full-width-diagram');
		diagram.scrollIntoView();
		await expect
			.poll((): string => diagram.querySelector('svg')?.textContent ?? '')
			.toContain('Right endpoint');
		expect(diagram.querySelectorAll('.ex-inspect-trigger').length).toBe(1);
		expect(diagram.querySelector('button[aria-label*="sidebar"]')).toBeNull();
		const original = requiredElement(diagram, ':scope > svg');
		await fixtureLocator(frame, original).click();
		expect(fixture.querySelector('.ex-inspect-dialog, .ex-drawer')).toBeNull();
		expect(diagram.querySelector(':scope > svg')).toBe(original);
		await fixtureLocator(frame, diagram).hover();
		const expand = frame.getByRole('button', { name: 'Expand Full-width diagram', exact: true });
		await expand.click();
		const modal = frame.getByRole('dialog', { name: 'Full-width diagram', exact: true });
		await expect.element(modal).toBeVisible();
		await expect.element(modal.getByText('Left endpoint', { exact: true })).toBeVisible();
		await modal.getByRole('button', { name: 'Zoom in', exact: true }).click();
		await expect.element(modal.getByText('125%', { exact: true })).toBeVisible();
		await userEvent.keyboard('{Escape}');
		await expect.element(modal).not.toBeInTheDocument();
		// Native close hides the dialog before its close event restores the live SVG.
		await expect.poll((): boolean => diagram.querySelector(':scope > svg') === original).toBe(true);
		expect(diagram.querySelector('svg')?.textContent).toContain('Right endpoint');
		expect(fixture.activeElement?.getAttribute('aria-label')).toBe('Expand Full-width diagram');
		await expand.click();
		await frame.getByRole('dialog').getByRole('button', { name: 'Close', exact: true }).click();
		await expect.poll((): boolean => diagram.querySelector(':scope > svg') === original).toBe(true);
		expect(diagram.querySelector('svg')?.textContent).toContain('Left endpoint');
	},
);

test('replacing sidebar content keeps the newly inspected item in view and restores its trigger on close', async (): Promise<void> => {
	await page.viewport(1600, 650);
	const { document: fixture, frame } = await openFixture('layout');
	// Native focus scrolling stays real; immediate scrolling makes a wrong destination deterministic.
	fixture.documentElement.style.scrollBehavior = 'auto';
	const first = requiredElement(fixture, '#reading-code');
	first.scrollIntoView({ block: 'center' });
	await expect.poll((): boolean => !!first.querySelector('code')).toBe(true);
	await fixtureLocator(frame, first).hover();
	const firstButton = frame.getByRole('button', {
		name: 'Open Short command in a sidebar',
		exact: true,
	});
	await firstButton.click();
	await expect
		.element(frame.getByRole('complementary', { name: 'Short command', exact: true }))
		.toBeVisible();

	const second = requiredElement(fixture, '#gesture-canvas');
	second.scrollIntoView({ block: 'center', behavior: 'instant' });
	await fixtureLocator(frame, second).hover();
	const secondButton = frame.getByRole('button', {
		name: 'Open Gesture canvas in a sidebar',
		exact: true,
	});
	await secondButton.click();
	const replacement = frame.getByRole('complementary', { name: 'Gesture canvas', exact: true });
	await expect.element(replacement).toBeVisible();
	expect(second.getBoundingClientRect().top).toBeGreaterThanOrEqual(0);
	expect(second.getBoundingClientRect().top).toBeLessThan(650);
	expect(first.querySelector('code')).not.toBeNull();
	expect(requiredElement(fixture, '.ex-drawer svg').textContent).toContain('Canvas start');
	await replacement.getByRole('button', { name: 'Close', exact: true }).click();
	expect(fixture.activeElement?.getAttribute('aria-label')).toBe(
		'Open Gesture canvas in a sidebar',
	);
	expect(second.closest('ex-document')).not.toBeNull();
});

test('inspection modifier wheel zooms and modifier drag pans while ordinary wheel keeps scrolling', async (): Promise<void> => {
	await page.viewport(1600, 650);
	const { document: fixture, frame } = await openFixture('layout');
	const canvas = requiredElement(fixture, '#gesture-canvas');
	canvas.scrollIntoView({ block: 'center', behavior: 'instant' });
	await fixtureLocator(frame, canvas).hover();
	const sidebarButton = requiredElement(
		canvas,
		'button[aria-label="Open Gesture canvas in a sidebar"]',
	);
	const sidebarButtonBounds = sidebarButton.getBoundingClientRect();
	const toolbarBounds = requiredElement(fixture, '.ex-document-toolbar').getBoundingClientRect();
	expect(sidebarButtonBounds.width).toBeGreaterThan(0);
	expect(sidebarButtonBounds.height).toBeGreaterThan(0);
	const sidebarButtonOverlapsToolbar =
		sidebarButtonBounds.left < toolbarBounds.right &&
		sidebarButtonBounds.right > toolbarBounds.left &&
		sidebarButtonBounds.top < toolbarBounds.bottom &&
		sidebarButtonBounds.bottom > toolbarBounds.top;
	expect(sidebarButtonOverlapsToolbar).toBe(false);
	await frame
		.getByRole('button', { name: 'Open Gesture canvas in a sidebar', exact: true })
		.click();
	const drawer = frame.getByRole('complementary', { name: 'Gesture canvas', exact: true });
	await expect.element(drawer).toBeVisible();
	const viewport = requiredElement(fixture, '.ex-drawer .ex-inspect-body');
	const viewportLocator = fixtureLocator(frame, viewport);
	const magnification = (): string =>
		requiredElement(fixture, '.ex-drawer .ex-inspect-header > span').textContent ?? '';
	const originalWidth = requiredElement(viewport, 'svg').getBoundingClientRect().width;
	await userEvent.keyboard('{Meta>}');
	try {
		await viewportLocator.wheel({ delta: { y: -350 } });
	} finally {
		await userEvent.keyboard('{/Meta}');
	}
	await expect.poll((): number => Number.parseInt(magnification(), 10)).toBeGreaterThan(120);
	expect(requiredElement(viewport, 'svg').getBoundingClientRect().width).toBeGreaterThan(
		originalWidth + 20,
	);
	const zoomedLabel = magnification();
	viewport.scrollTop = 0;
	await viewportLocator.wheel({ delta: { y: 150 } });
	await expect.poll((): number => viewport.scrollTop).toBeGreaterThan(20);
	expect(magnification()).toBe(zoomedLabel);
	viewport.scrollLeft = 0;
	viewport.scrollTop = 0;
	await commands.panInspection();
	await expect.poll((): number => viewport.scrollLeft).toBeGreaterThan(60);
	expect(viewport.scrollTop).toBeGreaterThan(30);
	await drawer.getByRole('button', { name: 'Reset zoom', exact: true }).click();
	expect(magnification()).toBe('100%');
	expect(viewport.scrollLeft).toBe(0);
	expect(viewport.scrollTop).toBe(0);
	await drawer.getByRole('button', { name: 'Close', exact: true }).click();
	expect(canvas.closest('ex-document')).not.toBeNull();
});

test('compact headings menu stays open after hover then click and reaches a Markdown heading', async (): Promise<void> => {
	await page.viewport(390, 900);
	const { document: fixture, frame } = await openFixture('navigation');
	await expect.element(frame.getByRole('heading', { name: 'Recovery', exact: true })).toBeVisible();
	const trigger = frame.getByRole('button', { name: 'Browse page headings', exact: true });
	const menu = frame.getByRole('navigation', {
		name: 'Page navigation',
		exact: true,
		includeHidden: true,
	});
	await trigger.hover();
	await expect.element(menu).toBeVisible();
	await trigger.click();
	await expect.element(menu).toBeVisible();
	await frame.getByRole('button', { name: 'Browse pages', exact: true }).click();
	await expect
		.element(
			frame
				.getByRole('navigation', { name: 'Pages', exact: true })
				.getByRole('link', { name: 'Implementation detail', exact: true }),
		)
		.toBeVisible();
	await userEvent.keyboard('{Escape}');
	await frame.getByRole('button', { name: 'Browse page headings', exact: true }).click();
	await menu.getByRole('link', { name: 'Recovery', exact: true }).click();
	await expect.element(menu).not.toBeVisible();
	await expect
		.poll((): boolean => insideViewport(requiredElement(fixture, '#notes--recovery')))
		.toBe(true);
});

test('Mermaid hover keeps themed connectors visible and their arrow scale stable in both themes', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('navigation');
	const owner = requiredElement(fixture, '#notes');
	owner.scrollIntoView();
	await expect
		.poll((): number => owner.querySelectorAll('ex-mermaid svg polyline[data-from]').length)
		.toBe(2);
	const diagram = requiredElement(owner, 'ex-mermaid svg');
	const registry = requiredElement(diagram, 'g[data-id="Registry"]');
	const edges = Array.from(diagram.querySelectorAll<SVGElement>('polyline[data-from]'));
	const widthBefore = edges.map(
		(edge): string => fixture.defaultView?.getComputedStyle(edge).strokeWidth ?? '',
	);
	const boundsBefore = registry.getBoundingClientRect();
	const verifyTheme = async (): Promise<string> => {
		const sample = fixture.createElement('span');
		sample.style.color = 'var(--accent)';
		fixture.body.append(sample);
		const expectedColor = fixture.defaultView?.getComputedStyle(sample).color;
		sample.remove();
		if (!expectedColor) throw new Error('The page accent has no computed colour.');
		await fixtureLocator(frame, registry).hover();
		await expect
			.poll((): boolean =>
				edges.every(
					(edge): boolean => fixture.defaultView?.getComputedStyle(edge).stroke === expectedColor,
				),
			)
			.toBe(true);
		expect(
			edges.map((edge): string => fixture.defaultView?.getComputedStyle(edge).strokeWidth ?? ''),
		).toEqual(widthBefore);
		expect(Math.abs(registry.getBoundingClientRect().width - boundsBefore.width)).toBeLessThan(1);
		expect(Math.abs(registry.getBoundingClientRect().height - boundsBefore.height)).toBeLessThan(1);
		await frame.getByRole('heading', { name: 'Session architecture', exact: true }).hover();
		await expect.poll((): boolean => diagram.classList.contains('ex-tracing')).toBe(false);
		return expectedColor;
	};
	const firstThemeColor = await verifyTheme();
	await frame.getByRole('button', { name: 'Switch light or dark theme', exact: true }).click();
	expect(await verifyTheme()).not.toBe(firstThemeColor);
});

test('inspection-only Card details are hidden before the runtime loads', async (): Promise<void> => {
	const iframe = document.createElement('iframe');
	iframe.srcdoc = `<!doctype html><html><head><link rel="stylesheet" href="/__demistify/library/theme.css"></head><body>
		<ex-card id="inspection-only"><p>Inline summary.</p><p slot="details">Inspection attachment.</p></ex-card>
		<ex-card id="inline" zoom="none"><p slot="details">Explicitly inline attachment.</p></ex-card>
	</body></html>`;
	document.body.append(iframe);
	await expect
		.poll((): boolean => !!iframe.contentDocument?.querySelector<HTMLLinkElement>('link')?.sheet)
		.toBe(true);
	const fixture = iframe.contentDocument;
	if (!fixture) throw new Error('Card fixture did not load.');

	expect(fixture.defaultView?.customElements.get('ex-card')).toBeUndefined();
	expect(requiredElement(fixture, '#inspection-only > p').getClientRects()).toHaveLength(1);
	expect(
		requiredElement(fixture, '#inspection-only > [slot="details"]').getClientRects(),
	).toHaveLength(0);
	expect(requiredElement(fixture, '#inline > [slot="details"]').getClientRects()).toHaveLength(1);
});

test.each([1400, 390])(
	'inspection-only Card details stay inline-hidden at %i CSS px',
	async (width): Promise<void> => {
		await page.viewport(width, 1000);
		const { document: fixture, frame } = await openFixture('layout');
		const template = fixture.createElement('template');
		template.innerHTML =
			'<ex-card id="inspection-only-card" label="Bounded job" title="Job"><p>Inline summary.</p><ex-markdown slot="details"><script type="text/markdown">**Inspection attachment.**</script></ex-markdown></ex-card>';
		requiredElement(fixture, 'ex-document').append(template.content);
		const card = requiredElement(fixture, '#inspection-only-card');
		const detail = requiredElement(card, '[slot="details"]');
		const local = fixtureLocator(frame, card);
		await expect.element(local.getByText('Inline summary.')).toBeVisible();
		expect(detail.hasAttribute('hidden')).toBe(true);

		await local.getByRole('button', { name: 'More about Bounded job' }).click();
		const inspector = frame.getByRole(width === 390 ? 'dialog' : 'complementary', {
			name: 'Bounded job',
			exact: true,
		});
		await expect.element(inspector.getByText('Inspection attachment.')).toBeVisible();
		const panel = requiredElement(
			fixture,
			width === 390 ? 'dialog.ex-inspect-dialog[open]' : '.ex-drawer[aria-label="Bounded job"]',
		);
		await expect
			.poll((): boolean => panel.getAnimations().every(({ playState }) => playState === 'finished'))
			.toBe(true);
		await page.screenshot({ path: browserScreenshotPath(`card-detail-${width}.png`) });
		await inspector.getByRole('button', { name: 'Close', exact: true }).click();
		await expect
			.poll((): boolean => fixture.documentElement.hasAttribute('data-drawer'))
			.toBe(false);
		expect(card.querySelector('[slot="details"]')).toBe(detail);
		expect(detail.hasAttribute('hidden')).toBe(true);
	},
);

test.each([1400, 390])(
	'untitled Card keeps its bounded canvas usable at %i CSS px',
	async (width): Promise<void> => {
		await page.viewport(width, 1000);
		const { document: fixture } = await openFixture('layout');
		const canvas = requiredElement(fixture, '#gesture-canvas');
		expect(canvas.querySelector(':scope > .ex-card-head')).toBeNull();

		const body = requiredElement(canvas, 'svg');
		const styles = fixture.defaultView?.getComputedStyle(canvas);
		const inset =
			Number.parseFloat(styles?.paddingTop ?? '') + Number.parseFloat(styles?.borderTopWidth ?? '');
		const bounds = canvas.getBoundingClientRect();
		expect(body.getBoundingClientRect().top - bounds.top).toBeCloseTo(inset);
		expect(bounds.height).toBe(240);
		expect(canvas.scrollHeight).toBeGreaterThan(canvas.clientHeight);

		const metadata = fixture.createElement('div');
		metadata.innerHTML =
			'<ex-card title="Named"></ex-card><ex-card tone="done"></ex-card><ex-card icon="code"></ex-card><ex-card tag="Observed"></ex-card><ex-card pick></ex-card>';
		requiredElement(fixture, 'ex-document').append(metadata);
		expect(metadata.querySelectorAll('ex-card > .ex-card-head')).toHaveLength(5);
		expect(metadata.querySelector('strong')?.textContent).toBe('Named');
	},
);

test.each([1400, 390])(
	'the built capabilities layer bands use shared reference styles at %i CSS px',
	async (width): Promise<void> => {
		await page.viewport(width, 1000);
		const { document: fixture } = await openFixture('capabilities', '#layers');
		const bands = fixture.querySelectorAll('#layers + .tree > .band');
		expect(bands).toHaveLength(5);
		const band = requiredElement(fixture, '#layers + .tree > .band');
		const rail = requiredElement(band, ':scope > .lv');
		const body = requiredElement(band, ':scope > .body');
		const fixtureWindow = fixture.defaultView;
		if (!fixtureWindow) throw new Error('Missing fixture window.');
		const bandStyle = fixtureWindow.getComputedStyle(band);
		const railStyle = fixtureWindow.getComputedStyle(rail);
		const railBounds = rail.getBoundingClientRect();
		const bodyBounds = body.getBoundingClientRect();
		expect(bandStyle.display).toBe('grid');
		expect(bandStyle.borderTopWidth).toBe('1px');
		if (width === 1400) {
			expect(railBounds.width).toBe(150);
			expect(railStyle.borderRightWidth).toBe('1px');
			expect(bodyBounds.left).toBeCloseTo(railBounds.right);
			expect(bodyBounds.top).toBeCloseTo(railBounds.top);
		} else {
			expect(railStyle.borderBottomWidth).toBe('1px');
			expect(bodyBounds.left).toBeCloseTo(railBounds.left);
			expect(bodyBounds.top).toBeCloseTo(railBounds.bottom);
		}
		await page.screenshot({ path: browserScreenshotPath(`reference-bands-${width}.png`) });
	},
);
