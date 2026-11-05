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
	const workspace = requiredElement(fixture, 'ex-drawer.lab-related-drawer');
	const picker = requiredElement(workspace, 'lab-search-picker details');
	const summary = requiredElement(picker, 'summary');
	await fixtureLocator(frame, summary).click();
	expect(picker.hasAttribute('open')).toBe(true);

	// Act: Escape belongs to the open picker.
	await userEvent.keyboard('{Escape}');

	// Assert: focus and both closing boundaries remain usable.
	expect(picker.hasAttribute('open')).toBe(false);
	expect(workspace.matches(':popover-open')).toBe(true);
	expect(fixture.activeElement).toBe(summary);
	// Leave the picker, then an unhandled Escape belongs to the workspace.
	await userEvent.keyboard('{Tab}{Escape}');
	expect(workspace.matches(':popover-open')).toBe(false);
	expect(fixture.activeElement?.getAttribute('aria-label')).toBe('Open call stack');
});

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
		await fixtureLocator(frame, requiredElement(fixture, 'button[data-frame="0"]')).click();
		const sheet = requiredElement(fixture, 'ex-drawer.lab-call-detail-sheet');
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
