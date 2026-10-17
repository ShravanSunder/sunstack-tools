export function applySvgTheme(
	svgSource: string,
	themeColors: Readonly<Record<string, string>>,
): string {
	const themeEntries = [...Object.entries(themeColors), ['surface-content', '#FFFFFF']];
	return themeEntries.reduce((source, [tokenName, colorValue]): string => {
		if (!tokenName || !colorValue || !/^#[0-9a-f]{6}$/i.test(colorValue)) return source;
		return source.replace(new RegExp(colorValue, 'gi'), `var(--${tokenName}, ${colorValue})`);
	}, svgSource);
}
export function removeSvgFontImports(svgSource: string): string {
	return svgSource.replace(/@import\s+url\('[^']*'\);/g, '');
}
