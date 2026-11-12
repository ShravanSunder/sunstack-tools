import { afterEach, expect, test } from 'vitest';
import { commands, page, userEvent } from 'vitest/browser';

import { firstTextRect, fixtureLocator, openFixture, requiredElement } from './fixture-page.ts';

afterEach(async (): Promise<void> => {
	document.body.replaceChildren();
	await commands.setReducedMotion('no-preference');
	await page.viewport(1600, 1000);
});

async function moveFocusOutsideMap(fixture: Document, remaining = 6): Promise<void> {
	if (!fixture.activeElement?.closest('.ex-drawer')) return;
	if (remaining === 0) throw new Error('Keyboard focus did not leave the non-modal Map.');
	await userEvent.keyboard('{Shift>}{Tab}{/Shift}');
	await moveFocusOutsideMap(fixture, remaining - 1);
}

test.each([1280, 390])(
	'Map keeps Pages and search keyboard launchers usable at %ipx',
	async (width): Promise<void> => {
		await page.viewport(width, 1000);
		const { document: fixture, frame } = await openFixture(
			'show-me-work',
			'',
			'reader-controls.html',
		);
		await frame.getByRole('button', { name: 'Map', exact: true }).click();
		const trigger = requiredElement(fixture, '.ex-map-trigger');
		const verifySearchShortcut = async (shortcut: string): Promise<void> => {
			const previousFocus = fixture.activeElement;
			await userEvent.keyboard(shortcut);
			await expect
				.element(frame.getByRole('dialog', { name: 'Search documents', exact: true }))
				.toBeVisible();
			await userEvent.keyboard('{Escape}');
			expect(trigger.getAttribute('aria-expanded')).toBe('true');
			await expect.poll((): Element | null => fixture.activeElement).toBe(previousFocus);
		};
		await verifySearchShortcut('{Control>}k{/Control}');
		await verifySearchShortcut('{Meta>}k{/Meta}');
		await moveFocusOutsideMap(fixture);
		await userEvent.keyboard('p');
		await expect
			.poll((): boolean => requiredElement(fixture, '.ex-pages-panel').matches(':popover-open'))
			.toBe(true);
		await frame.getByRole('button', { name: 'Browse pages', exact: true }).click();
		await expect
			.poll((): boolean => requiredElement(fixture, '.ex-pages-panel').matches(':popover-open'))
			.toBe(false);
		expect(trigger.getAttribute('aria-expanded')).toBe('true');
		await userEvent.keyboard('w');
		await expect.element(frame.getByRole('option', { name: /Read the story/ })).toBeVisible();
		await userEvent.keyboard('{Escape}');
	},
);

test.each([1280, 390])(
	'Map keeps an active walkthrough keyboard path usable at %ipx',
	async (width): Promise<void> => {
		await page.viewport(width, 1000);
		const { document: fixture, frame } = await openFixture(
			'show-me-work',
			'',
			'reader-controls.html',
		);
		await fixtureLocator(frame, requiredElement(fixture, 'h1')).click();
		await userEvent.keyboard('w');
		await frame.getByRole('option', { name: /Read the story/ }).click();
		await expect
			.poll((): string | null => requiredElement(fixture, '.ex-route-count').textContent)
			.toBe('1 / 2');
		await frame.getByRole('button', { name: 'Map', exact: true }).click();
		// Move native focus out of the non-modal rail without activating another overlay.
		await moveFocusOutsideMap(fixture);
		expect(fixture.activeElement?.closest('.ex-drawer')).toBeNull();
		expect(requiredElement(fixture, '.ex-map-trigger').getAttribute('aria-expanded')).toBe('true');
		await userEvent.keyboard('{ArrowRight}');
		await expect
			.poll((): string | null => requiredElement(fixture, '.ex-route-count').textContent)
			.toBe('2 / 2');
		await expect.poll((): string => fixture.location.hash).toBe('#story-other');
	},
);

