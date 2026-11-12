// The actual TypeScript compiler accepts valid callers and rejects each wrong shape
// at the intended line with the intended diagnostic.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

import { ownedDirectory } from './support/sdk-fixtures.ts';

const packageDirectory = fileURLToPath(new URL('../', import.meta.url));
const casesDirectory = join(packageDirectory, 'tests', 'compiler-cases');
const compiler = join(packageDirectory, 'node_modules', '.bin', 'tsc');

interface CompilerDiagnostic {
	readonly line: number;
	readonly code: string;
}

/** Compile one case file with the package's compiler options; return its diagnostics. */
function compile(caseFile: string): CompilerDiagnostic[] {
	const scratch = ownedDirectory('compiler');
	try {
		const project = join(scratch.path, 'tsconfig.json');
		writeFileSync(
			project,
			JSON.stringify({
				extends: join(packageDirectory, 'tsconfig.json'),
				compilerOptions: {
					noEmit: true,
					typeRoots: [join(packageDirectory, 'node_modules', '@types')],
				},
				include: [join(casesDirectory, caseFile)],
				exclude: [],
			}),
		);
		let output = '';
		try {
			execFileSync(compiler, ['-p', project, '--pretty', 'false'], {
				encoding: 'utf8',
				stdio: 'pipe',
			});
		} catch (error) {
			output =
				typeof error === 'object' && error !== null && 'stdout' in error
					? String(error.stdout)
					: String(error);
		}
		// Every error counts; one without a location (a setup failure) has line 0.
		return [...output.matchAll(/^(?:.*\((\d+),\d+\): )?error (TS\d+):/gmu)].map((match) => ({
			line: match[1] === undefined ? 0 : Number(match[1]),
			code: match[2] ?? '',
		}));
	} finally {
		scratch.remove();
	}
}

/** Each `// expect TSnnnn` marker names the diagnostic required on the next line. */
function expectedDiagnostics(caseFile: string): CompilerDiagnostic[] {
	return readFileSync(join(casesDirectory, caseFile), 'utf8')
		.split('\n')
		.flatMap((line, index) => {
			const match = /\/\/ expect (TS\d+)/u.exec(line);
			return match?.[1] === undefined ? [] : [{ line: index + 2, code: match[1] }];
		});
}

const sortDiagnostics = (diagnostics: CompilerDiagnostic[]): CompilerDiagnostic[] =>
	diagnostics.toSorted(
		(left, right) => left.line - right.line || left.code.localeCompare(right.code),
	);

await test('a complete valid caller compiles with no diagnostics', () => {
	assert.deepEqual(compile('valid-caller.ts'), []);
});

await test('wrong configuration and result shapes fail exactly where marked', () => {
	const expected = expectedDiagnostics('invalid-callers.ts');
	assert.ok(expected.length >= 8, 'every negative case is marked');
	assert.deepEqual(sortDiagnostics(compile('invalid-callers.ts')), sortDiagnostics(expected));
});

await test('generated declarations use literal unions, never TypeScript enums or any', () => {
	for (const file of ['contracts.ts', 'ast-grep.ts']) {
		const source = readFileSync(join(packageDirectory, 'src', 'contracts', file), 'utf8');
		assert.doesNotMatch(source, /\benum\s+\w+/u, `${file} declares an enum`);
		assert.doesNotMatch(source, /:\s*any\b|<any>|\bany\[\]/u, `${file} uses any`);
	}
});
