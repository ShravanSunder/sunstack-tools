import { nameChartSeries, prepareChartSource } from './chart-source.ts';
import { removeSvgFontImports } from './svg-fonts.ts';

type MermaidRenderer = (source: string, options: Record<string, unknown>) => string;

/** Renders diagram text with the page's colour tokens and local fonts. */
export function renderThemedDiagram(render: MermaidRenderer, sourceText: string): string {
	const chart = prepareChartSource(sourceText);
	return nameChartSeries(
		removeSvgFontImports(
			render(chart.source, {
				bg: 'var(--sheet)',
				fg: 'var(--ink)',
				accent: 'var(--accent)',
				surface: 'var(--accent-soft)',
				border: 'var(--rule)',
				font: 'IBM Plex Sans Condensed',
				transparent: true,
			}),
		),
		chart.source,
		chart.seriesNames,
	);
}
