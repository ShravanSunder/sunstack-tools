import { afterEach, expect, test } from 'vitest';
import { commands, page, userEvent } from 'vitest/browser';

import { fixtureLocator, insideViewport, openFixture, requiredElement } from './fixture-page.ts';

afterEach(async (): Promise<void> => {
	document.body.replaceChildren();
	await page.viewport(1600, 1000);
});

test('clicking the already-selected gallery view retains the image and its pan', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('gallery');
	const gallery = requiredElement(fixture, '#proof-gallery');
	const local = fixtureLocator(frame, gallery);
	await expect
		.poll(
			(): number =>
				gallery.querySelector<HTMLImageElement>('.ex-gallery-preview img')?.naturalWidth ?? 0,
		)
		.toBe(1000);
	await local.getByRole('button', { name: 'Zoom in', exact: true }).click();
	await local.getByRole('button', { name: 'Zoom in', exact: true }).click();
	const image = requiredElement(gallery, '.ex-gallery-preview img');
	const viewport = requiredElement(gallery, '.ex-gallery-preview');
	viewport.scrollLeft = 48;
	viewport.scrollTop = 36;
	const pan = { left: viewport.scrollLeft, top: viewport.scrollTop };
	expect(pan.left + pan.top).toBeGreaterThan(0);
	await local.getByRole('button', { name: 'Review view', exact: true }).click();
	expect(requiredElement(gallery, '.ex-gallery-preview img')).toBe(image);
	expect(viewport.scrollLeft).toBe(pan.left);
	expect(viewport.scrollTop).toBe(pan.top);
});

test.each([1600, 390])(
	'gallery thumbnails can be hidden and restored at %ipx without replacing the selected picture',
	async (width: number): Promise<void> => {
		await page.viewport(width, 1000);
		const { document: fixture, frame } = await openFixture('gallery');
		const gallery = requiredElement(fixture, '#proof-gallery');
		const local = fixtureLocator(frame, gallery);
		await expect
			.element(local.getByRole('button', { name: 'Hide thumbnails', exact: true }))
			.toBeVisible();
		const strip = requiredElement(gallery, '.ex-gallery-strip');
		const image = requiredElement(gallery, '.ex-gallery-preview img');
		const viewer = requiredElement(gallery, '.ex-gallery-viewer');
		const before = viewer.getBoundingClientRect();
		await local.getByRole('button', { name: 'Hide thumbnails', exact: true }).click();
		await expect.element(fixtureLocator(frame, strip)).not.toBeVisible();
		await expect
			.element(local.getByRole('button', { name: 'Show thumbnails', exact: true }))
			.toHaveAttribute('aria-expanded', 'false');
		const after = viewer.getBoundingClientRect();
		expect(width > 700 ? after.width - before.width : after.height - before.height).toBeGreaterThan(
			80,
		);
		expect(requiredElement(gallery, '.ex-gallery-preview img')).toBe(image);
		await local.getByRole('button', { name: 'Grid view', exact: true }).click();
		await local.getByRole('button', { name: 'Review view', exact: true }).click();
		await expect.element(fixtureLocator(frame, strip)).not.toBeVisible();
		const show = local.getByRole('button', { name: 'Show thumbnails', exact: true });
		await show.click();
		await userEvent.keyboard('{Enter}');
		await expect.element(fixtureLocator(frame, strip)).not.toBeVisible();
		await userEvent.keyboard('{Enter}');
		await expect.element(fixtureLocator(frame, strip)).toBeVisible();
		await expect
			.element(local.getByRole('button', { name: 'Hide thumbnails', exact: true }))
			.toHaveAttribute('aria-expanded', 'true');
		expect(fixture.documentElement.scrollWidth).toBeLessThanOrEqual(width);
	},
);

test('cross-page gallery walkthrough retains the chosen variation', async (): Promise<void> => {
	const { frame } = await openFixture('ui', '#cross-journey', 'journey.html');
	const iframeElement = document.querySelector<HTMLIFrameElement>('iframe');
	const selectedImage = (): string =>
		iframeElement?.contentDocument?.querySelector<HTMLImageElement>('.ex-gallery-preview img')
			?.alt ?? '';
	const player = frame.getByRole('region', { name: 'Walkthrough player', exact: true });
	await player.getByRole('button', { name: 'Desktop', exact: true }).click();
	await expect.poll(selectedImage).toBe('One Desktop');
	await player.getByRole('button', { name: 'Next step', exact: true }).click();
	await expect.poll(selectedImage).toBe('Next Desktop');
	await expect
		.element(player.getByRole('button', { name: 'Desktop', exact: true }))
		.toHaveAttribute('aria-pressed', 'true');
	await player.getByRole('button', { name: 'Previous step', exact: true }).click();
	await expect.poll(selectedImage).toBe('One Desktop');
});

