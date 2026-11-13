import { afterEach, expect, test } from 'vitest';
import { page, userEvent } from 'vitest/browser';

import { fixtureLocator, openFixture, requiredElement } from './fixture-page.ts';

afterEach((): void => document.body.replaceChildren());

test('standalone calls start from the first authored stack and frame', async (): Promise<void> => {
	const { document: fixture } = await openFixture('understanding-collection', '', 'execution.html');
	await expect
		.poll(() => fixture.querySelector('button[data-frame="0"]')?.getAttribute('aria-pressed'))
		.toBe('true');
	expect(requiredElement(fixture, 'ex-call-stack summary').textContent).toContain('Open Help');
	expect(fixture.querySelector('[role="alert"]')).toBeNull();
});

test('an unmapped need links to its authored owner while the stack remains usable', async (): Promise<void> => {
	await page.viewport(1600, 1000);
	const { document: fixture, frame } = await openFixture(
		'understanding-collection',
		'',
		'execution.html?need=U2',
	);
	await expect.poll(() => fixture.querySelector('ex-call-stack[data-ready]')).not.toBeNull();
	expect(requiredElement(fixture, '.ex-design-frame-detail').textContent).toContain(
		'No call is mapped to this need',
	);
	expect(requiredElement(fixture, '.ex-design-frame-detail a').getAttribute('href')).toBe(
		'requirements.html#body-U2',
	);
	expect(fixture.querySelector('button[data-frame][aria-pressed="true"]')).toBeNull();
	await fixtureLocator(frame, requiredElement(fixture, 'button[data-frame="0"]')).click();
	expect(fixture.querySelector('button[data-frame="0"]')?.getAttribute('aria-pressed')).toBe(
		'true',
	);
	expect(new URL(fixture.URL).searchParams.get('call')).toBe('handle-key');
});

test('a call-to-need link opens its visible authored body on the owner page', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture(
		'understanding-collection',
		'',
		'execution.html?stack=help-path&call=help-panel-open',
	);
	await expect.poll(() => fixture.querySelector('ex-call-stack[data-ready]')).not.toBeNull();
	const link = requiredElement(fixture, '.ex-design-frame-requirement a');
	expect(link.getAttribute('href')).toBe('requirements.html#body-U3');
	await fixtureLocator(frame, link).click();
	const iframe = document.querySelector<HTMLIFrameElement>('iframe[data-testid="explainer"]');
	await expect
		.poll(() => iframe?.contentDocument?.getElementById('body-U3')?.textContent)
		.toContain('Help should show the key');
	const target = iframe?.contentDocument?.getElementById('body-U3');
	if (!target) throw new Error('Missing authored need body');
	expect(target.closest('template')).toBeNull();
	expect(target.getBoundingClientRect().height).toBeGreaterThan(0);
	expect(iframe?.contentWindow?.location.hash).toBe('#body-U3');
});

test('arrow keys navigate standalone calls and keep keyboard focus on the selected frame', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture(
		'understanding-collection',
		'',
		'execution.html',
	);
	await expect.poll(() => fixture.querySelector('ex-call-stack[data-ready]')).not.toBeNull();
	await fixtureLocator(frame, requiredElement(fixture, 'button[data-frame="0"]')).click();
	await userEvent.keyboard('{ArrowDown}');
	expect(requiredElement(fixture, 'button[data-frame="1"]').getAttribute('aria-pressed')).toBe(
		'true',
	);
	expect(fixture.activeElement).toBe(requiredElement(fixture, 'button[data-frame="1"]'));
	expect(new URL(fixture.URL).searchParams.get('call')).toBe('input-owner-route');
	await userEvent.keyboard('{ArrowUp}');
	expect(requiredElement(fixture, 'button[data-frame="0"]').getAttribute('aria-pressed')).toBe(
		'true',
	);
	expect(fixture.activeElement).toBe(requiredElement(fixture, 'button[data-frame="0"]'));
});

