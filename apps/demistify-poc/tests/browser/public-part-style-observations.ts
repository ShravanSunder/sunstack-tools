import { expect } from 'vitest';
import { commands, page } from 'vitest/browser';

import { browserScreenshotPath, writeBrowserEvidence } from './browser-evidence.ts';
import { fixtureLocator, openFixture, requiredElement } from './fixture-page.ts';
import { publicPartStyleReference } from './public-part-style-reference.ts';

/** Public parts must keep the selected pre-fix styles, independently of inspector behavior. */
export async function captureFixedPublicPartStyles(props: {
	readonly width: number;
	readonly theme: string;
	readonly motion: string;
}): Promise<void> {
	const expected = publicPartStyleReference.profiles.find(
		(profile) =>
			profile.width === props.width &&
			profile.theme === props.theme &&
			profile.motion === props.motion,
	);
	if (!expected) throw new Error('Missing fixed selected-before style profile.');
	await page.viewport(props.width, props.width === 390 ? 844 : 1000);
	await commands.setReducedMotion(props.motion === 'reduce' ? 'reduce' : 'no-preference');
	const { document: fixture, frame } = await openFixture(
		'document-browsers',
		'',
		'public-style/requirements.html?need=U2&stage=program-design',
	);
	fixture.documentElement.dataset['theme'] = props.theme;
	await expect
		.poll(() => fixture.querySelector('.ex-design-stage-record ex-mermaid svg'))
		.not.toBeNull();
	await ['Program Design', 'Specification', 'Verification'].reduce(
		async (previous, stage): Promise<void> => {
			await previous;
			await frame.getByRole('button', { name: stage, exact: true }).click();
			await Promise.all(
				fixture.getAnimations().map((animation) => animation.finished.catch(() => undefined)),
			);
			const panel = requiredElement(fixture, '.ex-design-document-panel');
			const samples = expected.samples.filter((sample) => sample.stage === stage);
			expect(samples.length).toBeGreaterThan(0);
			const actual = [...new Set(samples.map((sample) => sample.selector))].flatMap((selector) =>
				[...panel.querySelectorAll(selector)].map((element) => {
					const style = getComputedStyle(element);
					return {
						stage,
						selector,
						display: style.display,
						margin: style.margin,
						padding: style.padding,
						color: style.color,
						background: style.backgroundColor,
						font: style.font,
						radius: style.borderRadius,
					};
				}),
			);
			expect(
				actual,
				`${stage} public parts against retained class-bearing selected styles`,
			).toEqual(samples);
		},
		Promise.resolve(),
	);
}

function publicStyleSamples(scope: Element, stage: string): readonly object[] {
	const selectors = [
		'[data-ex-part="sketch"]',
		'[data-ex-part="sketch"] > figcaption',
		'[data-ex-part="sketch"] ~ details:has(> .ex-table-scroll)',
		'[data-ex-part="cases"]',
		'[data-ex-part="cases"] > figcaption',
		'[data-ex-part="procedure"]',
		'[data-ex-part="procedure"] summary',
		'.ex-design-example-outcome > small + p',
	];
	return selectors.flatMap((selector) =>
		[...scope.querySelectorAll(selector)].map((element) => {
			const style = getComputedStyle(element);
			return {
				stage,
				selector,
				display: style.display,
				margin: style.margin,
				padding: style.padding,
				color: style.color,
				background: style.backgroundColor,
				font: style.font,
				radius: style.borderRadius,
				width: element.getBoundingClientRect().width,
			};
		}),
	);
}