test('Map keeps gallery arrow navigation usable from the page', async (): Promise<void> => {
	await page.viewport(1280, 1000);
	const { document: fixture, frame } = await openFixture('gallery');
	const root = requiredElement(fixture, 'ex-document');
	root.id ||= 'gallery-map-scope';
	// Integration input adds an authored Map to the existing real gallery, without cloning output.
	const map = fixture.createElement('ex-map');
	map.setAttribute('for', root.id);
	map.innerHTML =
		'<ex-tree label="Gallery destinations"><script type="text/markdown">- [Gallery](#proof-gallery)</script></ex-tree>';
	root.append(map);
	await frame.getByRole('button', { name: 'Map', exact: true }).click();
	const gallery = requiredElement(fixture, '#proof-gallery');
	await fixtureLocator(frame, requiredElement(gallery, '.ex-gallery-title')).click();
	await userEvent.keyboard('{ArrowRight}');
	await expect
		.poll((): string | null =>
			requiredElement(gallery, '.ex-gallery-preview img').getAttribute('alt'),
		)
		.toBe('Settings Mobile');
});

test.each([
	{
		name: 'ui',
		file: 'content-spacing.html',
		controls: ['.ex-search-trigger', '.ex-theme-toggle', '.ex-comments-trigger'],
	},
	{
		name: 'navigation',
		file: 'index.html',
		controls: [
			'.ex-search-trigger',
			'.ex-theme-toggle',
			'.ex-comments-trigger',
			'.ex-presentations-trigger',
		],
	},
	{
		name: 'show-me-work',
		file: 'index.html',
		controls: ['.ex-search-trigger', '.ex-theme-toggle', '.ex-comments-trigger', '.ex-map-trigger'],
	},
] as const)(
	'phone reader controls follow document header order on $name',
	async ({ name, file, controls }): Promise<void> => {
		await page.viewport(390, 1000);
		const { document: fixture } = await openFixture(name, '', file);
		await expect
			.poll((): boolean => controls.every((selector) => fixture.querySelector(selector) !== null))
			.toBe(true);
		const bounds = controls.map((selector) =>
			requiredElement(fixture, selector).getBoundingClientRect(),
		);
		for (const [index, rect] of bounds.entries()) {
			expect(rect.top).toBeGreaterThanOrEqual(0);
			expect(rect.bottom).toBeLessThanOrEqual(
				requiredElement(fixture, '.ex-document-toolbar').getBoundingClientRect().bottom,
			);
			const next = bounds[index + 1];
			if (next) expect(rect.right).toBeLessThanOrEqual(next.left);
		}
	},
);

test.each([1280, 390])(
	'Map totals track live membership while story bulk actions leave its branches alone at %ipx',
	async (width): Promise<void> => {
		await page.viewport(width, 1000);
		const { document: fixture, frame } = await openFixture('show-me-work');
		await frame.getByRole('button', { name: 'Map', exact: true }).click();
		const deliveryCount = requiredElement(
			requiredElement(fixture, '#work-map a.ex-tree-go[href$="#delivery"]').closest(
				'.ex-tree-node',
			) ?? fixture,
			'.ex-map-count',
		);
		expect(deliveryCount.textContent).toBe(' · 2');
		const branch = requiredElement(
			fixture,
			'#work-map button[aria-label="Toggle Where the work is"]',
		);
		expect(branch.getAttribute('aria-expanded')).toBe('false');
		await frame.getByRole('button', { name: 'Close Map', exact: true }).click();

		// Integration input: a newly authored story, not a clone of enhanced output.
		const template = fixture.createElement('template');
		template.innerHTML = `<ex-story><details id="added-story" open><summary>Newly added account</summary>
		<p slot="headline">An added member changes the count.</p><div slot="happened">Added locally.</div>
		<div slot="for-you">Read the updated count.</div><div slot="next">Remove this member.</div>
		<details slot="record"><summary>The record: 0 entries</summary></details></details></ex-story>`;
		const added = template.content.firstElementChild;
		if (!added) throw new Error('Missing authored membership input.');
		requiredElement(fixture, '#smaller').before(added);
		await expect.poll((): string | null => deliveryCount.textContent).toBe(' · 3');
		expect(requiredElement(fixture, '#delivery > .ex-section-head').textContent).toContain(
			'2 + 1 smaller',
		);
		await frame.getByRole('button', { name: 'Expand all stories', exact: true }).click();
		expect(branch.getAttribute('aria-expanded')).toBe('false');
		await frame.getByRole('button', { name: 'Collapse all stories', exact: true }).click();
		expect(branch.getAttribute('aria-expanded')).toBe('false');
		expect(requiredElement(fixture, '#ordinary-notes').hasAttribute('open')).toBe(false);
		added.remove();
		await expect.poll((): string | null => deliveryCount.textContent).toBe(' · 2');
		expect(requiredElement(fixture, '#delivery > .ex-section-head').textContent).toContain(
			'1 + 1 smaller',
		);
	},
);

