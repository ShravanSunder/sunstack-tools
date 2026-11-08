import { afterEach, expect, test } from 'vitest';
import { page } from 'vitest/browser';

import { fixtureLocator, openFixture, requiredElement } from './fixture-page.ts';

afterEach((): void => document.body.replaceChildren());

test('the need browser starts at the first authored need without U1', async (): Promise<void> => {
	const { document: fixture } = await openFixture(
		'understanding-collection',
		'',
		'requirements.html',
	);
	await expect
		.poll(() => fixture.querySelector('.ex-design-document-subject small')?.textContent)
		.toBe('FirstNeed · Input ownership');
	expect(fixture.querySelector('[role="alert"]')).toBeNull();
});

test('standalone calls start from the first authored stack and frame', async (): Promise<void> => {
	const { document: fixture } = await openFixture('understanding-collection', '', 'execution.html');
	await expect
		.poll(() => fixture.querySelector('button[data-frame="0"]')?.getAttribute('aria-pressed'))
		.toBe('true');
	expect(requiredElement(fixture, 'ex-call-stack summary').textContent).toContain('Open Help');
	expect(fixture.querySelector('[role="alert"]')).toBeNull();
});

test('an unmapped need keeps its gap while the authored stack remains usable', async (): Promise<void> => {
	await page.viewport(1600, 1000);
	const { document: fixture, frame } = await openFixture(
		'understanding-collection',
		'',
		'requirements.html?need=U2&stage=program-design',
	);
	await frame.getByRole('button', { name: 'Open call stack', exact: true }).click();
	await expect.poll(() => fixture.querySelector('ex-drawer button[data-frame="0"]')).not.toBeNull();
	expect(requiredElement(fixture, '.ex-design-related-content').textContent).toContain(
		'No call is mapped to this need',
	);
	expect(fixture.querySelector('button[data-frame][aria-pressed="true"]')).toBeNull();
	await fixtureLocator(frame, requiredElement(fixture, 'ex-drawer button[data-frame="0"]')).click();
	expect(fixture.querySelector('button[data-frame="0"]')?.getAttribute('aria-pressed')).toBe(
		'true',
	);
	expect(new URL(fixture.URL).searchParams.get('need')).toBe('U2');
});

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

test.each([1600, 390])(
	'rich Need body survives Overview and Related at %ipx',
	async (width): Promise<void> => {
		await page.viewport(width, 1000);
		const { document: fixture, frame } = await openFixture(
			'document-browsers',
			'',
			'full/requirements.html?need=DraftNeed',
		);
		await expect.poll(() => fixture.querySelector('ex-requirements[data-ready]')).not.toBeNull();
		const overview = requiredElement(fixture, '.ex-design-document-panel');
		expect(overview.textContent).toContain(
			'Distinct need-body detail: the draft remains editable for retry after refusal.',
		);
		const overviewLink = requiredElement(overview, 'a');
		const overviewTarget = requiredElement(overview, 'p[id] + p[id]');
		await activateLocalLink({ fixture, frame, link: overviewLink, target: overviewTarget });
		expect(overviewTarget.isConnected).toBe(true);
		await frame.getByRole('button', { name: 'View together', exact: true }).click();
		const related = requiredElement(fixture, '.ex-design-related-content');
		expect(related.textContent).toContain(
			'Distinct need-body detail: the draft remains editable for retry after refusal.',
		);
		const relatedLink = requiredElement(related, 'a');
		const relatedTarget = requiredElement(related, 'p[id] + p[id]');
		expect(relatedTarget.id).not.toBe(overviewTarget.id);
		await activateLocalLink({ fixture, frame, link: relatedLink, target: relatedTarget });
		expect(relatedTarget.isConnected).toBe(true);
		expect(
			requiredElement(fixture, 'ex-drawer.ex-design-related-drawer').matches(':popover-open'),
		).toBe(true);
	},
);

