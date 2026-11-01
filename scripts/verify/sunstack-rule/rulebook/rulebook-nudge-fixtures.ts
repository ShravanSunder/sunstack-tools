/** R1-R10 fixture preparation and source-derived values; never reads a product result. */
import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import path from 'node:path';

import {
	configure,
	equal,
	fixtureSource,
	objectValue,
	readObject,
	requireCondition,
	rulebookName,
	rulesName,
	textValue,
	writeJson,
	type JsonObject,
} from './rulebook-fixture-support.ts';

export const nudgeOracle = 'docs/specs/2026-10-31-rules-as-nudges-engine/verification-design.md';
export const locatedFile = 'skills/ops-observability-stack/SKILL.md';
export const nudgeSource = path.join(fixtureSource, 'nudge');

export function sourceText(root: string, file: string): string {
	return filesystem.readFileSync(path.join(root, file), 'utf8');
}
export function sourceSpan(root: string, file: string, start: number, end: number): JsonObject {
	const text = sourceText(root, file);
	const position = (offset: number): [number, number] => {
		const prefix = Buffer.from(text).subarray(0, offset).toString('utf8');
		const lines = prefix.split('\n');
		return [lines.length, Array.from(lines.at(-1) ?? '').length + 1];
	};
	const [startLine, startColumn] = position(start);
	const [endLine, endColumn] = position(end);
	return {
		source: {
			kind: 'document',
			repository: 'checked_repository',
			path: file,
			content_identity: createHash('sha256').update(text).digest('hex'),
		},
		byte_start: start,
		byte_end: end,
		start_line: startLine,
		start_column: startColumn,
		end_line: endLine,
		end_column: endColumn,
	};
}
export function pieceWhere(root: string, file: string, start = 0): JsonObject {
	const bytes = Buffer.from(sourceText(root, file));
	const span = sourceSpan(root, file, start, bytes.length);
	return {
		file,
		first_line: span['start_line'] ?? null,
		last_line: sourceText(root, file).trimEnd().split('\n').length,
		span,
	};
}
export function selectedFixture(destination: string, markers: readonly string[]): string {
	requireCondition(!filesystem.existsSync(destination), 'Nudge fixture destination exists');
	filesystem.cpSync(nudgeSource, destination, { recursive: true });
	const entries = readObject(path.join(destination, rulesName));
	writeJson(
		path.join(destination, rulesName),
		Object.fromEntries(
			markers.map((marker) => {
				const entry = entries[marker];
				requireCondition(entry, `Unknown nudge marker ${marker}`);
				return [marker, entry];
			}),
		),
	);
	// Keep original line numbers: deleting a peer must not move the standard's marker.
	filesystem.writeFileSync(
		path.join(destination, rulebookName),
		sourceText(destination, rulebookName)
			.split('\n')
			.map((line) => (markers.some((marker) => line.includes(`<!-- ${marker} -->`)) ? line : ''))
			.join('\n'),
	);
	return destination;
}
export function standardText(root: string, marker: string): string {
	const line = sourceText(root, rulebookName)
		.split('\n')
		.find((value) => value.includes(`<!-- ${marker} -->`));
	requireCondition(line, `No standard for ${marker}`);
	return line.replace(/\s*<!-- K\d+ -->/, '').trim();
}
export function standardPlace(root: string, marker: string): JsonObject {
	const line =
		sourceText(root, rulebookName)
			.split('\n')
			.findIndex((value) => value.includes(`<!-- ${marker} -->`)) + 1;
	requireCondition(line > 0, 'Standard marker lost');
	return {
		label: 'standard',
		file: filesystem.realpathSync(path.join(root, rulebookName)),
		first_line: line,
		last_line: line,
	};
}
export function locationBody(root: string): string {
	// Frozen source: line 6 is the H1; own text ends before the first H2 at line 25.
	const lines = sourceText(root, locatedFile).split('\n');
	equal(lines[5], '# Ops Observability Stack', 'Frozen H1 moved');
	equal(
		lines[16],
		'- IF the work is AgentStudio debug or beta, load `references/agentstudio-loop.md` and follow that loop.',
		'Frozen line 17 moved',
	);
	return lines.slice(6, 24).join('\n') + '\n';
}
export function locationQuestions(root: string, offer = true): JsonObject {
	const questions = structuredClone(
		objectValue(objectValue(readObject(path.join(root, rulesName))['K001'])['questions']),
	);
	if (offer) {
		const lines = sourceText(root, locatedFile).split('\n');
		questions['anchor_line'] = {
			type: 'choice',
			instructions: { check: 'body', task: 'Choose the line about this finding.' },
			criteria: Object.fromEntries(
				lines.slice(6, 24).map((line, index) => [`L${index + 7}`, line]),
			),
		};
	}
	return questions;
}
export function nudgeState(root: string, marker: string, file: string): JsonObject {
	const text = sourceText(root, file);
	const state: JsonObject = {
		standard: standardText(root, marker),
		where: pieceWhere(root, file),
		body: text,
	};
	if (marker === 'K001') {
		state['where'] = pieceWhere(
			root,
			file,
			Buffer.byteLength(text.split('\n').slice(0, 5).join('\n') + '\n'),
		);
		state['heading'] = '# Ops Observability Stack';
		state['body'] = locationBody(root);
		state['targets'] = [...text.matchAll(/load `([^`]+)`/g)].map((match) => {
			const name = textValue(match[1]);
			const target = path.posix.join(path.posix.dirname(file), name);
			const exists = filesystem.existsSync(path.join(root, target));
			const targetText = exists ? sourceText(root, target) : undefined;
			const value: JsonObject = { name, path: target, exists };
			if (targetText !== undefined) {
				value['opening'] = Array.from(targetText).slice(0, 120).join('');
				if (Array.from(targetText).length > 120) value['opening_cut'] = true;
			}
			return value;
		});
	}
	if (marker === 'K006') {
		const newline = text.indexOf('\n');
		const authored = text.slice(0, newline);
		state['cut'] = authored;
		const body = text.slice(newline + 1);
		state['body'] = body;
		if (body.length + authored.length > 1000) {
			const desired = 1000 - authored.length;
			const prefix = Math.ceil((desired - 5) / 2);
			const suffix = desired - 5 - prefix;
			state['body'] = body.slice(0, prefix) + '[cut]' + body.slice(-suffix);
			state['cut-fields'] = ['body'];
		}
	}
	return state;
}
export function onlyFiles(root: string, files: readonly string[]): void {
	configure(root, (config) => {
		config['check'] = { files: [...files] };
	});
}
export function setLocateLimit(root: string, limit: number): void {
	configure(root, (config) => {
		config['limits'] = { ...objectValue(config['limits']), max_locate_lines: limit };
	});
}