/** Same-content fixtures separate the public recipe from legacy author classes. */
export async function capturePublicStyleParity(props: {
	readonly width: number;
	readonly theme: string;
	readonly motion: string;
}): Promise<void> {
	const { width, theme, motion } = props;
	await page.viewport(width, width === 390 ? 844 : 1000);
	await commands.setReducedMotion(motion === 'reduce' ? 'reduce' : 'no-preference');
	const samples: string[] = [];
	await ['understanding', 'document-browsers'].reduce(async (previous, name): Promise<void> => {
		await previous;
		document.body.replaceChildren();
		const file = `${name === 'understanding' ? '' : 'public-style/'}requirements.html?need=U2&stage=program-design`;
		const { document: fixture, frame } = await openFixture(
			name === 'understanding' ? 'understanding' : 'document-browsers',
			'',
			file,
		);
		fixture.documentElement.dataset['theme'] = theme;
		await expect
			.poll(() => fixture.querySelector('.ex-design-stage-record ex-mermaid svg'))
			.not.toBeNull();
		const wrapper = requiredElement(fixture, 'ex-zoom:has(> ex-requirements)');
		const sidebar = requiredElement(wrapper, ':scope > .ex-inspect-controls .ex-inspect-sidebar');
		expect(getComputedStyle(sidebar).display).toBe('none');
		const styles: object[] = [];
		await ['Program Design', 'Specification', 'Verification'].reduce(
			async (prior, stage): Promise<void> => {
				await prior;
				await frame.getByRole('button', { name: stage, exact: true }).click();
				await Promise.all(
					fixture.getAnimations().map((animation) => animation.finished.catch(() => undefined)),
				);
				const panel = requiredElement(fixture, '.ex-design-document-panel');
				styles.push(...publicStyleSamples(panel, stage));
				await page.screenshot({
					path: browserScreenshotPath(
						`public-parity-${width}-${theme}-${motion}-${name}-${stage}.png`,
					),
				});
			},
			Promise.resolve(),
		);
		await frame.getByRole('button', { name: 'View together', exact: true }).click();
		const workspace = requiredElement(fixture, 'ex-drawer.ex-design-related-drawer');
		await expect.poll(() => workspace.matches(':popover-open')).toBe(true);
		await Promise.all(
			fixture.getAnimations().map((animation) => animation.finished.catch(() => undefined)),
		);
		styles.push(...publicStyleSamples(workspace, 'Related'));
		await page.screenshot({
			path: browserScreenshotPath(`public-parity-${width}-${theme}-${motion}-${name}-related.png`),
		});
		await frame.getByRole('button', { name: 'Close related documents', exact: true }).click();
		samples.push(JSON.stringify(styles));
		await writeBrowserEvidence(
			`public-parity-${width}-${theme}-${motion}-${name}.json`,
			JSON.stringify({ width, theme, motion, name, styles }, null, 2),
		);
		if (name === 'document-browsers') {
			const reader = requiredElement(fixture, 'ex-requirements');
			const expand = requiredElement(
				wrapper,
				':scope > .ex-inspect-controls button:not(.ex-inspect-sidebar)',
			);
			await fixtureLocator(frame, expand).click();
			const dialog = requiredElement(fixture, '.ex-inspect-dialog');
			await expect.poll(() => dialog.matches(':modal')).toBe(true);
			expect(requiredElement(dialog, 'ex-requirements')).toBe(reader);
			if (width === 390) {
				const detail = requiredElement(dialog, '.ex-design-requirement-detail');
				expect(getComputedStyle(detail).borderTopWidth).toBe('0px');
				expect(detail.getBoundingClientRect().right).toBeLessThanOrEqual(width);
			}
			await page.screenshot({
				path: browserScreenshotPath(`public-inspector-${width}-${theme}-${motion}.png`),
			});
			await frame
				.getByRole('dialog', { name: 'User requirements', exact: true })
				.getByRole('button', { name: /^Close/ })
				.click();
			await expect.poll(() => fixture.querySelector('.ex-inspect-dialog')).toBeNull();
			expect(wrapper.querySelector('ex-requirements')).toBe(reader);
			await expect.poll(() => fixture.activeElement).toBe(expand);
			await expect
				.poll(() => fixture.querySelector('.ex-design-example-outcome p [data-comment-control]'))
				.not.toBeNull();
			await fixtureLocator(
				frame,
				requiredElement(fixture, '.ex-design-example-outcome p [data-comment-control]'),
			).click();
			await frame
				.getByRole('textbox', { name: 'Your comment', exact: true })
				.fill('Check the public reader surface.');
			expect(fixture.activeElement?.getAttribute('aria-label')).toBe('Your comment');
			await frame.getByRole('textbox', { name: 'Your comment', exact: true }).fill('');
			await frame.getByRole('button', { name: 'Close comments', exact: true }).click();
		}
	}, Promise.resolve());
	expect(samples[1]).toBe(samples[0]);
}