test('walkthrough Markdown, subject icons and source-relative links survive a cross-page step', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('ui', '#narration', 'narration.html');
	const expectedLink = new URL('narration.html#summary', fixture.URL).href;
	const player = frame.getByRole('region', { name: 'Walkthrough player', exact: true });
	await expect
		.element(player.getByText('Why there are two MCP levels', { exact: true }))
		.toBeVisible();
	expect(fixture.querySelectorAll('.ex-route-narration li')).toHaveLength(2);
	expect(fixture.querySelector('.ex-route-narration code')?.textContent).toBe('SessionId');
	expect(fixture.querySelector('.ex-route-lead > .ph-path')).not.toBeNull();
	expect(fixture.querySelector('.ex-route-narration h1')).toBeNull();
	const titleStyle = fixture.defaultView?.getComputedStyle(
		requiredElement(fixture, '.ex-route-controls > strong'),
	);
	const narrationStyle = fixture.defaultView?.getComputedStyle(
		requiredElement(fixture, '.ex-route-narration p'),
	);
	expect(Number.parseFloat(titleStyle?.fontSize ?? '0')).toBeGreaterThan(
		Number.parseFloat(narrationStyle?.fontSize ?? '0'),
	);
	await player.getByRole('button', { name: 'Next step', exact: true }).click();
	await expect
		.element(frame.getByRole('heading', { name: 'Narration evidence', exact: true }))
		.toBeVisible();
	await expect.element(player.getByText('Inspect the evidence', { exact: true })).toBeVisible();
	await expect
		.element(player.getByRole('link', { name: 'original explanation', exact: true }))
		.toHaveAttribute('href', expectedLink);
	const nextDocument = document.querySelector<HTMLIFrameElement>('iframe')?.contentDocument;
	expect(nextDocument?.querySelector('.ex-route-lead > .ph-code')).not.toBeNull();
	await player.getByRole('button', { name: 'Previous step', exact: true }).click();
	await expect
		.element(player.getByText('Why there are two MCP levels', { exact: true }))
		.toBeVisible();
});

test('cross-page walkthrough arrives at a deep stop without an animated page sweep', async (): Promise<void> => {
	const { frame } = await openFixture('ui', '#narration', 'narration.html');
	const iframe = document.querySelector<HTMLIFrameElement>('iframe');
	if (!iframe) throw new Error('Missing walkthrough frame.');
	const readings: { top: number; scroll: number; height: number; player: number; fonts: string }[] =
		[];
	const landing = new Promise<readonly DOMRect[]>((resolve, reject): void => {
		iframe.addEventListener(
			'load',
			(): void => {
				const destination = iframe.contentDocument;
				const view = destination?.defaultView;
				if (!destination || !view) {
					reject(new Error('Destination did not load.'));
					return;
				}
				const target = requiredElement(destination, '#finish');
				const recordLanding = (): void => {
					readings.push({
						top: target.getBoundingClientRect().top,
						scroll: view.scrollY,
						height: destination.documentElement.scrollHeight,
						player:
							destination.querySelector('.ex-route-player')?.getBoundingClientRect().height ?? 0,
						fonts: destination.fonts.status,
					});
				};
				view.requestAnimationFrame((): void => {
					recordLanding();
					const first = target.getBoundingClientRect();
					view.requestAnimationFrame((): void => {
						recordLanding();
						resolve([first, target.getBoundingClientRect()]);
					});
				});
			},
			{ once: true },
		);
	});
	await frame.getByRole('button', { name: 'Next step', exact: true }).click();
	const [first, second] = await landing;
	if (!first || !second) throw new Error('No destination frames observed.');
	expect(first.top).toBeGreaterThanOrEqual(0);
	expect(first.bottom).toBeLessThan(1000);
	expect(Math.abs(second.top - first.top), JSON.stringify(readings)).toBeLessThanOrEqual(1);
	await expect.element(frame.getByText('Inspect the evidence', { exact: true })).toBeVisible();
});

