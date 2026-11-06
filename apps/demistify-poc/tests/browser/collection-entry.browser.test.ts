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
		.poll(() => fixture.querySelector('.lab-document-subject small')?.textContent)
		.toBe('FirstNeed · Input ownership');
	expect(fixture.querySelector('[role="alert"]')).toBeNull();
});

test('standalone calls start from the first authored stack and frame', async (): Promise<void> => {
	const { document: fixture } = await openFixture('understanding-collection', '', 'execution.html');
	await expect
		.poll(() => fixture.querySelector('button[data-frame="0"]')?.getAttribute('aria-pressed'))
		.toBe('true');
	expect(requiredElement(fixture, 'lab-execution-review summary').textContent).toContain(
		'Open Help',
	);
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
	expect(requiredElement(fixture, '.lab-related-content').textContent).toContain(
		'No call is mapped to this need',
	);
	expect(fixture.querySelector('button[data-frame][aria-pressed="true"]')).toBeNull();
	await fixtureLocator(frame, requiredElement(fixture, 'ex-drawer button[data-frame="0"]')).click();
	expect(fixture.querySelector('button[data-frame="0"]')?.getAttribute('aria-pressed')).toBe(
		'true',
	);
	expect(new URL(fixture.URL).searchParams.get('need')).toBe('U2');
});