test.each([1600, 390])(
	'encoded fragment activates its own target in both projections at %ipx',
	async (width): Promise<void> => {
		await page.viewport(width, 1000);
		const { document: fixture, frame } = await openFixture(
			'document-browsers',
			'',
			'full/requirements.html?need=DraftNeed&stage=specification',
		);
		await expect.poll(() => fixture.querySelector('ex-requirements[data-ready]')).not.toBeNull();
		await frame.getByRole('button', { name: 'View together', exact: true }).click();
		const rule = requiredElement(fixture, '.ex-design-rule-layout');
		const preview = requiredElement(rule, '[data-ex-part="rule"]');
		const previewTarget = requiredElement(preview, 'input + p[id]');
		expect(requiredElement(preview, '.malformed-note').getAttribute('href')).toBe('#%ZZ');
		expect(requiredElement(preview, 'a[href^="https:"]').getAttribute('href')).toBe(
			'https://example.org/reference#outside',
		);
		await activateLocalLink({
			fixture,
			frame,
			link: requiredElement(preview, '.encoded-note'),
			target: previewTarget,
		});
		expect(previewTarget.isConnected).toBe(true);
		const repeatedAddress = fixture.URL;
		await fixtureLocator(frame, requiredElement(preview, '.encoded-note')).click();
		expect(fixture.URL).toBe(repeatedAddress);
		expect(previewTarget.isConnected).toBe(true);
		await frame.getByText('Source details · DraftRule', { exact: true }).click();
		const source = requiredElement(rule, '.ex-design-record-source [data-ex-part="rule"]');
		const sourceTarget = requiredElement(source, 'input + p[id]');
		expect(sourceTarget.id).not.toBe(previewTarget.id);
		await activateLocalLink({
			fixture,
			frame,
			link: requiredElement(source, '.encoded-note'),
			target: sourceTarget,
		});
		expect(sourceTarget.isConnected).toBe(true);
	},
);

test.each([1600, 390])(
	'need/stage browser history and mobile Back preserve the selected identity at %ipx',
	async (width): Promise<void> => {
		await page.viewport(width, 1000);
		const { document: fixture, frame } = await openFixture(
			'understanding-collection',
			'',
			'requirements.html',
		);
		await expect.poll(() => fixture.querySelector('ex-requirements[data-ready]')).not.toBeNull();
		await fixtureLocator(
			frame,
			requiredElement(fixture, '.ex-design-requirement-list [data-need="U2"]'),
		).click();
		await frame.getByRole('button', { name: 'Program Design', exact: true }).click();
		expect(new URL(fixture.URL).searchParams.get('need')).toBe('U2');
		expect(new URL(fixture.URL).searchParams.get('stage')).toBe('program-design');
		const view = fixture.defaultView;
		if (!view) throw new Error('Missing history owner');
		view.history.back();
		await expect
			.poll(
				() =>
					fixture.querySelector('.ex-design-document-switcher [aria-pressed="true"]')?.textContent,
			)
			.toBe('Overview');
		expect(new URL(fixture.URL).searchParams.get('need')).toBe('U2');
		expect(new URL(fixture.URL).searchParams.has('stage')).toBe(false);
		view.history.forward();
		await expect
			.poll(
				() =>
					fixture.querySelector('.ex-design-document-switcher [aria-pressed="true"]')?.textContent,
			)
			.toBe('Program Design');
		expect(new URL(fixture.URL).searchParams.get('need')).toBe('U2');
		if (width === 390) {
			await frame.getByRole('button', { name: 'Back to needs', exact: true }).click();
			expect(new URL(fixture.URL).searchParams.has('need')).toBe(false);
			const selected = requiredElement(fixture, '.ex-design-requirement-list [data-need="U2"]');
			expect(selected.getAttribute('aria-current')).toBe('true');
			await expect.poll(() => fixture.activeElement).toBe(selected);
			await fixtureLocator(frame, selected).click();
			expect(new URL(fixture.URL).searchParams.get('need')).toBe('U2');
		}
	},
);

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

interface LocalLinkAction {
	readonly fixture: Document;
	readonly frame: ReturnType<typeof page.frameLocator>;
	readonly link: Element;
	readonly target: Element;
}
async function activateLocalLink({ fixture, frame, link, target }: LocalLinkAction): Promise<void> {
	const view = fixture.defaultView;
	if (!view) throw new Error('Missing local navigation window');
	const unchanged = new URL(link.getAttribute('href') ?? '', fixture.URL).href === fixture.URL;
	const navigation = unchanged
		? Promise.resolve()
		: new Promise<void>((resolve) =>
				view.addEventListener('hashchange', () => resolve(), { once: true }),
			);
	await fixtureLocator(frame, link).click();
	await navigation;
	expect(decodeURIComponent(new URL(fixture.URL).hash.slice(1))).toBe(target.id);
}

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

