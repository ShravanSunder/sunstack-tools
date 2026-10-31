import { afterEach, expect, test } from 'vitest';
import { page } from 'vitest/browser';

import {
	firstTextRect,
	fixtureLocator,
	insideViewport,
	openFixture,
	requiredElement,
} from './fixture-page.ts';

afterEach(async (): Promise<void> => {
	document.body.replaceChildren();
	await page.viewport(1600, 1000);
});

test('document disclosures retain Reading prose and Canvas visuals without changing island layouts', async (): Promise<void> => {
	const { document: fixture } = await openFixture('layout');
	const bounds = (selector: string): DOMRect =>
		requiredElement(fixture, selector).getBoundingClientRect();
	await expect
		.poll((): boolean => !!fixture.querySelector('#disclosure-code .ex-code-head'))
		.toBe(true);
	const verifyViewport = async (width: number): Promise<void> => {
		await page.viewport(width, 1000);
		const canvas = bounds('#disclosure-code');
		const headingStyle = fixture.defaultView?.getComputedStyle(
			requiredElement(fixture, '#disclosure-heading'),
		);
		if (!headingStyle) throw new Error('Missing disclosure heading style.');
		expect.soft(Number.parseFloat(headingStyle.paddingTop)).toBeGreaterThanOrEqual(24);
		expect.soft(Number.parseFloat(headingStyle.marginTop)).toBeGreaterThanOrEqual(40);
		for (const selector of [
			'#disclosure-heading',
			'#disclosure-prose',
			'#disclosure-list',
			'#nested-disclosure-prose',
		]) {
			const reading = bounds(selector);
			expect.soft(reading.width).toBeLessThanOrEqual(761);
			expect.soft(Math.abs(reading.left - canvas.left)).toBeLessThan(2);
			if (width === 1600) expect.soft(canvas.width - reading.width).toBeGreaterThan(100);
		}
		expect.soft(Math.abs(bounds('#disclosure-table').width - canvas.width)).toBeLessThan(2);
		const column = requiredElement(fixture, '#comparison-prose').parentElement;
		if (!column) throw new Error('Missing comparison column.');
		expect
			.soft(Math.abs(bounds('#comparison-prose').width - column.getBoundingClientRect().width))
			.toBeLessThan(2);
		expect.soft(fixture.documentElement.scrollWidth).toBeLessThanOrEqual(width + 1);
	};
	await verifyViewport(1600);
	await verifyViewport(390);
});

test('enlarging a Reading code block uses the inspection width and restores its page size', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('layout');
	const code = requiredElement(fixture, '#reading-code');
	code.scrollIntoView();
	await expect.poll((): boolean => !!code.querySelector('.ex-code-head')).toBe(true);
	const originalWidth = code.getBoundingClientRect().width;
	const originalContent = requiredElement(code, '.ex-code-head');
	expect(originalWidth).toBeCloseTo(760, 0);
	await fixtureLocator(frame, code).hover();
	await frame.getByRole('button', { name: 'Expand Short command', exact: true }).click();
	const modal = frame.getByRole('dialog', { name: 'Short command', exact: true });
	await expect.element(modal).toBeVisible();
	const content = requiredElement(fixture, '.ex-inspect-body > ex-code');
	const body = requiredElement(fixture, '.ex-inspect-body');
	const style = fixture.defaultView?.getComputedStyle(body);
	if (!style) throw new Error('Missing inspection style.');
	const availableWidth =
		body.clientWidth - Number.parseFloat(style.paddingLeft) - Number.parseFloat(style.paddingRight);
	expect.soft(content.getBoundingClientRect().width).toBeGreaterThan(originalWidth + 100);
	expect.soft(Math.abs(content.getBoundingClientRect().width - availableWidth)).toBeLessThan(2);
	await modal.getByRole('button', { name: 'Close', exact: true }).click();
	expect(code.getAttribute('data-width')).toBe('reading');
	expect(code.getBoundingClientRect().width).toBeCloseTo(originalWidth, 0);
	expect(requiredElement(code, '.ex-code-head')).toBe(originalContent);
});