test.each(['light', 'dark'])(
	'Map and its original control survive both 899/900 transitions in %s',
	async (theme): Promise<void> => {
		await page.viewport(899, 1000);
		const { document: fixture, frame } = await openFixture('show-me-work');
		fixture.documentElement.dataset['theme'] = theme;
		await frame.getByRole('button', { name: 'Map', exact: true }).click();
		const trigger = requiredElement(fixture, '.ex-map-trigger');
		const tree = requiredElement(fixture, '#work-map');
		const panel = requiredElement(fixture, '.ex-drawer[data-presentation="map"]');
		await frame.getByRole('button', { name: 'Toggle Where the work is', exact: true }).click();
		const verifyResize = async (width: number): Promise<void> => {
			await page.viewport(width, 1000);
			expect(requiredElement(fixture, '.ex-map-trigger')).toBe(trigger);
			expect(requiredElement(fixture, '#work-map')).toBe(tree);
			expect(fixture.querySelector('dialog:modal')).toBeNull();
			expect(
				requiredElement(tree, 'button[aria-label="Toggle Where the work is"]').getAttribute(
					'aria-expanded',
				),
			).toBe('true');
			await expect
				.poll((): number => panel.getBoundingClientRect().width)
				.toBe(width === 899 ? 899 : 460);
		};
		await verifyResize(900);
		await verifyResize(899);
		await fixtureLocator(frame, requiredElement(fixture, '.ex-comments-trigger')).click();
		expect(trigger.closest('dialog:modal > header')).not.toBeNull();
		await page.viewport(900, 1000);
		await expect.poll((): Element | null => fixture.querySelector('dialog:modal')).toBeNull();
		expect(trigger.closest('.ex-reader-controls')).not.toBeNull();
		await page.viewport(899, 1000);
		await expect.poll((): boolean => trigger.closest('dialog:modal > header') !== null).toBe(true);
		await frame.getByRole('button', { name: 'Map', exact: true }).click();
		expect(requiredElement(fixture, '.ex-map-trigger')).toBe(trigger);
		expect(requiredElement(fixture, '#work-map')).toBe(tree);
		await frame.getByRole('button', { name: 'Close Map', exact: true }).click();
		expect(trigger.closest('.ex-reader-controls')).not.toBeNull();
	},
);