test.each([
	['help-panel-open', 'HelpPanel.open'],
	['missing-call', 'Call not found in this stack'],
])(
	'a cold nonfixture call address selects its own call or explicit gap: %s',
	async (callId, expected): Promise<void> => {
		await page.viewport(1600, 1000);
		const { document: fixture } = await openFixture(
			'understanding-collection',
			'',
			`execution.html?stack=help-path&call=${callId}`,
		);
		await expect.poll(() => fixture.querySelector('ex-call-stack[data-ready]')).not.toBeNull();
		expect(requiredElement(fixture, '.ex-design-frame-detail').textContent).toContain(expected);
		expect(new URL(fixture.URL).searchParams.get('stack')).toBe('help-path');
		expect(new URL(fixture.URL).searchParams.get('call')).toBe(callId);
		if (callId === 'missing-call')
			expect(fixture.querySelector('button[data-frame][aria-pressed="true"]')).toBeNull();
	},
);

test('wide call hover previews detail without changing the pinned call or address', async (): Promise<void> => {
	await page.viewport(1600, 1000);
	const { document: fixture, frame } = await openFixture(
		'understanding-collection',
		'',
		'execution.html?stack=help-path&call=handle-key',
	);
	await expect.poll(() => fixture.querySelector('ex-call-stack[data-ready]')).not.toBeNull();
	const pinned = requiredElement(fixture, 'button[data-frame="0"]');
	const preview = requiredElement(fixture, 'button[data-frame="1"]');
	const address = fixture.URL;
	const opacity = getComputedStyle(pinned).opacity;
	await fixtureLocator(frame, preview).hover();
	await expect
		.poll(() => fixture.querySelector('.ex-design-frame-detail > small')?.textContent)
		.toContain('Preview');
	expect(pinned.getAttribute('aria-pressed')).toBe('true');
	expect(preview.getAttribute('aria-pressed')).toBe('false');
	expect(fixture.URL).toBe(address);
	expect(getComputedStyle(pinned).opacity).toBe(opacity);
	await frame
		.getByRole('heading', { name: 'From a call to the need it serves', exact: true })
		.hover();
	await expect
		.poll(() => fixture.querySelector('.ex-design-frame-detail > small')?.textContent)
		.toContain('Selected call');
	await fixtureLocator(frame, preview).click();
	expect(preview.getAttribute('aria-pressed')).toBe('true');
	expect(new URL(fixture.URL).searchParams.get('call')).toBe('input-owner-route');
});

test('a cold supporting-call address expands its own hidden group', async (): Promise<void> => {
	await page.viewport(1600, 1000);
	const { document: fixture } = await openFixture(
		'understanding',
		'',
		'execution.html?stack=start-session&call=event-bridge-forward',
	);
	await expect.poll(() => fixture.querySelector('ex-call-stack[data-ready]')).not.toBeNull();
	expect(requiredElement(fixture, 'button[data-frame="2"]').getAttribute('aria-pressed')).toBe(
		'true',
	);
	expect(requiredElement(fixture, 'ex-design-hidden-calls details').hasAttribute('open')).toBe(
		true,
	);
	expect(requiredElement(fixture, '.ex-design-frame-detail').textContent).toContain(
		'EventBridge.forward',
	);
	expect(new URL(fixture.URL).searchParams.get('call')).toBe('event-bridge-forward');
});

test.each(['standalone'])(
	'an unlinked Action frame keeps its authored category: %s',
	async (): Promise<void> => {
		await page.viewport(390, 1000);
		const { document: fixture, frame } = await openFixture(
			'document-browsers',
			'',
			'cross-page/execution.html',
		);
		await expect.poll(() => fixture.querySelector('button[data-frame="2"]')).not.toBeNull();
		await fixtureLocator(frame, requiredElement(fixture, 'button[data-frame="2"]')).click();
		const detail = requiredElement(fixture, '.ex-design-frame-detail');
		expect(requiredElement(detail, 'small').textContent).toContain('Action');
		expect(requiredElement(detail, 'pre code').textContent).toBe('finishBatch();');
		expect(detail.textContent).not.toContain('Supporting plumbing');
		expect(detail.textContent).toContain('No direct requirement link is declared');
		await frame.getByRole('button', { name: 'Close call details', exact: true }).click();
		await fixtureLocator(frame, requiredElement(fixture, 'ex-design-hidden-calls summary')).click();
		await fixtureLocator(frame, requiredElement(fixture, 'button[data-frame="1"]')).click();
		expect(requiredElement(detail, 'small').textContent).toContain('Plumbing');
		expect(detail.querySelector('.ex-design-frame-requirement a')).toBeNull();
		await frame.getByRole('button', { name: 'Close call details', exact: true }).click();
		await fixtureLocator(frame, requiredElement(fixture, 'button[data-frame="0"]')).click();
		expect(requiredElement(detail, 'small').textContent).toContain('Input');
		expect(
			new URL(
				requiredElement(detail, '.ex-design-frame-requirement a').getAttribute('href') ?? '',
				fixture.URL,
			).hash,
		).toBe('#need-body');
	},
);