test('sequence messages remain readable in both themes and on a narrow screen', async (): Promise<void> => {
	const { document: fixture } = await openFixture('ui', '', 'sequence-readability.html');
	const diagram = requiredElement(fixture, '#sequence');
	await expect.poll((): boolean => diagram.hasAttribute('prepared')).toBe(true);
	await fixture.fonts.ready;
	const message = Array.from(diagram.querySelectorAll<SVGTextElement>('text')).find(
		(text): boolean => text.textContent === 'return the same identity',
	);
	const view = fixture.defaultView;
	if (!message || !view) throw new Error('Missing rendered sequence message.');
	const canvas = fixture.createElement('canvas');
	const context = canvas.getContext('2d');
	if (!context) throw new Error('Missing colour measurement context.');
	const luminance = (colour: string): number => {
		context.clearRect(0, 0, 1, 1);
		context.fillStyle = colour;
		context.fillRect(0, 0, 1, 1);
		const channels = context.getImageData(0, 0, 1, 1).data;
		return [0.2126, 0.7152, 0.0722].reduce((total, weight, index): number => {
			const channel = (channels[index] ?? 0) / 255;
			return (
				total + weight * (channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4)
			);
		}, 0);
	};
	for (const theme of ['dark', 'light']) {
		fixture.documentElement.dataset['theme'] = theme;
		const foreground = luminance(view.getComputedStyle(message).fill);
		const background = luminance(
			view.getComputedStyle(fixture.documentElement).getPropertyValue('--surface-page'),
		);
		expect
			.soft((Math.max(foreground, background) + 0.05) / (Math.min(foreground, background) + 0.05))
			.toBeGreaterThanOrEqual(4.5);
	}
	const verifyViewport = async (width: number): Promise<void> => {
		await page.viewport(width, 1000);
		const renderedSize =
			Number.parseFloat(view.getComputedStyle(message).fontSize) * (message.getScreenCTM()?.a ?? 0);
		expect.soft(renderedSize).toBeGreaterThanOrEqual(15.9);
		expect(fixture.documentElement.scrollWidth).toBeLessThanOrEqual(width + 1);
	};
	await verifyViewport(1600);
	await verifyViewport(390);
});

async function verifyDiagramWidth(width: number): Promise<void> {
	await page.viewport(width, 1000);
	const { document: fixture, frame } = await openFixture('layout');
	const diagram = requiredElement(fixture, '#wide-diagram');
	diagram.scrollIntoView();
	await expect
		.poll((): string => diagram.querySelector('svg')?.textContent ?? '')
		.toContain('Report the observed outcome');
	const first = frame.getByText('Receive a complete request', { exact: false }).first();
	const last = frame.getByText('Report the observed outcome', { exact: false }).first();
	const firstNode = requiredElement(diagram, 'g.node[data-id="input"]');
	const lastNode = requiredElement(diagram, 'g.node[data-id="finish"]');
	expect(diagram.scrollWidth).toBeGreaterThan(diagram.clientWidth + 100);
	await expect.element(first).toBeVisible();
	diagram.scrollLeft = 0;
	expect(firstNode.getBoundingClientRect().left).toBeGreaterThanOrEqual(
		diagram.getBoundingClientRect().left - 2,
	);
	await fixtureLocator(frame, diagram).hover();
	const controls = requiredElement(diagram, '.ex-inspect-controls');
	expect(insideViewport(controls)).toBe(true);
	diagram.scrollLeft = diagram.scrollWidth;
	await expect
		.poll(
			(): boolean =>
				lastNode.getBoundingClientRect().right <= diagram.getBoundingClientRect().right + 2,
		)
		.toBe(true);
	await expect.element(last).toBeVisible();
	expect(insideViewport(controls)).toBe(true);
	expect(fixture.documentElement.scrollWidth).toBeLessThanOrEqual(width + 2);
	await frame.getByRole('button', { name: 'Expand Wide comparison', exact: true }).click();
	await expect
		.element(frame.getByRole('dialog', { name: 'Wide comparison', exact: true }))
		.toBeVisible();
	await frame.getByRole('dialog').getByRole('button', { name: 'Close', exact: true }).click();
	document.body.replaceChildren();
}

test('wide diagrams scroll locally to both endpoints with reachable controls and no outer overflow', async (): Promise<void> => {
	await verifyDiagramWidth(1600);
	await verifyDiagramWidth(390);
});

