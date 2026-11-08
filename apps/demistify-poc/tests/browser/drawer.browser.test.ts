import { afterEach, expect, test } from 'vitest';
import { page, userEvent } from 'vitest/browser';

import type { DrawerIsland } from '../../../../packages/demistify/src/islands/drawer.ts';
import { fixtureLocator, openFixture, requiredElement } from './fixture-page.ts';

afterEach(async (): Promise<void> => {
	document.body.replaceChildren();
	await page.viewport(1600, 1000);
});

test('Escape closes a nested stack picker before its bottom workspace', async (): Promise<void> => {
	// Arrange: the actual call browser supplies its own Escape consumer.
	const { document: fixture, frame } = await openFixture(
		'understanding',
		'',
		'requirements.html?need=U2&stage=program-design',
	);
	await frame.getByRole('button', { name: 'Open call stack', exact: true }).click();
	const workspace = requiredElement(fixture, 'ex-drawer.ex-design-related-drawer');
	await expect.poll(() => workspace.querySelector('ex-design-picker details')).not.toBeNull();
	const picker = requiredElement(workspace, 'ex-design-picker details');
	const summary = requiredElement(picker, 'summary');
	await fixtureLocator(frame, summary).click();
	expect(picker.hasAttribute('open')).toBe(true);

	// Act: Escape belongs to the open picker.
	await userEvent.keyboard('{Escape}');

	// Assert: focus and both closing boundaries remain usable.
	expect(picker.hasAttribute('open')).toBe(false);
	expect(workspace.matches(':popover-open')).toBe(true);
	expect(fixture.activeElement).toBe(summary);
	// Once the picker is closed, Escape on its summary belongs to the workspace.
	await userEvent.keyboard('{Escape}');
	expect(workspace.matches(':popover-open')).toBe(false);
	expect(fixture.activeElement?.getAttribute('aria-label')).toBe('Open call stack');
});

test.each([1024, 1440, 1600])(
	'the open workspace keeps desktop reader controls reachable at %ipx',
	async (width): Promise<void> => {
		// Arrange: the real desktop page normally floats its reader controls at the bottom.
		await page.viewport(width, 1000);
		const { document: fixture, frame } = await openFixture(
			'understanding',
			'',
			'requirements.html?need=U2',
		);
		await frame.getByRole('button', { name: 'View together', exact: true }).click();
		const workspace = requiredElement(fixture, 'ex-drawer.ex-design-related-drawer');
		await expect.poll((): boolean => workspace.matches(':popover-open')).toBe(true);
		await expect.poll((): number => workspace.getBoundingClientRect().bottom).toBeCloseTo(1000, 0);

		// Assert: pointer hit testing, not a scripted click, establishes that each control is exposed.
		for (const selector of ['.ex-comments-trigger', '.ex-theme-toggle', '.ex-search-trigger']) {
			const control = requiredElement(fixture, selector);
			const controlBounds = control.getBoundingClientRect();
			const target = fixture.elementFromPoint(
				controlBounds.left + controlBounds.width / 2,
				controlBounds.top + controlBounds.height / 2,
			);
			expect(target === control || (target !== null && control.contains(target))).toBe(true);
		}
		const bounds = workspace.getBoundingClientRect();
		expect(bounds.top).toBeCloseTo(
			requiredElement(fixture, '.ex-document-toolbar').getBoundingClientRect().bottom,
			0,
		);
		expect(bounds.left).toBe(0);
		expect(bounds.right).toBe(width);

		// Act: exercise the same live controls with real pointer clicks.
		fixture.documentElement.dataset['theme'] = 'dark';
		await fixtureLocator(frame, requiredElement(fixture, '.ex-theme-toggle')).click();
		expect(fixture.documentElement.dataset['theme']).toBe('light');
		await fixtureLocator(frame, requiredElement(fixture, '.ex-comments-trigger')).click();
		await expect
			.element(frame.getByRole('button', { name: 'Close comments', exact: true }))
			.toBeVisible();
		expect(workspace.matches(':popover-open')).toBe(true);
		for (const selector of ['.ex-comments-trigger', '.ex-theme-toggle', '.ex-search-trigger']) {
			const control = requiredElement(fixture, selector);
			const controlBounds = control.getBoundingClientRect();
			const target = fixture.elementFromPoint(
				controlBounds.left + controlBounds.width / 2,
				controlBounds.top + controlBounds.height / 2,
			);
			expect(target === control || (target !== null && control.contains(target))).toBe(true);
		}
		await frame.getByRole('button', { name: 'Close comments', exact: true }).click();
		await fixtureLocator(frame, requiredElement(fixture, '.ex-search-trigger')).click();
		expect(fixture.querySelector('dialog:modal')).not.toBeNull();
		await userEvent.keyboard('{Escape}');
		await expect.poll(() => fixture.querySelector('dialog:modal')).toBeNull();
		expect(workspace.matches(':popover-open')).toBe(true);
		await frame.getByRole('button', { name: 'Close related documents', exact: true }).click();
		expect(requiredElement(fixture, '.ex-theme-toggle').getBoundingClientRect().bottom).toBe(
			1000 - 12,
		);
	},
);