test('presentation crosses files, returns to its home and reports an unavailable stop visibly', async (): Promise<void> => {
	const { frame } = await openFixture('gallery', '#cross-page', 'navigation.html');
	await expect
		.element(
			frame
				.getByRole('region', { name: 'Walkthrough player', exact: true })
				.getByText('The shared decision.', { exact: true }),
		)
		.toBeVisible();
	await frame.getByRole('button', { name: 'Next step', exact: true }).click();
	await expect
		.element(frame.getByRole('heading', { name: 'Handoff evidence', exact: true }))
		.toBeVisible();
	await expect.element(frame.getByText('The second file.', { exact: true })).toBeVisible();
	await frame.getByRole('button', { name: 'Next step', exact: true }).click();
	await expect
		.poll(
			(): string =>
				document
					.querySelector<HTMLIFrameElement>('iframe')
					?.contentDocument?.querySelector('.ex-route-narration')?.textContent ?? '',
			{ timeout: 8000 },
		)
		.toBe('Target unavailable: unavailable-target');
	await expect
		.element(frame.getByRole('heading', { name: 'Presentations and deep links', exact: true }))
		.toBeVisible();
});

test('screenshot modal dismisses through backdrop and Escape without losing main selection', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('gallery');
	const gallery = requiredElement(fixture, '#proof-gallery');
	const local = fixtureLocator(frame, gallery);
	const expand = local.getByRole('button', { name: 'View screenshot fullscreen', exact: true });
	await expand.click();
	const dialog = frame.getByRole('dialog', { name: 'Home Mobile', exact: true });
	await expect.element(dialog).toBeVisible();
	await commands.clickInspectionBackdrop();
	await expect.element(dialog).not.toBeInTheDocument();
	expect(fixture.activeElement?.getAttribute('aria-label')).toBe('View screenshot fullscreen');
	await expand.click();
	await userEvent.keyboard('{Escape}');
	await expect.element(dialog).not.toBeInTheDocument();
	await expect.element(local.getByRole('img', { name: 'Home Mobile', exact: true })).toBeVisible();
});

test('narrow gallery contains its toolbar and scrolling thumbnails within the page', async (): Promise<void> => {
	await page.viewport(390, 900);
	const { document: fixture, frame } = await openFixture('gallery');
	const gallery = requiredElement(fixture, '#proof-gallery');
	await expect
		.poll(
			(): number =>
				gallery.querySelector<HTMLImageElement>('.ex-gallery-preview img')?.naturalWidth ?? 0,
		)
		.toBe(1000);
	expect(fixture.documentElement.scrollWidth).toBeLessThanOrEqual(391);
	const strip = requiredElement(gallery, '.ex-gallery-strip');
	expect(strip.scrollWidth).toBeGreaterThan(strip.clientWidth);
	expect(fixture.defaultView?.getComputedStyle(strip).overflowX).toBe('auto');
	await expect
		.element(
			frame.getByRole('button', { name: 'Browse page headings', exact: true, includeHidden: true }),
		)
		.not.toBeVisible();
});

test('gallery switches between review and grid without changing selection or another island', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('gallery');
	const gallery = requiredElement(fixture, '#proof-gallery');
	const local = fixtureLocator(frame, gallery);
	await expect.element(local.getByRole('button', { name: 'Grid view', exact: true })).toBeVisible();
	await local.getByRole('button', { name: 'Grid view', exact: true }).click();
	await expect
		.element(local.getByRole('button', { name: 'Grid view', exact: true }))
		.toHaveAttribute('aria-pressed', 'true');
	expect(gallery.querySelectorAll('.ex-gallery-grid figure')).toHaveLength(4);
	expect(requiredElement(fixture, '#second-gallery').getAttribute('view')).toBe('review');
	await local.getByRole('button', { name: 'Review view', exact: true }).click();
	await expect.element(local.getByRole('img', { name: 'Home Mobile', exact: true })).toBeVisible();
});

test.each(['review', 'grid'])(
	'gallery %s image opens fullscreen without a screenshot sidebar action',
	async (view: string): Promise<void> => {
		const { document: fixture, frame } = await openFixture('gallery');
		const gallery = requiredElement(fixture, '#proof-gallery');
		const local = fixtureLocator(frame, gallery);
		await expect
			.element(local.getByRole('button', { name: 'Open screenshot in sidebar', exact: true }))
			.not.toBeInTheDocument();
		if (view === 'grid') {
			await local.getByRole('button', { name: 'Grid view', exact: true }).click();
			await local.getByRole('button', { name: 'View Home Mobile fullscreen', exact: true }).click();
		} else {
			await local.getByRole('img', { name: 'Home Mobile', exact: true }).click();
		}
		const dialog = frame.getByRole('dialog', { name: 'Home Mobile', exact: true });
		await expect.element(dialog).toBeVisible();
		await expect
			.element(dialog.getByRole('img', { name: 'Home Mobile', exact: true }))
			.toBeVisible();
		await expect.element(dialog.getByText('SessionId', { exact: true })).toBeVisible();
		expect(fixture.querySelector('.ex-drawer')).toBeNull();
		await userEvent.keyboard('{Escape}');
		await expect.element(dialog).not.toBeInTheDocument();
		expect(gallery.getAttribute('view')).toBe(view);
		expect(
			requiredElement(gallery, '[data-capture-id="home-mobile"]').getAttribute('aria-current'),
		).toBe('true');
	},
);

