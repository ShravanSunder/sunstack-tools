/** Exercise the actual verification HTTP hook and event journal with a harmless owned child. */
import filesystem from 'node:fs';
import operatingSystem from 'node:os';
import path from 'node:path';

import { afterEach, expect, test } from 'vitest';

import { baselineGit, gitCommand, parseObject, readObject } from './rulebook-fixture-support.ts';
import { declaredTarget, RulebookProcess } from './rulebook-process.ts';
import { keepKinds } from './selection-fixture.ts';
import { expectedSelectionState } from './selection-journey.ts';
const roots: string[] = [];
afterEach(() => {
	for (const root of roots.splice(0)) filesystem.rmSync(root, { recursive: true, force: true });
});
test('intentional provider error retains capture and response events without becoming a harness error', async () => {
	const root = filesystem.mkdtempSync(path.join(operatingSystem.tmpdir(), 'selection-process-'));
	roots.push(root);
	const source = path.join(root, 'source');
	filesystem.mkdirSync(source);
	const binary = path.join(source, 'harmless-cli');
	filesystem.writeFileSync(
		binary,
		`#!${process.execPath}\nconst response=await fetch(process.argv[2],{method:'POST',body:JSON.stringify({questions:{['c1']:{type:'noul',instructions:'Observe'}},state:{where:{file:'one.md'},candidates:{['c1']:{body:'One'}},shared:{}}})});process.stdout.write(JSON.stringify({status:response.status,body:await response.json()}));`,
	);
	filesystem.chmodSync(binary, 0o755);
	baselineGit(source);
	const target = declaredTarget({
		binary,
		source,
		commit: gitCommand(source, ['rev-parse', 'HEAD']),
	});
	const observer = new RulebookProcess(target, path.join(root, 'evidence'));
	await observer.start();
	observer.respond = async (request) => {
		expect(request.body['state']).toHaveProperty('candidates');
		return { status: 500, body: { error: 'controlled failure' } };
	};
	try {
		const result = await observer.run({
			root: source,
			name: 'controlled-provider-error',
			arguments: [observer.endpoint],
			expectedExit: 0,
		});
		expect(parseObject(result.stdout.toString('utf8'))).toEqual({
			status: 500,
			body: { error: 'controlled failure' },
		});
		expect(result.captures).toHaveLength(1);
		expect(observer.events).toEqual([
			{ request: 1, event: 'received' },
			{ request: 1, event: 'responded' },
		]);
	} finally {
		await observer.close();
	}
	expect(readObject(path.join(root, 'evidence/harness.json'))['errors']).toEqual([]);
});

test('source oracle keeps a parent section remainder and removes only strict child ranges', () => {
	const root = filesystem.mkdtempSync(path.join(operatingSystem.tmpdir(), 'selection-oracle-'));
	roots.push(root);
	const text = '## Workflow\n\n1. Parent item.\n\n### Child\n\n1. Child item.\n';
	filesystem.writeFileSync(path.join(root, 'one.md'), text);
	const child = text.indexOf('### Child');
	filesystem.writeFileSync(
		path.join(root, 'scan-oracle.json'),
		JSON.stringify({
			matches: [
				{ rule: 'section', file: 'one.md', start: 0, end: text.length, text },
				{
					rule: 'section',
					file: 'one.md',
					start: child,
					end: text.length,
					text: text.slice(child),
				},
				{ rule: 'workflow-candidate', file: 'one.md', start: 0, end: text.length, text },
			],
		}),
	);
	expect(expectedSelectionState(root, 'one.md')).toEqual({
		candidates: { ['c1']: { heading: '## Workflow', body: '\n1. Parent item.\n\n' } },
		shared: { outline: ['## Workflow', '### Child'] },
	});
});

test('pruning a kind retains a valid canonical body-rule witness', () => {
	const root = filesystem.mkdtempSync(path.join(operatingSystem.tmpdir(), 'selection-prune-'));
	roots.push(root);
	filesystem.writeFileSync(
		path.join(root, 'skill-rules.config.json'),
		JSON.stringify({
			evidence: {
				kinds: {
					step: { rule: 'step', fields: { body: { from: 'remainder' } } },
					section: { rule: 'section', fields: { body: { from: 'remainder' } } },
				},
			},
		}),
	);
	filesystem.writeFileSync(
		path.join(root, 'rules.json'),
		JSON.stringify({ K001: { name: 'step_body', evidence: 'step' } }),
	);
	keepKinds(root, ['section']);
	expect(readObject(path.join(root, 'rules.json'))).toEqual({
		K001: { name: 'step_body', evidence: 'section' },
	});
});