test.each([
	['execution.html', 'light'],
	['execution.html', 'dark'],
	['requirements.html?need=U2&stage=program-design', 'light'],
	['requirements.html?need=U2&stage=program-design', 'dark'],
])(
	'a phone call sheet fits its body within the viewport cap: %s, %s',
	async (file, theme): Promise<void> => {
		// Arrange: cover both standalone calls and calls nested in the document workspace.
		await page.viewport(390, 844);
		const { document: fixture, frame } = await openFixture('understanding', '', file);
		fixture.documentElement.dataset['theme'] = theme;
		if (file.startsWith('requirements'))
			await frame.getByRole('button', { name: 'Open call stack', exact: true }).click();
		// The shared document toolbar can precede the app controller's module upgrade.
		await expect.poll(() => fixture.querySelector('button[data-frame="0"]')).not.toBeNull();
		await fixtureLocator(frame, requiredElement(fixture, 'button[data-frame="0"]')).click();
		const sheet = requiredElement(fixture, 'ex-drawer.ex-design-call-detail-sheet');
		await expect.poll((): boolean => sheet.matches(':popover-open')).toBe(true);
		const body = requiredElement(sheet, '.ex-inspect-body');
		const header = requiredElement(sheet, 'header');

		// Assert: content is visible, or only scrolls after reaching the documented 80dvh cap.
		const availableBodyHeight = 844 * 0.8 - header.getBoundingClientRect().height - 2;
		expect(body.scrollHeight).toBeGreaterThan(100);
		expect(body.clientHeight).toBeGreaterThanOrEqual(
			Math.floor(Math.min(body.scrollHeight, availableBodyHeight)),
		);
		expect(sheet.getBoundingClientRect().height).toBeLessThanOrEqual(844 * 0.8 + 1);
		await frame.getByRole('button', { name: 'Close call details', exact: true }).click();
		expect(sheet.matches(':popover-open')).toBe(false);
	},
);

test.each(['large', 'content'])(
	'an authored-open %s drawer upgrades once and can close by API and Escape',
	async (size): Promise<void> => {
		// Arrange: parse the open attribute before the real deferred runtime upgrades the element.
		const iframe = document.createElement('iframe');
		iframe.dataset['testid'] = 'explainer';
		iframe.title = 'Authored drawer fixture';
		iframe.style.cssText = 'display:block;width:100%;height:100vh;border:0';
		iframe.srcdoc = `<!doctype html><html><head><link rel="stylesheet" href="/__demistify/library/theme.css"></head><body>
			<ex-document><ex-drawer open size="${size}" label="Authored drawer"><header class="ex-inspect-header"><strong>Authored drawer</strong><button type="button">Close</button></header><div class="ex-inspect-body">${'<p>Visible authored content.</p>'.repeat(50)}</div></ex-drawer></ex-document>
			<script type="module">
				document.body.dataset.closeCount = '0';
				document.addEventListener('close', (event) => {
					if (event.target.localName === 'ex-drawer') document.body.dataset.closeCount = String(Number(document.body.dataset.closeCount) + 1);
				}, true);
				await import('/__demistify/library/runtime.js');
				document.body.dataset.runtimeReady = 'true';
			</script></body></html>`;
		document.body.append(iframe);
		await expect.poll(() => iframe.contentDocument?.body.dataset['runtimeReady']).toBe('true');
		const fixture = iframe.contentDocument;
		if (!fixture) throw new Error('Authored drawer fixture did not load.');
		const drawer = fixture.querySelector<DrawerIsland>('ex-drawer');
		if (!drawer) throw new Error('Authored drawer is missing.');
		expect(drawer.matches(':popover-open')).toBe(true);
		expect(fixture.body.dataset['closeCount']).toBe('0');
		if (size === 'content') {
			const body = requiredElement(drawer, '.ex-inspect-body');
			expect(drawer.getBoundingClientRect().height).toBeLessThanOrEqual(801);
			expect(body.clientHeight).toBeGreaterThan(100);
			expect(body.scrollHeight).toBeGreaterThan(body.clientHeight);
		}

		// Act: duplicate show calls must be harmless, then each close has one observable event.
		drawer.show();
		drawer.close();
		expect(drawer.matches(':popover-open')).toBe(false);
		expect(fixture.body.dataset['closeCount']).toBe('1');
		drawer.show();
		const frame = page.frameLocator(page.getByTestId('explainer'));
		await fixtureLocator(frame, requiredElement(drawer, 'button')).click();
		await userEvent.keyboard('{Escape}');
		expect(drawer.matches(':popover-open')).toBe(false);
		expect(fixture.body.dataset['closeCount']).toBe('2');
	},
);