test.each(['specification', 'program-design', 'verification'])(
	'cross-page stage IDs retain their own native targets: %s',
	async (stage): Promise<void> => {
		await page.viewport(390, 1000);
		const { document: fixture, frame } = await openFixture(
			'document-browsers',
			'',
			`cross-page/requirements.html?need=BoundaryNeed&stage=${stage}`,
		);
		await expect.poll(() => fixture.querySelector('.ex-design-stage-record')).not.toBeNull();
		const record = requiredElement(fixture, '.ex-design-stage-record');
		const input = requiredElement(record, '[data-local-input]');
		const label = requiredElement(record, '[data-local-label]');
		const target = requiredElement(record, '[data-local-target]');
		await fixtureLocator(frame, label).click();
		expect(fixture.activeElement).toBe(input);
		expect(fixture.getElementById(label.getAttribute('for') ?? '')).toBe(input);
		const labelledBy = input.getAttribute('aria-labelledby')?.split(' ') ?? [];
		expect(labelledBy).toHaveLength(3);
		expect(fixture.getElementById(labelledBy[0] ?? '')).toBe(label);
		expect(fixture.getElementById(labelledBy[1] ?? '')).toBe(
			record.querySelector('[data-local-extra]'),
		);
		expect(labelledBy[2]).toBe('outside-label');
		const describedBy = input.getAttribute('aria-describedby')?.split(' ') ?? [];
		expect(fixture.getElementById(describedBy[0] ?? '')).toBe(target);
		expect(describedBy[1]).toBe('outside-description');
		await activateLocalLink({
			fixture,
			frame,
			link: requiredElement(record, '[data-local-link]'),
			target,
		});
		await activateLocalLink({
			fixture,
			frame,
			link: requiredElement(record, '[data-encoded-link]'),
			target,
		});
		expect(target.isConnected).toBe(true);
		expect(requiredElement(record, '[data-malformed-link]').getAttribute('href')).toBe('#%ZZ');
		expect(requiredElement(record, '[data-external-link]').getAttribute('href')).toBe(
			'https://example.org/reference#outside',
		);
		if (stage === 'verification') {
			const summary = requiredElement(record, 'p[aria-describedby]');
			expect(fixture.getElementById(summary.getAttribute('aria-describedby') ?? '')).toBe(
				record.querySelector('[data-ex-part="procedure"]'),
			);
			await activateLocalLink({
				fixture,
				frame,
				link: requiredElement(record, '[data-procedure-link]'),
				target: requiredElement(record, '[data-ex-part="procedure"]'),
			});
			const observed = requiredElement(record, '.ex-design-example-outcome');
			await activateLocalLink({
				fixture,
				frame,
				link: requiredElement(observed, '[data-observation-link]'),
				target: requiredElement(observed, 'p[id] + p[id]'),
			});
		}
		const ids = [...fixture.querySelectorAll('[id]')].map((element) => element.id);
		expect(new Set(ids).size).toBe(ids.length);
	},
);

test('an owner-page stage keeps its authored raw deep-link target', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture(
		'document-browsers',
		'',
		'cross-page/specification.html?need=BoundaryNeed&stage=specification#shared-target',
	);
	await expect.poll(() => fixture.querySelector('.ex-design-stage-record')).not.toBeNull();
	const record = requiredElement(fixture, '.ex-design-stage-record');
	const target = requiredElement(record, '[data-local-target]');
	expect(target.id).toBe('shared-target');
	expect(fixture.getElementById('shared-target')).toBe(target);
	await activateLocalLink({
		fixture,
		frame,
		link: requiredElement(record, '[data-encoded-link]'),
		target,
	});
});

test.each(['standalone', 'contextual'])(
	'an unlinked Action frame keeps its authored category: %s',
	async (entry): Promise<void> => {
		await page.viewport(390, 1000);
		const { document: fixture, frame } = await openFixture(
			'document-browsers',
			'',
			entry === 'standalone'
				? 'cross-page/execution.html'
				: 'cross-page/requirements.html?need=BoundaryNeed&stage=program-design',
		);
		if (entry === 'contextual')
			await frame.getByRole('button', { name: 'Open call stack', exact: true }).click();
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
			).searchParams.get('need'),
		).toBe('BoundaryNeed');
	},
);
