import { renderMermaidSVG } from 'beautiful-mermaid';
import { expect, test } from 'vitest';

import { renderThemedDiagram } from './diagram.ts';

test('category declarations work with CRLF source', (): void => {
	const svg = renderThemedDiagram(
		renderMermaidSVG,
		'flowchart LR\r\n  a[Reader] --> b[Store]\r\n  class a slot6',
	);

	expect(svg).toContain('fill="var(--chart-6-soft)"');
});

test('flowchart category slots supply soft fills and borders without authored class definitions', (): void => {
	const source = `flowchart LR
  a[Client] --> b[Worker]
  b --> c[Registry]
  c --> d[Project]
  d --> e[Identity]
  e --> f[Archive]
  class a slot1
  class b slot2
  class c slot3
  class d slot4
  class e slot5
  class f slot6`;

	const svg = renderThemedDiagram(renderMermaidSVG, source);

	for (const slot of [1, 2, 3, 4, 5, 6]) {
		expect(svg).toContain(`fill="var(--chart-${slot}-soft)"`);
		expect(svg).toContain(`stroke="var(--chart-${slot})"`);
	}
	expect(svg).toContain('fill="var(--ink)"');
});

test('unclassified nodes receive distinct identity colors over a neutral base', (): void => {
	const svg = renderThemedDiagram(renderMermaidSVG, 'flowchart LR\n  a[Client] --> b[Service]');

	expect(svg).toContain('--surface:var(--keep-bg)');
	expect(svg).toContain('data-diagram-slot="1"');
	expect(svg).toContain('data-diagram-slot="2"');
});

test('authored classes retain precedence and sequence syntax remains valid', (): void => {
	const authored = renderThemedDiagram(
		renderMermaidSVG,
		'graph TD\n  a[Client]\n  classDef slot1 fill:var(--keep-bg)\n  class a slot1',
	);
	const sequence = renderThemedDiagram(
		renderMermaidSVG,
		'sequenceDiagram\n  Client->>Service: Request',
	);

	expect(authored).toContain('fill="var(--keep-bg)"');
	expect(authored).not.toContain('fill="var(--chart-1-soft)"');
	expect(sequence).toContain('Request');
	expect(sequence).not.toContain('slot1');
	expect(sequence).toContain('data-diagram-slot="1"');
});

test.each([
	['stateDiagram-v2\n [*] --> Draft\n Draft --> Review', 'Draft'],
	['classDiagram\n Workspace --> Project', 'Workspace'],
	['erDiagram\n WORKSPACE ||--o{ PROJECT : contains', 'WORKSPACE'],
])('default colors also cover %s', (source, identity): void => {
	const svg = renderThemedDiagram(renderMermaidSVG, source);

	expect(svg).toContain(`data-id="${identity}"`);
	expect(svg).toContain('data-diagram-slot="1"');
	expect(svg).toContain('data-diagram-slot="2"');
});