test('gallery arrows change screens and same-screen variations from body focus, while search keeps its keys', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('gallery');
	const gallery = requiredElement(fixture, '#proof-gallery');
	const local = fixtureLocator(frame, gallery);
	await frame.getByRole('heading', { name: 'Integrated component review', exact: true }).click();
	await userEvent.keyboard('{ArrowDown}');
	await expect.element(local.getByRole('img', { name: 'Home Laptop', exact: true })).toBeVisible();
	await userEvent.keyboard('{ArrowRight}');
	await expect
		.element(local.getByRole('img', { name: 'Settings Laptop', exact: true }))
		.toBeVisible();
	await userEvent.keyboard('{ArrowUp}');
	await expect
		.element(local.getByRole('img', { name: 'Settings Mobile', exact: true }))
		.toBeVisible();
	const search = local.getByRole('searchbox', { name: 'Search screenshots', exact: true });
	await search.fill('Settings');
	await userEvent.keyboard('{ArrowLeft}');
	await expect
		.element(local.getByRole('img', { name: 'Settings Mobile', exact: true }))
		.toBeVisible();
	expect(requiredElement(fixture, '#second-gallery .ex-gallery-count').textContent).toBe(
		'4 screenshots',
	);
});

test.each(['thumbnail', 'preview'])(
	'gallery keeps keyboard navigation after replacing the focused %s',
	async (origin): Promise<void> => {
		const { document: fixture, frame } = await openFixture('gallery');
		const gallery = requiredElement(fixture, '#proof-gallery');
		const local = fixtureLocator(frame, gallery);
		if (origin === 'thumbnail')
			await local.getByRole('button', { name: 'Home Mobile', exact: true }).click();
		else {
			const preview = gallery.querySelector<HTMLElement>('.ex-gallery-preview');
			if (!preview) throw new Error('Screenshot viewport is missing.');
			preview.focus();
		}
		await userEvent.keyboard('{ArrowDown}{ArrowUp}{ArrowRight}{ArrowDown}');
		await expect
			.element(local.getByRole('img', { name: 'Settings Laptop', exact: true }))
			.toBeVisible();
		expect(gallery.contains(fixture.activeElement)).toBe(true);
		await userEvent.keyboard('{ArrowLeft}{ArrowUp}');
		await expect
			.element(local.getByRole('img', { name: 'Home Mobile', exact: true }))
			.toBeVisible();
		expect(gallery.contains(fixture.activeElement)).toBe(true);
	},
);

test.each([
	{ width: 1600, name: 'Filter screenshots' },
	{ width: 1600, name: 'Keyboard shortcuts' },
	{ width: 390, name: 'Filter screenshots' },
	{ width: 390, name: 'Keyboard shortcuts' },
])(
	'gallery $name stays beside its invoker at $width px',
	async ({ width, name }): Promise<void> => {
		await page.viewport(width, 900);
		const { document: fixture, frame } = await openFixture('gallery');
		const gallery = requiredElement(fixture, '#proof-gallery');
		const local = fixtureLocator(frame, gallery);
		const invoker = local.getByRole('button', { name, exact: true });
		await invoker.click();
		const panel = requiredElement(gallery, ':popover-open');
		const button = requiredElement(gallery, `button[aria-label="${name}"]`);
		await expect
			.poll((): boolean => {
				const popup = panel.getBoundingClientRect();
				const anchor = button.getBoundingClientRect();
				return (
					Math.min(Math.abs(popup.top - anchor.bottom), Math.abs(anchor.top - popup.bottom)) <= 12
				);
			})
			.toBe(true);
		expect(insideViewport(panel)).toBe(true);
		expect(panel.getBoundingClientRect().right).toBeLessThanOrEqual(
			fixture.documentElement.clientWidth,
		);
		await userEvent.keyboard('{Escape}');
	},
);

