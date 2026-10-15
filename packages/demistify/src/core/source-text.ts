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

/** Reads island source from a script or template child, or plain text when the author wrote no wrapper. */
export function readIslandSource(owner: HTMLElement): string | undefined {
	const wrapper = owner.querySelector(':scope > script, :scope > template');
	if (wrapper instanceof HTMLTemplateElement) return wrapper.content.textContent ?? '';
	if (wrapper) return wrapper.textContent ?? '';
	return owner.children.length === 0 ? (owner.textContent ?? '') : undefined;
}
