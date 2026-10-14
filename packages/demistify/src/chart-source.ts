/**
 * Adapts xychart text for the diagram engine: category labels may be quoted, and each series may
 * be named (`bar "Before" [1, 2]`). The engine accepts neither, so names are removed before
 * rendering and written into the legend afterwards.
 */
export interface PreparedChart {
	readonly source: string;
	readonly seriesNames: readonly (string | undefined)[];
}

export function prepareChartSource(sourceText: string): PreparedChart {
	if (!/^\s*xychart/i.test(sourceText)) return { source: sourceText, seriesNames: [] };
	const seriesNames: (string | undefined)[] = [];
	const source = sourceText
		.split('\n')
		.map((line): string => {
			const categories = /^(\s*x-axis\s+(?:"[^"]*"\s*)?)\[([^\]]*)\]/.exec(line);
			if (categories)
				return `${categories[1] ?? ''}[${(categories[2] ?? '')
					.split(',')
					.map((label): string => label.trim().replace(/^"(.*)"$/, '$1'))
					.join(', ')}]`;
			const series = /^(\s*)(bar|line)\s+(?:"([^"]*)"\s*)?(\[.*)$/.exec(line);
			if (!series) return line;
			seriesNames.push(series[3]);
			return `${series[1] ?? ''}${series[2] ?? ''} ${series[4] ?? ''}`;
		})
		.join('\n');
	return { source, seriesNames };
}

export function nameChartSeries(
	svg: string,
	sourceText: string,
	seriesNames: readonly (string | undefined)[],
): string {
	if (seriesNames.every((name): boolean => name === undefined)) return svg;
	let barIndex = 0;
	let lineIndex = 0;
	const kinds = Array.from(
		sourceText.matchAll(/^\s*(bar|line)\s+\[/gm),
		(match): string => match[1] ?? '',
	);
	let named = svg;
	kinds.forEach((kind, index): void => {
		const generic = kind === 'bar' ? `Bar ${(barIndex += 1)}` : `Line ${(lineIndex += 1)}`;
		const name = seriesNames[index];
		if (name) named = named.replaceAll(`>${generic}<`, `>${name.replaceAll('<', '&lt;')}<`);
	});
	return named;
}
