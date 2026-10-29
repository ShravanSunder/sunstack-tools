// Ordinary stream comparison shared by live grading and retained-raw regrading.
import assert from 'node:assert/strict';

function linesFrom(bytes: Buffer): string[] {
	const text = bytes.toString();
	return text === '' ? [] : (text.endsWith('\n') ? text.slice(0, -1) : text).split('\n');
}

function managerLine(route: string, line: string): boolean {
	if (['exec-script', 'npx', 'exec-npx'].includes(route))
		return /^npm (?:warn|error|notice|info|verbose|silly|http|timing)\b/.test(line);
	if (route === 'pnpm-dlx')
		return (
			/^Done in [0-9.]+(?:ms|s|m) using pnpm v12\.14\.0$/.test(line) ||
			/^\u2009(?:WARN|ELIFECYCLE|ERR_PNPM_[A-Z_]+)\u2009 /.test(line)
		);
	return false;
}

function compareStreams(
	native: { stdout: Buffer; stderr: Buffer },
	channel: { stdout: Buffer; stderr: Buffer },
	route: string,
): { native_line_positions: number[]; manager_extra_lines: string[] } {
	assert.deepEqual(channel.stdout, native.stdout, 'stdout bytes differ from native');
	const nativeLines = linesFrom(native.stderr),
		channelLines = linesFrom(channel.stderr);
	const positions: number[] = [],
		extras: string[] = [];
	for (const [index, line] of channelLines.entries()) {
		if (positions.length < nativeLines.length && line === nativeLines[positions.length])
			positions.push(index);
		else {
			assert.ok(line === '' || managerLine(route, line), 'unattributed extra stderr line: ' + line);
			extras.push(line);
		}
	}
	assert.equal(positions.length, nativeLines.length, 'native stderr line missing or reordered');
	return { native_line_positions: positions, manager_extra_lines: extras };
}

export { compareStreams };