test('200ms settled filtering retains the exact selected image node and Fill sizing', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('gallery');
	const gallery = requiredElement(fixture, '#proof-gallery');
	const local = fixtureLocator(frame, gallery);
	await expect
		.poll(
			(): number =>
				gallery.querySelector<HTMLImageElement>('.ex-gallery-preview img')?.naturalWidth ?? 0,
		)
		.toBe(1000);
	await local.getByRole('button', { name: 'Fill screenshot', exact: true }).click();
	const image = requiredElement(gallery, '.ex-gallery-preview img');
	const filledWidth = image.getBoundingClientRect().width;
	let lastInput = 0;
	let settledAfter = 0;
	gallery.addEventListener(
		'input',
		(): void => {
			lastInput = fixture.defaultView?.performance.now() ?? 0;
		},
		{ capture: true },
	);
	const observer = new MutationObserver((): void => {
		if (gallery.querySelectorAll('.ex-gallery-thumbnail').length === 1)
			settledAfter = (fixture.defaultView?.performance.now() ?? 0) - lastInput;
	});
	observer.observe(requiredElement(gallery, '.ex-gallery-strip'), { childList: true });
	try {
		const search = local.getByRole('searchbox', { name: 'Search screenshots', exact: true });
		await search.fill('Home');
		await search.fill('Home Mobile');
		await expect
			.poll((): number => gallery.querySelectorAll('.ex-gallery-thumbnail').length)
			.toBe(1);
		expect(settledAfter).toBeGreaterThanOrEqual(199);
		expect(requiredElement(gallery, '.ex-gallery-preview img')).toBe(image);
		expect(
			requiredElement(gallery, '.ex-gallery-preview img').getBoundingClientRect().width,
		).toBeCloseTo(filledWidth, 0);
		expect(gallery.querySelector('.ex-gallery-zoom')).toBeNull();
		expect(requiredElement(gallery, '[aria-label="Image zoom"]').textContent).not.toContain('%');
	} finally {
		observer.disconnect();
	}
});

test('cold presentation and duplicate diagram node fragments resolve to the intended owner', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture(
		'gallery',
		'#walkthrough',
		'navigation.html',
	);
	await expect
		.poll((): boolean =>
			requiredElement(fixture, '#requirements--required-behavior').classList.contains(
				'ex-route-focus',
			),
		)
		.toBe(true);
	await frame.getByRole('button', { name: 'Next step', exact: true }).click();
	await expect
		.poll(
			(): string => fixture.querySelector('#after .ex-route-focus')?.getAttribute('data-id') ?? '',
		)
		.toBe('Registry');
	expect(fixture.querySelector('#before .ex-route-focus')).toBeNull();
	await expect
		.element(fixtureLocator(frame, requiredElement(fixture, '.ex-presentation-marker')))
		.toBeVisible();
	expect(
		requiredElement(fixture, '.ex-presentation-marker').getBoundingClientRect().right,
	).toBeLessThan(requiredElement(fixture, '#after').getBoundingClientRect().left);
	await frame.getByRole('button', { name: 'Next step', exact: true }).click();
	await expect
		.poll((): boolean => requiredElement(fixture, '#lifetime').classList.contains('ex-route-focus'))
		.toBe(true);
	fixture.defaultView?.history.back();
	await expect
		.poll(
			(): string => fixture.querySelector('#after .ex-route-focus')?.getAttribute('data-id') ?? '',
		)
		.toBe('Registry');
	fixture.defaultView?.history.forward();
	await expect
		.poll((): boolean => requiredElement(fixture, '#lifetime').classList.contains('ex-route-focus'))
		.toBe(true);
});

test('file Markdown renders tasks, literal code and relative images; tree links and preformatted text retain their roles', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('gallery', '', 'navigation.html');
	const notes = requiredElement(fixture, '#requirements');
	await expect
		.element(frame.getByRole('cell', { name: 'Session lookup', exact: true }))
		.toBeVisible();
	expect(notes.querySelectorAll('input[type="checkbox"]')).toHaveLength(4);
	const tasks = notes.querySelectorAll(
		'li:has(> input[type="checkbox"]), li:has(> p > input[type="checkbox"])',
	);
	expect(tasks).toHaveLength(4);
	for (const task of tasks)
		expect(fixture.defaultView?.getComputedStyle(task).listStyleType).toBe('none');
	expect(notes.querySelector('pre code')?.textContent).toContain('count < 3');
	await expect
		.poll((): number => notes.querySelector<HTMLImageElement>('img')?.naturalWidth ?? 0)
		.toBe(1000);
	const tree = requiredElement(fixture, 'ex-tree');
	await expect.poll((): number => tree.querySelectorAll('.ex-tree-go').length).toBe(3);
	expect(tree.querySelector('.ex-tree-label a')).toBeNull();
	await frame.getByRole('link', { name: 'Go to Registry node', exact: true }).click();
	await expect.poll((): string => fixture.location.hash).toBe('#after--Registry');
	await expect
		.poll((): number => requiredElement(fixture, '#after--Registry').getBoundingClientRect().width)
		.toBeGreaterThan(0);
	expect(requiredElement(fixture, '#terminal-layout pre').textContent).toBe(
		'┌─ Sessions ──────┐\n│ > review       │\n└────────────────┘',
	);
	const outline = requiredElement(fixture, '.ex-heading-outline');
	expect(outline.scrollHeight).toBeGreaterThan(outline.clientHeight);
	expect(fixture.defaultView?.getComputedStyle(outline).overflowY).toBe('auto');
});