test.each([
	[1280, 'no-preference'],
	[390, 'no-preference'],
	[1280, 'reduce'],
	[390, 'reduce'],
] as const)(
	'Map and Comments share the actual animation family at %ipx with %s motion',
	async (width, motion): Promise<void> => {
		await commands.setReducedMotion(motion);
		await page.viewport(width, 1000);
		const { document: fixture, frame } = await openFixture('show-me-work');
		await frame.getByRole('button', { name: 'Map', exact: true }).click();
		const mapStyle = fixture.defaultView?.getComputedStyle(
			requiredElement(fixture, '.ex-drawer[data-presentation="map"]'),
		);
		const animation = mapStyle?.animationName;
		const duration = mapStyle?.animationDuration;
		expect(animation).toBe(motion === 'reduce' ? 'none' : 'ex-drawer-enter');
		if (motion === 'no-preference') expect(Number.parseFloat(duration ?? '')).toBeGreaterThan(0);
		await fixtureLocator(frame, requiredElement(fixture, '.ex-comments-trigger')).click();
		const comments = requiredElement(
			fixture,
			width < 900 ? 'dialog:modal' : '.ex-drawer[aria-label="Comments"]',
		);
		const commentsStyle = fixture.defaultView?.getComputedStyle(comments);
		expect(commentsStyle?.animationName).toBe(animation);
		expect(commentsStyle?.animationDuration).toBe(duration);
	},
);

test.each([1280, 390])(
	'Map leaves Pages usable and keyboard opening and closing restore visible focus at %ipx',
	async (width): Promise<void> => {
		await page.viewport(width, 1000);
		const { document: fixture, frame } = await openFixture('show-me-work');
		const map = frame.getByRole('button', { name: 'Map', exact: true });
		await map.click();
		await frame.getByRole('button', { name: 'Close Map', exact: true }).click();
		const trigger = requiredElement(fixture, '.ex-map-trigger');
		expect(fixture.activeElement).toBe(trigger);
		await userEvent.keyboard('{Enter}');
		expect(trigger.getAttribute('aria-expanded')).toBe('true');
		expect(fixture.activeElement?.getAttribute('aria-label')).toBe('Close Map');
		await userEvent.keyboard('{Tab}');
		expect(fixture.activeElement?.closest('.ex-map-body')).not.toBeNull();
		expect(fixture.activeElement?.matches(':focus-visible')).toBe(true);
		await frame.getByRole('button', { name: 'Browse pages', exact: true }).click();
		expect(requiredElement(fixture, '.ex-pages-panel').matches(':popover-open')).toBe(true);
		expect(trigger.getAttribute('aria-expanded')).toBe('true');
		expect(fixture.querySelector('dialog:modal')).toBeNull();
		await frame.getByRole('button', { name: 'Browse pages', exact: true }).click();
		expect(requiredElement(fixture, '.ex-pages-panel').matches(':popover-open')).toBe(false);
		await frame.getByRole('button', { name: 'Close Map', exact: true }).click();
		expect(fixture.activeElement).toBe(trigger);
	},
);

test.each([1280, 390])(
	'an independent bottom workspace and Map/Comments keep their own lifetimes at %ipx',
	async (width): Promise<void> => {
		await page.viewport(width, 1000);
		const { document: fixture, frame } = await openFixture('show-me-work');
		// Component integration: authored-open workspace, then real pointer replacement of the rail.
		const workspace = fixture.createElement('ex-drawer');
		workspace.setAttribute('open', '');
		workspace.setAttribute('label', 'Independent workspace');
		workspace.innerHTML =
			'<header class="ex-inspect-header"><strong>Workspace</strong></header><div class="ex-inspect-body"><p>A separate workspace remains open.</p></div>';
		requiredElement(fixture, 'ex-document').append(workspace);
		await expect.poll((): boolean => workspace.matches(':popover-open')).toBe(true);
		await frame.getByRole('button', { name: 'Map', exact: true }).click();
		expect(workspace.matches(':popover-open')).toBe(true);
		await fixtureLocator(frame, requiredElement(fixture, '.ex-comments-trigger')).click();
		expect(workspace.matches(':popover-open')).toBe(true);
		await frame.getByRole('button', { name: 'Close comments', exact: true }).click();
		expect(workspace.matches(':popover-open')).toBe(true);
		workspace.removeAttribute('open');
		expect(requiredElement(fixture, '.ex-reader-controls').parentElement).toBe(
			requiredElement(fixture, '.ex-document-toolbar'),
		);
		expect(requiredElement(fixture, '.ex-document-toolbar').parentElement).toBe(fixture.body);
	},
);

