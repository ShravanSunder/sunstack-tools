import { playwright } from '@vitest/browser-playwright';
import { defineConfig } from 'vitest/config';

import { browserPort } from './tests/browser/browser-port.ts';
import { builtFixtures } from './tests/browser/built-fixtures.ts';
import {
	clickInspectionBackdrop,
	panInspection,
	setReducedMotion,
} from './tests/browser/gesture-commands.ts';

export default defineConfig({
	plugins: [await builtFixtures()],
	test: {
		api: { host: '127.0.0.1', port: await browserPort() },
		include: ['tests/browser/**/*.browser.test.ts'],
		testTimeout: 20_000,
		expect: { poll: { timeout: 5_000 } },
		browser: {
			commands: { clickInspectionBackdrop, panInspection, setReducedMotion },
			enabled: true,
			headless: true,
			provider: playwright({ actionTimeout: 5_000 }),
			instances: [{ browser: 'chromium' }],
			viewport: { width: 1600, height: 1000 },
			screenshotFailures: false,
		},
	},
});
