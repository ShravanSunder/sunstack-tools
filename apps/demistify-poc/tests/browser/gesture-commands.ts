import type { BrowserCommand } from 'vitest/node';

/** Vitest's native dragAndDrop targets HTML drag/drop; this gesture needs real pointer capture. */
export const panInspection: BrowserCommand = async (context): Promise<void> => {
	if (context.provider.name !== 'playwright') throw new Error('Pointer pan needs Playwright.');
	const viewport = context.iframe
		.getByTestId('explainer')
		.contentFrame()
		.locator('.ex-drawer .ex-inspect-body');
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
	}
}