test.each([1280, 390])(
	'all original reader controls stay usable with Map and a bottom workspace at %ipx',
	async (width): Promise<void> => {
		await page.viewport(width, 1000);
		const { document: fixture, frame } = await openFixture(
			'show-me-work',
			'',
			'reader-controls.html',
		);
		const controls = [
			'.ex-presentations-trigger',
			'.ex-theme-toggle',
			'.ex-search-trigger',
			'.ex-comments-trigger',
			'.ex-map-trigger',
			'.ex-pages-trigger',
		];
		const verifyHitTargets = (): void => {
			for (const selector of controls) {
				const element = requiredElement(fixture, selector),
					rect = element.getBoundingClientRect();
				expect(rect.width).toBeGreaterThan(0);
				const hit = fixture.elementFromPoint(
					rect.left + rect.width / 2,
					rect.top + rect.height / 2,
				);
				expect(hit === element || (hit !== null && element.contains(hit))).toBe(true);
			}
		};
		verifyHitTargets();
		// Component setup opens the independent workspace; every subsequent control action is native.
		const workspace = fixture.createElement('ex-drawer');
		workspace.setAttribute('label', 'Reader controls workspace');
		workspace.setAttribute('open', '');
		workspace.innerHTML =
			'<header class="ex-inspect-header"><strong>Workspace</strong></header><div class="ex-inspect-body"><p>Retain the reader controls.</p></div>';
		requiredElement(fixture, 'ex-document').append(workspace);
		await expect.poll((): boolean => workspace.matches(':popover-open')).toBe(true);
		verifyHitTargets();
		await frame.getByRole('button', { name: 'Map', exact: true }).click();
		verifyHitTargets();
		await frame.getByRole('button', { name: 'Browse walkthroughs', exact: true }).click();
		await expect.element(frame.getByRole('option', { name: /Read the story/ })).toBeVisible();
		await userEvent.keyboard('{Escape}');
		expect(requiredElement(fixture, '.ex-map-trigger').getAttribute('aria-expanded')).toBe('true');
		await fixtureLocator(frame, requiredElement(fixture, '.ex-search-trigger')).click();
		expect(fixture.querySelector('dialog:modal')).not.toBeNull();
		await userEvent.keyboard('{Escape}');
		await fixtureLocator(frame, requiredElement(fixture, '.ex-theme-toggle')).click();
		verifyHitTargets();
		await fixtureLocator(frame, requiredElement(fixture, '.ex-comments-trigger')).click();
		await frame.getByRole('button', { name: 'Map', exact: true }).click();
		verifyHitTargets();
		await frame.getByRole('button', { name: 'Close Map', exact: true }).click();
		expect(workspace.matches(':popover-open')).toBe(true);
		verifyHitTargets();
		workspace.removeAttribute('open');
	},
);

test.each([1280, 390])(
	'cold encoded entry links open only their owning native folds at %ipx',
	async (width): Promise<void> => {
		await page.viewport(width, 1000);
		const { document: fixture } = await openFixture('show-me-work', '#%65ntry-two');
		await expect
			.poll((): boolean => requiredElement(fixture, '#entry-two').hasAttribute('open'))
			.toBe(true);
		for (const id of ['story-main', 'main-record'])
			expect(requiredElement(fixture, `#${id}`).hasAttribute('open')).toBe(true);
		for (const id of ['entry-one', 'story-minor', 'smaller', 'other-record'])
			expect(requiredElement(fixture, `#${id}`).hasAttribute('open')).toBe(false);
		const title = requiredElement(fixture, '#entry-two > summary').getBoundingClientRect();
		expect(title.top).toBeGreaterThanOrEqual(
			requiredElement(fixture, '#delivery > .ex-section-head').getBoundingClientRect().bottom,
		);
		expect(title.bottom).toBeLessThan(1000);
	},
);