test('cold Markdown fence node links reveal the lazy diagram before it renders', async (): Promise<void> => {
	const { document: fixture } = await openFixture(
		'gallery',
		'#requirements--diagram-1--Registry',
		'navigation.html',
	);
	await expect
		.poll(
			(): number =>
				fixture.querySelector('#requirements--diagram-1--Registry')?.getBoundingClientRect()
					.width ?? 0,
			{ timeout: 8000 },
		)
		.toBeGreaterThan(0);
	const target = requiredElement(fixture, '#requirements--diagram-1--Registry');
	expect(target.getAttribute('data-id')).toBe('Registry');
	await expect
		.poll((): boolean => {
			const bounds = target.getBoundingClientRect();
			return bounds.top >= 0 && bounds.bottom <= (fixture.defaultView?.innerHeight ?? 0);
		})
		.toBe(true);
});

test('thumbnail image clicks select a capture without opening an inspection panel', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('gallery');
	const gallery = requiredElement(fixture, '#proof-gallery');
	const local = fixtureLocator(frame, gallery);
	await fixtureLocator(
		frame,
		requiredElement(gallery, '[data-capture-id="settings-laptop"] img'),
	).click();
	await expect
		.element(local.getByRole('img', { name: 'Settings Laptop', exact: true }))
		.toBeVisible();
	expect(fixture.querySelector('.ex-drawer')).toBeNull();
});

test('choosing a thumbnail does not discard a pending search update', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('gallery');
	const gallery = requiredElement(fixture, '#proof-gallery');
	const local = fixtureLocator(frame, gallery);
	let inputAt = 0;
	let clickAfter = Number.POSITIVE_INFINITY;
	gallery.addEventListener('input', (): void => {
		inputAt = fixture.defaultView?.performance.now() ?? 0;
	});
	gallery.addEventListener(
		'click',
		(): void => {
			clickAfter = (fixture.defaultView?.performance.now() ?? 0) - inputAt;
		},
		{ once: true, capture: true },
	);
	await local.getByRole('searchbox', { name: 'Search screenshots', exact: true }).fill('Settings');
	await local.getByRole('button', { name: 'Settings Laptop', exact: true }).click();
	expect(clickAfter).toBeLessThan(200);
	await expect
		.poll((): string => requiredElement(gallery, '.ex-gallery-count').textContent ?? '')
		.toBe('2 screenshots');
	expect(gallery.querySelectorAll('.ex-gallery-thumbnail')).toHaveLength(2);
	await expect
		.element(local.getByRole('img', { name: 'Settings Laptop', exact: true }))
		.toBeVisible();
});

test('template-backed literal text preserves closing-script content in main and sidebar', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture(
		'gallery',
		'#literal-markup',
		'navigation.html',
	);
	const drawing = requiredElement(fixture, '#literal-markup');
	const expected = 'first line\n</script>\n<widget>& value\nlast line';
	await expect.poll((): string => drawing.querySelector('pre')?.textContent ?? '').toBe(expected);
	await userEvent.hover(fixtureLocator(frame, drawing));
	await fixtureLocator(frame, drawing)
		.getByRole('button', { name: 'Open Literal markup in a sidebar', exact: true })
		.click();
	const panel = frame.getByRole('complementary', { name: 'Literal markup', exact: true });
	await expect.element(panel).toBeVisible();
	expect(requiredElement(fixture, '.ex-drawer pre').textContent).toBe(expected);
	expect(drawing.querySelector('pre')?.textContent).toBe(expected);
});

