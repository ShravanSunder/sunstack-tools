import { expect, it } from 'vitest';

import { normalizeIslandText } from './source-text.ts';

it('preserves Markdown and Mermaid structure after HTML formatter indentation', (): void => {
	expect(normalizeIslandText('\n\t\t# Design\n\n\t\tA paragraph.\n\t')).toBe(
		'# Design\n\nA paragraph.',
	);
	expect(normalizeIslandText('\n\tflowchart TD\n\t  A --> B\n')).toBe('flowchart TD\n  A --> B');
});
