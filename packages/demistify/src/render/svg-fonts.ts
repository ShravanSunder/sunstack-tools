export function removeSvgFontImports(svgSource: string): string {
	return svgSource.replace(/@import\s+url\('[^']*'\);/g, '');
}