test.each([1280, 390])(
	'same-document relative and absolute links retain native history and repeat reveal at %ipx',
	async (width): Promise<void> => {
		await page.viewport(width, 1000);
		const { document: fixture, frame } = await openFixture('show-me-work');
		const holder = fixture.createElement('p');
		const relative = fixture.createElement('a');
		relative.href = 'index.html#%73tory-minor';
		relative.textContent = 'Relative routine';
		const absolute = fixture.createElement('a');
		absolute.href = new URL('#story-minor', fixture.URL).href;
		absolute.target = '_SELF';
		absolute.textContent = 'Absolute routine';
		holder.append(relative, absolute);
		requiredElement(fixture, 'h1').after(holder);
		await expect.poll((): string => fixture.readyState).toBe('complete');
		const initialUrl = fixture.URL;
		await fixtureLocator(frame, relative).click();
		await expect
			.poll((): boolean => requiredElement(fixture, '#story-minor').hasAttribute('open'))
			.toBe(true);
		await expect.poll((): string => fixture.location.hash).toBe('#%73tory-minor');
		// The runner shares joint session history across removed iframes; inspect destinations,
		// rather than assuming no forward entries can be truncated by a native activation.
		fixture.defaultView?.history.back();
		await expect.poll((): string => fixture.URL).toBe(initialUrl);
		fixture.defaultView?.history.forward();
		await expect.poll((): string => fixture.location.hash).toBe('#%73tory-minor');
		await fixtureLocator(frame, requiredElement(fixture, '#story-minor > summary')).click();
		await fixtureLocator(frame, requiredElement(fixture, '#smaller > summary')).click();
		await fixtureLocator(frame, absolute).click();
		await expect
			.poll((): boolean => requiredElement(fixture, '#story-minor').hasAttribute('open'))
			.toBe(true);
		await expect.poll((): string => fixture.location.hash).toBe('#story-minor');
		expect(requiredElement(fixture, '#smaller').hasAttribute('open')).toBe(true);
		expect(requiredElement(fixture, '#entry-one').hasAttribute('open')).toBe(false);
		const afterDifferentFragmentSpelling = fixture.defaultView?.history.length;
		await fixtureLocator(frame, requiredElement(fixture, '#story-minor > summary')).click();
		await fixtureLocator(frame, absolute).click();
		await expect
			.poll((): boolean => requiredElement(fixture, '#story-minor').hasAttribute('open'))
			.toBe(true);
		expect(fixture.defaultView?.history.length).toBe(afterDifferentFragmentSpelling);
		fixture.defaultView?.history.back();
		await expect.poll((): string => fixture.location.hash).toBe('#%73tory-minor');
		expect(requiredElement(fixture, '#story-minor').hasAttribute('open')).toBe(true);
	},
);

test.each([1280, 390])(
	'missing and malformed Map destinations preserve the panel and leave unrelated folds closed at %ipx',
	async (width): Promise<void> => {
		await page.viewport(width, 1000);
		const { document: fixture, frame } = await openFixture('show-me-work', '', 'invalid-map.html');
		await frame.getByRole('button', { name: 'Map', exact: true }).click();
		expect(fixture.documentElement.dataset['exIssues']).toBeUndefined();
		await frame.getByRole('link', { name: 'Go to Not supplied', exact: true }).click();
		await expect.poll((): string => fixture.location.hash).toBe('#not-supplied');
		expect(requiredElement(fixture, '.ex-map-trigger').getAttribute('aria-expanded')).toBe('true');
		expect(requiredElement(fixture, '#story-unrelated').hasAttribute('open')).toBe(false);
		await frame.getByRole('link', { name: 'Go to Malformed', exact: true }).click();
		await expect.poll((): string => fixture.location.hash).toBe('#bad%escape');
		expect(requiredElement(fixture, '.ex-map-trigger').getAttribute('aria-expanded')).toBe('true');
		expect(requiredElement(fixture, '#story-unrelated').hasAttribute('open')).toBe(false);
	},
);