test('a workspace keeps a live walkthrough and newly reported authoring issues reachable', async (): Promise<void> => {
	// Arrange: use existing library controls, including ones created after the workspace opens.
	const { document: fixture, frame } = await openFixture('navigation', '', 'index.html?ex-check');
	const workspace = fixture.createElement('ex-drawer');
	workspace.setAttribute('label', 'Overlay coexistence');
	workspace.setAttribute('open', '');
	workspace.innerHTML =
		'<header class="ex-inspect-header"><strong>Workspace</strong><button type="button">Close workspace</button></header><div class="ex-inspect-body"><p>Keep reader controls available.</p></div>';
	requiredElement(fixture, 'ex-document').append(workspace);
	await expect.poll(() => workspace.matches(':popover-open')).toBe(true);
	await frame.getByRole('button', { name: 'Browse walkthroughs', exact: true }).click();
	await frame.getByRole('option', { name: /Implementation plan/ }).click();
	const invalidBadge = fixture.createElement('ex-badge');
	invalidBadge.setAttribute('tone', 'unsupported');
	invalidBadge.textContent = 'Authoring diagnostic probe';
	requiredElement(fixture, 'ex-document').append(invalidBadge);
	await expect.poll(() => fixture.querySelector('.ex-issues button')).not.toBeNull();

	// Assert: neither the active player nor the diagnostic panel sits behind the top layer.
	const verifyOverlays = (): void => {
		for (const selector of ['.ex-route-player button', '.ex-issues button']) {
			const control = requiredElement(fixture, selector);
			const bounds = control.getBoundingClientRect();
			const target = fixture.elementFromPoint(
				bounds.left + bounds.width / 2,
				bounds.top + bounds.height / 2,
			);
			expect.soft(target === control || (target !== null && control.contains(target))).toBe(true);
		}
	};
	verifyOverlays();
	const sheet = fixture.createElement('ex-drawer');
	sheet.setAttribute('size', 'content');
	sheet.setAttribute('open', '');
	sheet.innerHTML =
		'<header class="ex-inspect-header"><strong>Nested details</strong></header><div class="ex-inspect-body"><p>Reader controls remain above this sheet too.</p></div>';
	workspace.append(sheet);
	await expect.poll(() => sheet.matches(':popover-open')).toBe(true);
	verifyOverlays();
	sheet.removeAttribute('open');
	await frame.getByRole('button', { name: 'Close walkthrough', exact: true }).click();
	expect(workspace.matches(':popover-open')).toBe(true);
	workspace.removeAttribute('open');
	expect(requiredElement(fixture, '.ex-route-player').parentElement).toBe(fixture.body);
	expect(requiredElement(fixture, '.ex-issues').parentElement).toBe(fixture.body);
	expect(fixture.querySelector('.ex-drawer-reader-overlays:popover-open')).toBeNull();
});

test.each(['body', 'modal'])(
	'closing a workspace respects the player owner moving it to %s',
	async (destination): Promise<void> => {
		// Arrange: borrow the existing live player, just as a real walkthrough workspace does.
		const { document: fixture } = await openFixture('navigation');
		const player = requiredElement(fixture, '.ex-route-player');
		const workspace = fixture.createElement('ex-drawer');
		workspace.setAttribute('open', '');
		workspace.innerHTML = '<div class="ex-inspect-body"><p>Workspace</p></div>';
		requiredElement(fixture, 'ex-document').append(workspace);
		await expect.poll(() => player.parentElement?.className).toBe('ex-drawer-reader-overlays');
		const modal = fixture.createElement('dialog');

		// Act: exercise the same DOM transfer performed by route.ts's player owner.
		if (destination === 'body') {
			fixture.body.append(player);
			await expect.poll(() => player.parentElement?.className).toBe('ex-drawer-reader-overlays');
		} else {
			fixture.body.append(modal);
			modal.showModal();
			modal.append(player);
		}
		workspace.removeAttribute('open');

		// Assert: return only content still borrowed, with no abandoned origin markers.
		expect(player.parentElement).toBe(destination === 'body' ? fixture.body : modal);
		const comments = fixture.createTreeWalker(fixture.body, NodeFilter.SHOW_COMMENT);
		const remainingOrigins: Node[] = [];
		let comment = comments.nextNode();
		while (comment) {
			if (comment.textContent === 'reader overlay origin') remainingOrigins.push(comment);
			comment = comments.nextNode();
		}
		expect(remainingOrigins).toHaveLength(0);
		if (modal.open) modal.close();
	},
);
