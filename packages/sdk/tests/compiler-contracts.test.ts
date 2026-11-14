// The actual TypeScript compiler accepts valid callers and rejects each wrong shape
// at the intended line with the intended diagnostic.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
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

/** One compiler process, kept whole so a crash cannot pass as a clean compile. */
type CompilerRun =
	| { readonly kind: 'compiled' }
	| { readonly kind: 'diagnostics'; readonly diagnostics: CompilerDiagnostic[] }
	| { readonly kind: 'process_failure'; readonly detail: string };

/** Compile one case file with the package's compiler options. */
function compile(caseFile: string, compilerPath: string = compiler): CompilerRun {
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
		const result = spawnSync(compilerPath, ['-p', project, '--pretty', 'false'], {
			encoding: 'utf8',
		});
		const detail = `status ${String(result.status)}, signal ${String(result.signal)}, error ${String(result.error)}, stderr ${result.stderr}, stdout ${result.stdout}`;
		if (result.error !== undefined || result.signal !== null || result.status === null) {
			return { kind: 'process_failure', detail };
		}
		// Every error counts; one without a location (a setup failure) has line 0.
		const diagnostics = [...result.stdout.matchAll(/^(?:.*\((\d+),\d+\): )?error (TS\d+):/gmu)].map(
			(match) => ({
				line: match[1] === undefined ? 0 : Number(match[1]),
				code: match[2] ?? '',
			}),
		);
		if (result.status === 0 && diagnostics.length === 0 && result.stderr === '')
			return { kind: 'compiled' };
		// A failing exit must be explained by compiler diagnostics, not by anything else.
		if (result.status !== 0 && diagnostics.length > 0 && result.stderr === '') {
			return { kind: 'diagnostics', diagnostics };
		}
		return { kind: 'process_failure', detail };
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

await test('a complete valid caller compiles: exit 0, no diagnostics, nothing on stderr', () => {
	assert.deepEqual(compile('valid-caller.ts'), { kind: 'compiled' });
});

await test('configured Markdown islands and empty defaults compile from Rust-derived settings', () => {
	assert.deepEqual(compile('valid-markdown-islands.ts'), { kind: 'compiled' });
});

await test('island selectors reject missing tags, unknown members, null and non-string values', () => {
	const expected = expectedDiagnostics('invalid-markdown-islands.ts');
	assert.equal(expected.length, 6);
	const result = compile('invalid-markdown-islands.ts');
	assert.equal(result.kind, 'diagnostics', result.kind === 'process_failure' ? result.detail : '');
	if (result.kind === 'diagnostics')
		assert.deepEqual(sortDiagnostics(result.diagnostics), sortDiagnostics(expected));
});

await test('unscoped and section-scoped inside fields compile with their derived types', () => {
	assert.deepEqual(compile('valid-scoped-inside.ts'), { kind: 'compiled' });
});

await test('document fields accept scalar paths, candidate lists and both result variants', () => {
	assert.deepEqual(compile('valid-document-fields.ts'), { kind: 'compiled' });
});

await test('document variants reject incorrect paths and missing payloads', () => {
	const expected = expectedDiagnostics('invalid-document-fields.ts');
	assert.equal(expected.length, 6);
	const result = compile('invalid-document-fields.ts');
	assert.equal(result.kind, 'diagnostics', result.kind === 'process_failure' ? result.detail : '');
	if (result.kind === 'diagnostics')
		assert.deepEqual(sortDiagnostics(result.diagnostics), sortDiagnostics(expected));
});

await test('scoped inside fields reject cross-branch combinations and invalid scopes', () => {
	const expected = expectedDiagnostics('invalid-scoped-inside.ts');
	assert.equal(expected.length, 9);
	const result = compile('invalid-scoped-inside.ts');
	assert.equal(result.kind, 'diagnostics', result.kind === 'process_failure' ? result.detail : '');
	if (result.kind === 'diagnostics')
		assert.deepEqual(sortDiagnostics(result.diagnostics), sortDiagnostics(expected));
});

await test('wrong configuration and result shapes fail exactly where marked', () => {
	const expected = expectedDiagnostics('invalid-callers.ts');
	assert.ok(expected.length >= 8, 'every negative case is marked');
	const run = compile('invalid-callers.ts');
	assert.equal(run.kind, 'diagnostics', run.kind === 'process_failure' ? run.detail : '');
	if (run.kind === 'diagnostics') {
		assert.deepEqual(sortDiagnostics(run.diagnostics), sortDiagnostics(expected));
	}
});

await test('a compiler that cannot run is a process failure, never a clean compile', () => {
	const missing = compile(
		'valid-caller.ts',
		join(packageDirectory, 'node_modules', '.bin', 'absent-tsc'),
	);
	assert.equal(missing.kind, 'process_failure');
	const notACompiler = compile('valid-caller.ts', process.execPath);
	assert.equal(
		notACompiler.kind,
		'process_failure',
		'node rejecting the arguments is not a compile',
	);
});

await test('generated declarations use literal unions, never TypeScript enums or any', () => {
	for (const file of ['contracts.ts', 'ast-grep.ts']) {
		const source = readFileSync(join(packageDirectory, 'src', 'contracts', file), 'utf8');
		assert.doesNotMatch(source, /\benum\s+\w+/u, `${file} declares an enum`);
		assert.doesNotMatch(source, /:\s*any\b|<any>|\bany\[\]/u, `${file} uses any`);
	}
});