test.each([1280, 390])(
	'a later native destination cancels an older lazy fragment request at %ipx',
	async (width): Promise<void> => {
		await page.viewport(width, 1000);
		const { document: fixture, frame } = await openFixture('show-me-work');
		const delayed = fixture.createElement('a');
		delayed.href = '#late-target';
		delayed.textContent = 'An island not yet added';
		requiredElement(fixture, 'h1').after(delayed);
		await fixtureLocator(frame, delayed).click();
		await expect.poll((): string => fixture.location.hash).toBe('#late-target');
		await frame.getByRole('button', { name: 'Map', exact: true }).click();
		await frame.getByRole('link', { name: 'Go to Routine story', exact: true }).click();
		await expect.poll((): string => fixture.location.hash).toBe('#story-minor');
		// A real late Markdown island emits readiness; an obsolete resolver must not scroll to it.
		const late = fixture.createElement('ex-markdown');
		late.id = 'late-target';
		const source = fixture.createElement('script');
		source.type = 'text/markdown';
		source.textContent = '## Late unrelated material\n\nThis belongs after the active destination.';
		late.append(source);
		requiredElement(fixture, 'ex-document').append(late);
		await expect.poll((): boolean => late.hasAttribute('prepared')).toBe(true);
		const view = fixture.defaultView;
		if (!view) throw new Error('Missing fixture window.');
		await new Promise<void>((resolve): void => {
			view.requestAnimationFrame((): void => resolve());
		});
		const title = requiredElement(fixture, '#story-minor > summary').getBoundingClientRect();
		expect(title.top).toBeGreaterThanOrEqual(
			requiredElement(fixture, '#delivery > .ex-section-head').getBoundingClientRect().bottom,
		);
		expect(title.bottom).toBeLessThan(1000);
		expect(requiredElement(fixture, '#entry-one').hasAttribute('open')).toBe(false);
	},
);

test.each([1280, 390])(
	'story text remains natively selectable after Map closes at %ipx',
	async (width): Promise<void> => {
		await page.viewport(width, 1000);
		const { document: fixture, frame } = await openFixture('show-me-work');
		await frame.getByRole('button', { name: 'Map', exact: true }).click();
		await frame.getByRole('button', { name: 'Close Map', exact: true }).click();
		const text = requiredElement(fixture, '#story-main [slot="headline"]');
		text.scrollIntoView({ block: 'center', behavior: 'instant' });
		const rect = firstTextRect(text);
		await commands.dragReviewSelection({
			startX: rect.left + 1,
			startY: rect.top + rect.height / 2,
			endX: rect.left + 25,
			endY: rect.top + rect.height / 2,
		});
		await expect.poll((): string => fixture.getSelection()?.toString() ?? '').not.toBe('');
		expect(text.textContent).toContain(fixture.getSelection()?.toString());
		const summary = requiredElement(fixture, '#story-main > summary');
		summary.scrollIntoView({ block: 'center', behavior: 'instant' });
		const titleRect = firstTextRect(requiredElement(summary, '.ex-story-title'));
		await commands.dragReviewSelection({
			startX: titleRect.left + 1,
			startY: titleRect.top + titleRect.height / 2,
			endX: titleRect.left + 100,
			endY: titleRect.top + titleRect.height / 2,
		});
		await expect.poll((): string => fixture.getSelection()?.toString() ?? '').not.toBe('');
		expect(summary.textContent).toContain(fixture.getSelection()?.toString());
		expect(requiredElement(fixture, '#story-main').hasAttribute('open')).toBe(true);
		expect(fixture.activeElement).toBe(summary);
		await userEvent.keyboard('{Enter}');
		expect(requiredElement(fixture, '#story-main').hasAttribute('open')).toBe(false);
		await userEvent.keyboard(' ');
		expect(requiredElement(fixture, '#story-main').hasAttribute('open')).toBe(true);
	},
);
