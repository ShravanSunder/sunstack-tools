import type { BrowserCommand } from 'vitest/node';

/** Vitest's native dragAndDrop targets HTML drag/drop; this gesture needs real pointer capture. */
export const panInspection: BrowserCommand = async (context): Promise<void> => {
	if (context.provider.name !== 'playwright') throw new Error('Pointer pan needs Playwright.');
	const viewport = context.iframe
		.getByTestId('explainer')
		.contentFrame()
		.locator('.ex-inspect-body:visible');
	const bounds = await viewport.boundingBox();
	if (!bounds) throw new Error('Inspection viewport is not visible.');
	const startX = bounds.x + bounds.width * 0.7;
	const startY = bounds.y + bounds.height * 0.6;
	await context.page.keyboard.down('Meta');
	try {
		await context.page.mouse.move(startX, startY);
		await context.page.mouse.down();
		await context.page.mouse.move(startX - 120, startY - 80, { steps: 5 });
		await context.page.mouse.up();
	} finally {
		await context.page.keyboard.up('Meta');
	}
};

declare module 'vitest/browser' {
	interface BrowserCommands {
		panInspection: () => Promise<void>;
		clickInspectionBackdrop: () => Promise<void>;
		setReducedMotion: (preference: 'reduce' | 'no-preference') => Promise<void>;
		dragReviewSelection: (coordinates: {
			readonly startX: number;
			readonly startY: number;
			readonly endX: number;
			readonly endY: number;
		}) => Promise<void>;
	}
}

/** Real text selection uses pointer input in the existing fixture, never a Selection setter. */
export const dragReviewSelection: BrowserCommand<
	[
		{
			readonly startX: number;
			readonly startY: number;
			readonly endX: number;
			readonly endY: number;
		},
	],
	void
> = async (context, coordinates): Promise<void> => {
	if (context.provider.name !== 'playwright') throw new Error('Text selection needs Playwright.');
	const bounds = await context.iframe.getByTestId('explainer').boundingBox();
	if (!bounds) throw new Error('Review fixture is not visible.');
	await context.page.mouse.move(bounds.x + coordinates.startX, bounds.y + coordinates.startY);
	await context.page.mouse.down();
	await context.page.mouse.move(bounds.x + coordinates.endX, bounds.y + coordinates.endY, {
		steps: 12,
	});
	await context.page.mouse.up();
};

/** Set the real browser preference so motion checks exercise the CSS media query. */
export const setReducedMotion: BrowserCommand<['reduce' | 'no-preference'], void> = async (
	context,
	preference,
): Promise<void> => {
	if (context.provider.name !== 'playwright')
		throw new Error('Motion preference needs Playwright.');
	await context.page.emulateMedia({ reducedMotion: preference });
};

/** A real click outside the native dialog, rather than dispatching a synthetic close event. */
export const clickInspectionBackdrop: BrowserCommand = async (context): Promise<void> => {
	if (context.provider.name !== 'playwright') throw new Error('Backdrop input needs Playwright.');
	const frame = context.iframe.getByTestId('explainer');
	const bounds = await frame.boundingBox();
	if (!bounds) throw new Error('Fixture frame is not visible.');
	await context.page.mouse.click(bounds.x + 2, bounds.y + 2);
};