test('picker search owns arrow input and Escape returns focus without changing the call', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('understanding', '', 'execution.html');
	await expect.poll(() => fixture.querySelector('ex-call-stack[data-ready]')).not.toBeNull();
	const picker = requiredElement(fixture, 'ex-design-picker details');
	const summary = requiredElement(picker, 'summary');
	await fixtureLocator(frame, summary).click();
	const input = requiredElement(picker, 'input');
	await expect.poll(() => fixture.activeElement).toBe(input);
	await frame
		.getByRole('searchbox', { name: 'Search call stacks', exact: true })
		.fill('no-matching-stack');
	expect(picker.querySelectorAll('button[data-picker-value]')).toHaveLength(0);
	const address = fixture.URL;
	await userEvent.keyboard('{ArrowDown}{ArrowRight}');
	expect(fixture.URL).toBe(address);
	expect(requiredElement(fixture, 'button[data-frame="0"]').getAttribute('aria-pressed')).toBe(
		'true',
	);
	expect(fixture.activeElement).toBe(input);
	await userEvent.keyboard('{Escape}');
	expect(picker.hasAttribute('open')).toBe(false);
	expect(fixture.activeElement).toBe(summary);
	expect(fixture.URL).toBe(address);
});

test('Left and Right switch standalone stacks while preserving each selected call address', async (): Promise<void> => {
	await page.viewport(1600, 1000);
	const { document: fixture, frame } = await openFixture('understanding', '', 'execution.html');
	await expect.poll(() => fixture.querySelector('ex-call-stack[data-ready]')).not.toBeNull();
	await fixtureLocator(frame, requiredElement(fixture, 'button[data-frame="0"]')).click();
	await userEvent.keyboard('{ArrowRight}');
	expect(new URL(fixture.URL).searchParams.get('stack')).toBe('open-help');
	expect(new URL(fixture.URL).searchParams.get('call')).toBe('help-panel-open');
	expect(requiredElement(fixture, 'ex-design-picker summary').textContent).toContain('Open Help');
	expect(fixture.activeElement).toBe(requiredElement(fixture, 'ex-design-call-view'));
	await userEvent.keyboard('{ArrowLeft}');
	expect(new URL(fixture.URL).searchParams.get('stack')).toBe('start-session');
	expect(new URL(fixture.URL).searchParams.get('call')).toBe('start-session-run');
	expect(requiredElement(fixture, 'ex-design-picker summary').textContent).toContain(
		'Start a session',
	);
});

test('current Cedar calls link to their preserved visible owner body', async (): Promise<void> => {
	await page.viewport(390, 844);
	const { document: fixture, frame } = await openFixture('cedar', '', 'execution.html');
	await expect.poll(() => fixture.querySelector('ex-call-stack[data-ready]')).not.toBeNull();
	expect(requiredElement(fixture, 'ex-call-stack [role="group"]').getAttribute('aria-label')).toBe(
		'Supplied Cedar import calls',
	);
	await fixtureLocator(frame, requiredElement(fixture, 'button[data-frame="0"]')).click();
	const detail = requiredElement(fixture, '.ex-design-frame-detail');
	expect(detail.textContent).toContain('CedarImport.read(source)');
	expect(requiredElement(detail, 'pre code').textContent).toBe(
		'const sourceText = await source.read();',
	);
	const link = requiredElement(detail, '.ex-design-frame-requirement a');
	expect(link.getAttribute('href')).toBe('requirements.html#keep-draft-body');
	await fixtureLocator(frame, link).click();
	const iframe = document.querySelector<HTMLIFrameElement>('iframe[data-testid="explainer"]');
	await expect
		.poll(() => iframe?.contentDocument?.getElementById('keep-draft-body')?.textContent)
		.toContain('If an import is rejected, keep the current note draft');
	const target = iframe?.contentDocument?.getElementById('keep-draft-body');
	if (!target) throw new Error('Missing visible Cedar need');
	expect(target.closest('template')).toBeNull();
	expect(target.getBoundingClientRect().height).toBeGreaterThan(0);
});