test('tag filters group many pictures without turning picture titles into choices', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('gallery');
	const gallery = requiredElement(fixture, '#proof-gallery');
	const local = fixtureLocator(frame, gallery);
	await local.getByRole('button', { name: 'Filter screenshots', exact: true }).click();
	expect(gallery.querySelectorAll('[data-gallery-facet="screen"]')).toHaveLength(0);
	await local.getByRole('checkbox', { name: 'Account', exact: true }).click();
	await expect
		.poll((): string => requiredElement(gallery, '.ex-gallery-count').textContent ?? '')
		.toBe('2 screenshots');
	await expect.element(local.getByRole('checkbox', { name: 'Journey', exact: true })).toBeVisible();
});

test('Fit contains the picture and Fill covers the viewport, with no percentage control', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('gallery');
	const gallery = requiredElement(fixture, '#proof-gallery');
	const local = fixtureLocator(frame, gallery);
	await expect
		.poll(
			(): number =>
				gallery.querySelector<HTMLImageElement>('.ex-gallery-preview img')?.naturalWidth ?? 0,
		)
		.toBe(1000);
	await local.getByRole('button', { name: 'Fit screenshot', exact: true }).click();
	const fitted = requiredElement(gallery, '.ex-gallery-preview img').getBoundingClientRect();
	await local.getByRole('button', { name: 'Fill screenshot', exact: true }).click();
	const filled = requiredElement(gallery, '.ex-gallery-preview img').getBoundingClientRect();
	const stage = requiredElement(gallery, '.ex-gallery-preview');
	const view = fixture.defaultView;
	if (!view) throw new Error('Missing fixture window.');
	const style = view.getComputedStyle(stage);
	const innerWidth =
		stage.clientWidth -
		Number.parseFloat(style.paddingLeft) -
		Number.parseFloat(style.paddingRight);
	const innerHeight =
		stage.clientHeight -
		Number.parseFloat(style.paddingTop) -
		Number.parseFloat(style.paddingBottom);
	expect(fitted.width).toBeLessThanOrEqual(innerWidth + 1);
	expect(fitted.height).toBeLessThanOrEqual(innerHeight + 1);
	expect(filled.width).toBeGreaterThanOrEqual(innerWidth - 1);
	expect(filled.height).toBeGreaterThanOrEqual(innerHeight - 1);
	expect(gallery.querySelector('input[type="radio"]')).toBeNull();
	expect(gallery.querySelector('.ex-gallery-zoom')).toBeNull();
});

test('gallery walkthrough uses one floating player, two-axis keys and an explicit missing variation', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('ui', '#journey', 'journey.html');
	const gallery = requiredElement(fixture, '#journey-gallery');
	const local = fixtureLocator(frame, gallery);
	const player = frame.getByRole('region', {
		name: 'Walkthrough player',
		exact: true,
		includeHidden: true,
	});
	await expect.element(player.getByRole('button', { name: 'Desktop', exact: true })).toBeVisible();
	await player.getByRole('button', { name: 'Desktop', exact: true }).click();
	await expect.element(local.getByRole('img', { name: 'One Desktop', exact: true })).toBeVisible();
	expect(requiredElement(fixture, '.ex-route-count').textContent).toBe('1 / 3');
	await userEvent.keyboard('{ArrowRight}');
	await expect.element(local.getByRole('img', { name: 'Two Desktop', exact: true })).toBeVisible();
	await userEvent.keyboard('{ArrowRight}{ArrowUp}');
	await expect
		.element(local.getByRole('status', { name: 'Mobile capture is unavailable', exact: true }))
		.toBeVisible();
	expect(requiredElement(fixture, '.ex-route-count').textContent).toBe('3 / 3');
	expect(requiredElement(gallery, '.ex-gallery-picture').hasAttribute('hidden')).toBe(true);
	expect(fixture.querySelector('.ex-drawer')).toBeNull();
	await player.getByRole('button', { name: 'Desktop', exact: true }).click();
	await expect
		.element(local.getByRole('img', { name: 'Three Desktop', exact: true }))
		.toBeVisible();
	expect(fixture.querySelectorAll('.ex-route-player')).toHaveLength(1);
	await expect
		.poll((): boolean => {
			const caption = requiredElement(
				gallery,
				'.ex-gallery-picture > figcaption',
			).getBoundingClientRect();
			const bar = requiredElement(fixture, '.ex-route-player').getBoundingClientRect();
			return (
				bar.top - caption.bottom >= 12 && bar.width >= gallery.getBoundingClientRect().width - 2
			);
		})
		.toBe(true);
	await page.viewport(390, 900);
	await expect.poll((): number => fixture.documentElement.scrollWidth).toBeLessThanOrEqual(391);
	await expect
		.poll(
			(): number =>
				requiredElement(fixture, '.ex-route-player').getBoundingClientRect().top -
				requiredElement(gallery, '.ex-gallery-picture > figcaption').getBoundingClientRect().bottom,
		)
		.toBeGreaterThanOrEqual(12);
	await player.getByRole('button', { name: 'Close walkthrough', exact: true }).click();
	await expect
		.element(local.getByRole('searchbox', { name: 'Search screenshots', exact: true }))
		.toBeVisible();
});

