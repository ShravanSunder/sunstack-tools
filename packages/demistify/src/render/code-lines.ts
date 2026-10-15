/**
 * Splits code or a unified diff into files and numbered rows. Pure, so the runtime and the checker
 * number lines the same way.
 */
export type LineKind = 'added' | 'removed' | 'context' | 'hunk';

export function lineKind(line: string, diff: boolean): LineKind {
	if (!diff) return 'context';
	if (line.startsWith('@@')) return 'hunk';
	if (line.startsWith('+')) return 'added';
	if (line.startsWith('-')) return 'removed';
	return 'context';
}

export interface CodeLine {
	readonly text: string;
	readonly kind: LineKind;
	/** The number shown and the number notes attach to; undefined for hunk headers. */
	readonly number: number | undefined;
}
export interface CodeFile {
	readonly path: string | undefined;
	readonly lines: CodeLine[];
}

const fileHeader = /^diff --git a\/(\S+) b\/(\S+)/;
const fileMetadata =
	/^(index |new file mode|deleted file mode|similarity index|rename (from|to)|old mode|new mode|--- |\+\+\+ )/;
const hunkHeader = /^@@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? @@/;

/**
 * Every row gets one number, removed lines included, so the number a reader sees is the number a
 * note names. A `@@ -a +b @@` hunk header restarts the count at b; otherwise rows count from `start`.
 */
export function parseCode(
	lines: readonly string[],
	diff: boolean,
	firstNumber: number,
): CodeFile[] {
	const files: CodeFile[] = [];
	let current: CodeFile = { path: undefined, lines: [] };
	let rowNumber = firstNumber;
	for (const line of lines) {
		const header = diff ? fileHeader.exec(line) : null;
		const plusPath =
			diff && line.startsWith('+++ ') ? line.slice(4).replace(/^b\//, '') : undefined;
		if (header) {
			if (current.lines.length > 0 || current.path) files.push(current);
			current = { path: header[2], lines: [] };
			continue;
		}
		if (plusPath !== undefined && current.lines.length === 0) {
			if (plusPath !== '/dev/null') current = { path: current.path ?? plusPath, lines: [] };
			continue;
		}
		if (diff && current.lines.length === 0 && fileMetadata.test(line)) {
			if (line.startsWith('--- ') && !current.path)
				current = { path: line.slice(4).replace(/^a\//, ''), lines: [] };
			continue;
		}
		const kind = lineKind(line, diff);
		if (kind === 'hunk') {
			const numbers = hunkHeader.exec(line);
			if (numbers) rowNumber = Number(numbers[1]);
			current.lines.push({ text: line, kind, number: undefined });
			continue;
		}
		current.lines.push({ text: line, kind, number: rowNumber });
		rowNumber += 1;
	}
	files.push(current);
	return files;
}
