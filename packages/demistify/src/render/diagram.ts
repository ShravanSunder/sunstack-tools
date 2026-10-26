import { nameChartSeries, prepareChartSource } from './chart-source.ts';
import { respaceGraphEdges } from './graph-source.ts';
import { removeSvgFontImports } from './svg-fonts.ts';

type MermaidRenderer = (source: string, options: Record<string, unknown>) => string;

/** Flowchart categories use the shared palette without coupling color to a page link. */
function supplyCategoryStyles(sourceText: string): string {
	const definitions = [1, 2, 3, 4, 5, 6]
		.map(
			(slot): string =>
				`classDef slot${slot} fill:var(--chart-${slot}-soft),stroke:var(--chart-${slot}),color:var(--ink)`,
		)
		.join('\n');
	// Insert before authored statements so explicit class definitions retain their precedence.
	return sourceText.replace(
		/^[\t ]*(?:flowchart|graph)[\t ]+(?:TD|TB|LR|BT|RL)[\t ]*$/im,
		(header): string => `${header}\n${definitions}`,
	);
}

/** Repeated identities (such as sequence participants) keep one palette slot in this drawing. */
function colorDiagramIdentities(svg: string): string {
	const slots = new Map<string, number>();
	return svg.replace(
		/<g class="(?:node|actor|class-node|entity|subgraph)" data-id="([^"]+)"[^>]*>/g,
		(group, identity: string): string => {
			if (/data-shape="state-(?:start|end)"/.test(group)) return group;
			let slot = slots.get(identity);
			if (slot === undefined) {
				slot = (slots.size % 6) + 1;
				slots.set(identity, slot);
			}
			return group.replace(/>$/, ` data-diagram-slot="${slot}">`);
		},
	);
}

/** Renders diagram text with the page's colour tokens and local fonts. */
export function renderThemedDiagram(render: MermaidRenderer, sourceText: string): string {
	const chart = prepareChartSource(supplyCategoryStyles(respaceGraphEdges(sourceText)));
	return nameChartSeries(
		colorDiagramIdentities(
			removeSvgFontImports(
				render(chart.source, {
					bg: 'var(--surface-content)',
					fg: 'var(--ink)',
					muted: 'var(--ink-soft)',
					line: 'var(--ink-soft)',
					// Inherit the page accent; assigning --accent to itself makes it invalid.
					surface: 'var(--keep-bg)',
					border: 'var(--rule)',
					font: 'Source Sans 3',
					transparent: true,
				}),
			),
		),
		chart.source,
		chart.seriesNames,
	);
}