test('tree multiline text stays beside its first-line icon and preserves inline code and loose paragraphs', async (): Promise<void> => {
	const { document: fixture, frame } = await openFixture('layout');
	const tree = requiredElement(fixture, '#tight-labels');
	tree.scrollIntoView();
	await expect
		.poll((): string => tree.querySelector('.ex-tree-node')?.textContent ?? '')
		.toContain('Short and multiline labels');
	for (const text of ['Short label', 'A deliberately long label', 'Keep']) {
		const label = Array.from(tree.querySelectorAll('.ex-tree-label')).find((element): boolean =>
			(element.textContent ?? '').trim().startsWith(text),
		);
		if (!label) throw new Error(`Missing tree scenario: ${text}`);
		const row = label.parentElement;
		if (!row) throw new Error('Missing tree row.');
		const icon = requiredElement(row, '.ex-tone-icon');
		const textRect = firstTextRect(label);
		const iconRect = icon.getBoundingClientRect();
		expect(Math.abs(textRect.top - iconRect.top)).toBeLessThan(10);
		expect(textRect.left).toBeGreaterThanOrEqual(iconRect.right);
		if (text === 'A deliberately long label')
			expect(label.getBoundingClientRect().height).toBeGreaterThan(40);
	}
	const code = requiredElement(tree, 'code');
	const codeRect = code.getBoundingClientRect();
	const label = requiredElement(tree, 'code').closest('.ex-tree-label');
	if (!label) throw new Error('Inline code lost its label.');
	expect(Math.abs(codeRect.top - firstTextRect(label).top)).toBeLessThan(10);
	const loose = requiredElement(fixture, '#loose-labels');
	await expect
		.poll((): number => loose.querySelectorAll('.ex-tree-label p').length)
		.toBeGreaterThan(1);
	const paragraphs = loose.querySelectorAll('.ex-tree-label p');
	const firstParagraph = paragraphs[0];
	const secondParagraph = paragraphs[1];
	if (!firstParagraph || !secondParagraph) throw new Error('Loose paragraphs were flattened.');
	expect(secondParagraph.textContent).toContain('A second paragraph retains');
	expect(secondParagraph.getBoundingClientRect().top).toBeGreaterThan(
		firstParagraph.getBoundingClientRect().bottom + 3,
	);
	expect(
		Math.abs(
			secondParagraph.getBoundingClientRect().left - firstParagraph.getBoundingClientRect().left,
		),
	).toBeLessThan(2);
	await frame.getByText('A deliberately long label', { exact: false }).click();
	await expect
		.element(frame.getByText('A nested child has the same alignment', { exact: false }))
		.toBeVisible();
	const branchLabel = Array.from(tree.querySelectorAll('.ex-tree-label')).find(
		(element): boolean =>
			element.textContent?.trim().startsWith('A deliberately long label') ?? false,
	);
	const branch = branchLabel?.closest('.ex-tree-node');
	if (!branch) throw new Error('Missing tree branch.');
	const disclosure = fixtureLocator(frame, requiredElement(branch, '.ex-tree-disclosure'));
	await disclosure.click();
	await expect
		.element(frame.getByText('A nested child has the same alignment', { exact: false }))
		.not.toBeVisible();
	await disclosure.click();
	await expect
		.element(frame.getByText('A nested child has the same alignment', { exact: false }))
		.toBeVisible();
});

test('narrow enlargement hides only sidebar controls, fills the viewport and restores live content', async (): Promise<void> => {
	await page.viewport(390, 900);
	const { document: fixture, frame } = await openFixture('layout');
	const code = requiredElement(fixture, '#reading-code');
	await expect.poll((): boolean => !!code.querySelector('.ex-code-head')).toBe(true);
	code.scrollIntoView();
	const head = requiredElement(code, '.ex-code-head');
	const sidebar = requiredElement(code, '.ex-inspect-sidebar');
	expect(fixture.defaultView?.getComputedStyle(sidebar).display).toBe('none');
	await frame.getByRole('button', { name: 'Expand Short command', exact: true }).click();
	const modal = frame.getByRole('dialog', { name: 'Short command', exact: true });
	await expect.element(modal).toBeVisible();
	const bounds = requiredElement(fixture, '.ex-inspect-dialog').getBoundingClientRect();
	expect(bounds.left).toBeCloseTo(0, 0);
	expect(bounds.width).toBeCloseTo(390, 0);
	expect(bounds.height).toBeCloseTo(900, 0);
	await modal.getByRole('button', { name: 'Close', exact: true }).click();
	expect(requiredElement(code, '.ex-code-head')).toBe(head);
	await page.viewport(1600, 1000);
	await expect
		.poll((): string | undefined => fixture.defaultView?.getComputedStyle(sidebar).display)
		.not.toBe('none');
});

