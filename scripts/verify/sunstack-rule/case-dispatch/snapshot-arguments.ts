/** Remove only the injected selection input; keep original checked scope and config. */
import assert from 'node:assert/strict';
import path from 'node:path';

export function ordinarySnapshotArguments(
	original: readonly string[],
	ordinaryReport: string,
): string[] {
	assert.ok(path.isAbsolute(ordinaryReport), 'Ordinary report must be external and absolute');
	assert.equal(original[0], 'lint', 'Unexpected executable protocol');
	const boundary = original.indexOf('--');
	assert.ok(
		boundary > 0 && original.indexOf('--', boundary + 1) < 0,
		'Expected exactly one path boundary',
	);
	const prefix = original.slice(0, boundary);
	const positions = (name: string): number[] =>
		prefix.flatMap((value, index) => (value === name ? [index] : []));
	const selection = positions('--case-selection');
	const prepare = positions('--prepare');
	assert.equal(selection.length, 1, 'Expected one injected selection argument');
	assert.equal(prepare.length, 1, 'Expected one prepare output');
	const selectionIndex = selection[0];
	const prepareIndex = prepare[0];
	assert.ok(selectionIndex !== undefined && prepareIndex !== undefined);
	assert.ok(
		prefix[selectionIndex + 1] && !prefix[selectionIndex + 1]?.startsWith('--'),
		'Missing selection path',
	);
	assert.ok(
		prefix[prepareIndex + 1] && !prefix[prepareIndex + 1]?.startsWith('--'),
		'Missing prepare path',
	);
	assert.ok(
		selectionIndex + 1 !== prepareIndex && prepareIndex + 1 !== selectionIndex,
		'Overlapping option values',
	);
	return original.flatMap((value, index) => {
		if (index === selectionIndex || index === selectionIndex + 1) return [];
		if (index === prepareIndex + 1) return [ordinaryReport];
		return [value];
	});
}
