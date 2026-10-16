import { afterEach, expect, test } from 'vitest';
import { commands, page, userEvent } from 'vitest/browser';

import notesSource from '../../experiments/agent-written/navigation-markdown-fix/notes.md?raw';
import { fixtureLocator, insideViewport, openFixture, requiredElement } from './fixture-page.ts';

afterEach(async (): Promise<void> => {
	document.body.replaceChildren();
	await page.viewport(1600, 1000);
});

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

test('presentation menu shows descriptions and switches the visible explainer and outline', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('navigation');
	await expect
		.element(frame.getByRole('heading', { name: 'Session ownership', exact: true }))
		.toBeVisible();
	await frame.getByRole('button', { name: 'Browse presentations', exact: true }).click();
	const menu = frame.getByRole('navigation', { name: 'Presentations', exact: true });
	await expect
		.element(menu.getByText('Who owns a session and how reconnect works.', { exact: true }))
		.toBeVisible();
	await expect
		.element(menu.getByText('The proposed path and its two delivery phases.', { exact: true }))
		.toBeVisible();
	await menu.getByRole('link', { name: /Implementation plan/ }).click();
	await expect
		.element(frame.getByRole('heading', { name: 'Implementation plan', exact: true }))
		.toBeVisible();
	await expect
		.element(
			frame.getByRole('heading', {
				name: 'Session architecture',
				exact: true,
				includeHidden: true,
			}),
		)
		.not.toBeVisible();
	await expect
		.element(frame.getByRole('heading', { name: 'Review', exact: true, includeHidden: true }))
		.not.toBeVisible();
	await expect
		.poll((): boolean => {
			const outline = requiredElement(fixture, '.ex-heading-panel');
			return (
				(outline.textContent ?? '').includes('Phases') &&
				!(outline.textContent ?? '').includes('Session ownership')
			);
		})
		.toBe(true);
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

test('visual sidebar copies retain main content and fullscreen restores content and focus', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('layout');
	const diagram = requiredElement(fixture, '#full-width-diagram');
	diagram.scrollIntoView();
	await expect
		.poll((): string => diagram.querySelector('svg')?.textContent ?? '')
		.toContain('Right endpoint');
	await frame.getByRole('heading', { name: 'Full-width diagram', exact: true }).hover();
	await fixtureLocator(frame, diagram).hover();
	await frame
		.getByRole('button', { name: 'Open Full-width diagram in a sidebar', exact: true })
		.click();
	await expect
		.element(frame.getByRole('complementary', { name: 'Full-width diagram', exact: true }))
		.toBeVisible();
	expect(diagram.querySelector('svg')?.textContent).toContain('Left endpoint');
	expect(diagram.closest('ex-document')).not.toBeNull();
	expect(insideViewport(diagram)).toBe(true);
	expect(requiredElement(fixture, '.ex-drawer svg').textContent).toContain('Right endpoint');
	expect(diagram.getBoundingClientRect().height).toBeGreaterThan(30);
	await frame.getByRole('button', { name: 'Close', exact: true }).click();
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
	expect(diagram.querySelector('svg')?.textContent).toContain('Right endpoint');
	expect(fixture.activeElement?.getAttribute('aria-label')).toBe('Expand Full-width diagram');
	await expand.click();
	await frame.getByRole('dialog').getByRole('button', { name: 'Close', exact: true }).click();
	expect(diagram.querySelector('svg')?.textContent).toContain('Left endpoint');
});

test('inspection modifier wheel zooms and modifier drag pans while ordinary wheel keeps scrolling', async (): Promise<void> => {
	await page.viewport(1600, 650);
	const { document: fixture, frame } = await openFixture('layout');
	const canvas = requiredElement(fixture, '#gesture-canvas');
	canvas.scrollIntoView();
	await fixtureLocator(frame, canvas).hover();
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