test('swipe and free-board read canvases keep initially visible card rails inside their scroll clip', async (): Promise<void> => {
	await page.viewport(390, 900);
	const { document: fixture, frame } = await openFixture('layout');
	const documentRoot = requiredElement(fixture, 'ex-document');
	const examples = fixture.createElement('section');
	examples.innerHTML =
		'<ex-grid id="rail-swipe" swipe><ex-card title="First" tone="note" zoom><p>First card.</p></ex-card><ex-card title="Second" tone="note" zoom><p>Second card.</p></ex-card></ex-grid><ex-board id="rail-free"><div style="position:relative;width:700px;height:250px"><ex-card style="position:absolute;top:0;left:0;width:280px" title="Placed card" tone="note" zoom><p>Placed content.</p></ex-card></div></ex-board>';
	documentRoot.append(examples);
	await expect
		.poll(
			(): number => examples.querySelectorAll('.ex-inspect-controls > .ex-inspect-trigger').length,
		)
		.toBe(6);
	await expect
		.poll((): number => examples.querySelectorAll('ex-card > .ex-entity-comment').length)
		.toBe(3);
	for (const selector of ['#rail-swipe', '#rail-free > .ex-board-canvas']) {
		const clip = requiredElement(examples, selector);
		const card = requiredElement(clip, 'ex-card');
		clip.scrollIntoView({ block: 'center' });
		expect(card.querySelectorAll(':scope > .ex-entity-comment')).toHaveLength(1);
		const top = clip.getBoundingClientRect().top;
		for (const control of card.querySelectorAll(
			'.ex-inspect-controls > button, :scope > .ex-entity-comment',
		)) {
			if (control.getBoundingClientRect().width === 0) continue;
			expect(control.getBoundingClientRect().top).toBeGreaterThanOrEqual(top);
		}
	}
	const board = requiredElement(examples, '#rail-free');
	await fixtureLocator(frame, board).getByRole('button', { name: 'Map view', exact: true }).click();
	await expect.poll((): string | null => board.getAttribute('data-mode')).toBe('map');
	const placed = requiredElement(board, 'ex-card');
	await expect
		.poll(
			(): number =>
				requiredElement(
					placed,
					'.ex-inspect-trigger:not(.ex-inspect-sidebar)',
				).getBoundingClientRect().top - board.getBoundingClientRect().top,
		)
		.toBeGreaterThanOrEqual(0);
});

