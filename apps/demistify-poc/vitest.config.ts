import { defineConfig, mergeConfig } from 'vitest/config';

import viteConfig from './vite.config.ts';

export default mergeConfig(
	viteConfig,
	defineConfig({
		test: {
			globals: false,
			environment: 'node',
			include: [
				'src/**/*.test.ts',
				'tests/integration/**/*.test.ts',
				'../../packages/demistify/src/**/*.test.ts',
			],
		},
	}),
);