test('gallery walkthrough restores browse filters, selection and zoom; variation switching can be omitted', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('ui', '', 'journey.html');
	const gallery = requiredElement(fixture, '#journey-gallery');
	const local = fixtureLocator(frame, gallery);
	await local.getByRole('searchbox', { name: 'Search screenshots', exact: true }).fill('One');
	await expect
		.poll((): string => requiredElement(gallery, '.ex-gallery-count').textContent ?? '')
		.toBe('2 screenshots');
	await local.getByRole('button', { name: 'One Desktop', exact: true }).click();
	await local.getByRole('button', { name: 'Fill screenshot', exact: true }).click();
	await expect
		.poll(
			(): number =>
				gallery.querySelector<HTMLImageElement>('.ex-gallery-preview img')?.naturalWidth ?? 0,
		)
		.toBe(1000);
	const widthWithThumbnails = requiredElement(
		gallery,
		'.ex-gallery-preview img',
	).getBoundingClientRect().width;
	await local.getByRole('button', { name: 'Hide thumbnails', exact: true }).click();
	await expect
		.poll(
			(): number =>
				requiredElement(gallery, '.ex-gallery-preview img').getBoundingClientRect().width,
		)
		.toBeGreaterThan(widthWithThumbnails);
	const filledWidth = requiredElement(gallery, '.ex-gallery-preview img').getBoundingClientRect()
		.width;
	await local.getByRole('button', { name: 'Grid view', exact: true }).click();
	await frame.getByRole('button', { name: 'Browse walkthroughs', exact: true }).click();
	await frame
		.getByRole('option', {
			name: 'Screen journey review › Review screens Three logical screens with optional size variations.',
			exact: true,
		})
		.click();
	const player = frame.getByRole('region', {
		name: 'Walkthrough player',
		exact: true,
		includeHidden: true,
	});
	await expect.element(local.getByRole('img', { name: 'One Mobile', exact: true })).toBeVisible();
	await player.getByRole('button', { name: 'Close walkthrough', exact: true }).click();
	await expect
		.element(local.getByRole('button', { name: 'Grid view', exact: true }))
		.toHaveAttribute('aria-pressed', 'true');
	expect(gallery.querySelector<HTMLInputElement>('input[type="search"]')?.value).toBe('One');
	expect(
		requiredElement(gallery, '[data-capture-id="one-desktop"]').getAttribute('aria-current'),
	).toBe('true');
	await local.getByRole('button', { name: 'Review view', exact: true }).click();
	await expect.element(local.getByRole('img', { name: 'One Desktop', exact: true })).toBeVisible();
	await expect
		.element(local.getByRole('button', { name: 'Show thumbnails', exact: true }))
		.toHaveAttribute('aria-expanded', 'false');
	await expect
		.element(local.getByRole('button', { name: 'Fill screenshot', exact: true }))
		.toHaveAttribute('aria-pressed', 'true');
	await expect
		.poll(
			(): number =>
				requiredElement(gallery, '.ex-gallery-preview img').getBoundingClientRect().width,
		)
		.toBeCloseTo(filledWidth, 0);
	await frame.getByRole('button', { name: 'Browse walkthroughs', exact: true }).click();
	await frame
		.getByRole('option', {
			name: 'Screen journey review › Fixed screens Use the authored captures without variation switching.',
			exact: true,
		})
		.click();
	await expect
		.element(player.getByRole('group', { name: 'Screen variation', exact: true }))
		.not.toBeInTheDocument();
	await player.getByRole('button', { name: 'Next step', exact: true }).click();
	await userEvent.keyboard('{ArrowUp}');
	await expect.element(local.getByRole('img', { name: 'Two Mobile', exact: true })).toBeVisible();
	expect(requiredElement(fixture, '.ex-route-count').textContent).toBe('2 / 2');
	await userEvent.keyboard('{Escape}');
	await expect.element(player).not.toBeVisible();
});