test('four tabs, Markdown first-heading text and multi-file paths remain clear of floating controls', async (): Promise<void> => {
	await page.viewport(390, 900);
	const { document: fixture } = await openFixture('layout');
	const examples = fixture.createElement('section');
	examples.innerHTML =
		'<ex-tabs id="four-tabs" label="Four views" zoom><section label="Design">First</section><section label="Build">Second</section><section label="Inspect">Third</section><section label="Deliver">Fourth</section></ex-tabs><ex-markdown id="first-heading"><script type="text/markdown"># A heading for the whole delivery</script></ex-markdown><ex-card id="headed-card" title="Card body" tone="note"><ex-markdown><script type="text/markdown"># First body heading</script></ex-markdown></ex-card><ex-board><ex-region id="headed-region" title="Region body" tone="note"><ex-markdown><script type="text/markdown"># First region heading</script></ex-markdown></ex-region></ex-board><ex-code id="long-path-diff" lang="diff"><script type="text/plain">diff --git a/verylongunbrokencomponentsegmentthatdoesnotfitinanarrowcolumn/design.ts b/verylongunbrokencomponentsegmentthatdoesnotfitinanarrowcolumn/design.ts\n--- a/verylongunbrokencomponentsegmentthatdoesnotfitinanarrowcolumn/design.ts\n+++ b/verylongunbrokencomponentsegmentthatdoesnotfitinanarrowcolumn/design.ts\n@@ -1 +1 @@\n-old\n+new\ndiff --git a/builder.ts b/builder.ts\n--- a/builder.ts\n+++ b/builder.ts\n@@ -1 +1 @@\n-old\n+new</script></ex-code>';
	requiredElement(fixture, 'ex-document').append(examples);
	await expect
		.poll((): boolean => requiredElement(examples, '#first-heading').hasAttribute('prepared'))
		.toBe(true);
	await expect
		.poll((): number => examples.querySelectorAll('#four-tabs [role="tab"]').length)
		.toBe(4);
	for (const selector of ['#four-tabs [role="tab"]', '#first-heading h1']) {
		for (const label of examples.querySelectorAll(selector)) {
			const owner = label.closest('ex-tabs,ex-markdown');
			if (!owner) throw new Error('Missing control owner');
			const range = fixture.createRange();
			range.selectNodeContents(label);
			const control = requiredElement(
				owner,
				'.ex-inspect-controls > .ex-inspect-trigger:not(.ex-inspect-sidebar)',
			).getBoundingClientRect();
			for (const text of range.getClientRects()) {
				expect(
					Math.min(text.right, control.right) > Math.max(text.left, control.left) &&
						Math.min(text.bottom, control.bottom) > Math.max(text.top, control.top),
					`${selector} text overlaps ${owner.localName} rail`,
				).toBe(false);
			}
		}
	}
	for (const selector of ['#headed-card', '#headed-region']) {
		const owner = requiredElement(examples, selector);
		const band = requiredElement(owner, '.ex-card-head,.ex-region-head');
		const heading = requiredElement(owner, 'ex-markdown h1');
		expect(
			Math.abs(heading.getBoundingClientRect().top - band.getBoundingClientRect().bottom - 20),
			selector + ' retains one body inset',
		).toBeLessThan(1);
		const source = requiredElement(
			owner,
			'ex-markdown > .ex-inspect-controls',
		).getBoundingClientRect();
		const range = fixture.createRange();
		range.selectNodeContents(heading);
		for (const text of range.getClientRects()) {
			expect(
				Math.min(text.right, source.right) > Math.max(text.left, source.left) &&
					Math.min(text.bottom, source.bottom) > Math.max(text.top, source.top),
				selector + ' heading clears source rail',
			).toBe(false);
		}
	}
	const code = requiredElement(examples, '#long-path-diff');
	await expect.poll((): number => code.querySelectorAll('.ex-code-file').length).toBe(2);
	for (const path of code.querySelectorAll('.ex-code-file > summary')) {
		expect(path.scrollWidth).toBeLessThanOrEqual(code.clientWidth + 1);
	}
});

test('bare Markdown section headings retain document margin rhythm and clear source controls', async (): Promise<void> => {
	await page.viewport(390, 900);
	const { document: fixture } = await openFixture('layout');
	const example = fixture.createElement('ex-document');
	example.id = 'section-rhythm';
	example.innerHTML = [2, 3, 4, 5, 6]
		.map(
			(level): string =>
				`<ex-card id="before-heading-${level}" title="Before section" tone="note"><p>Existing content.</p></ex-card>\n\n${'#'.repeat(level)} Section heading ${level} for the delivery\n\nSection content.\n\n`,
		)
		.join('');
	fixture.body.append(example);
	await expect.poll((): number => example.querySelectorAll('ex-markdown[prepared]').length).toBe(5);
	const section = requiredElement(example, 'ex-markdown h2');
	const preceding = requiredElement(example, '#before-heading-2');
	expect(
		section.getBoundingClientRect().top - preceding.getBoundingClientRect().bottom,
	).toBeCloseTo(40, 0);
	for (const heading of example.querySelectorAll('ex-markdown > :is(h2,h3,h4,h5,h6)')) {
		const markdown = heading.parentElement;
		if (!markdown) throw new Error('Missing Markdown owner');
		expect(fixture.defaultView?.getComputedStyle(markdown).paddingTop).toBe('0px');
		const control = requiredElement(markdown, '.ex-inspect-controls').getBoundingClientRect();
		const range = fixture.createRange();
		range.selectNodeContents(heading);
		for (const text of range.getClientRects()) {
			expect(
				Math.min(text.right, control.right) > Math.max(text.left, control.left) &&
					Math.min(text.bottom, control.bottom) > Math.max(text.top, control.top),
				heading.tagName + ' clears source rail',
			).toBe(false);
		}
	}
});
