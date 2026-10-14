export function normalizeIslandText(sourceText: string): string {
	const sourceLines = sourceText.replaceAll('\r\n', '\n').split('\n');
	const nonemptyLines = sourceLines.filter((line): boolean => line.trim().length > 0);
	const commonIndent = Math.min(
		...nonemptyLines.map((line): number => /^[\t ]*/.exec(line)?.[0].length ?? 0),
	);
	if (!Number.isFinite(commonIndent)) return '';
	return sourceLines
		.map((line): string =>
			line.slice(Math.min(commonIndent, /^[\t ]*/.exec(line)?.[0].length ?? 0)),
		)
		.join('\n')
		.trim();
}
