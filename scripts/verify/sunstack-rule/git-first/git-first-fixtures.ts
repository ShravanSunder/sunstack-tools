/** Owned raw-identity edge states: no original input or product source is mutated. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';

import {
	prepareRepository,
	checkedFile,
	assetFile,
	configFile,
	pngBytes,
} from '../cli-reports/scope-fixtures.ts';
import {
	gitCommand,
	readObject,
	writeJson,
	objectValue,
} from '../rulebook/rulebook-fixture-support.ts';

export const edgeCases = [
	'staged-only',
	'unstaged-only',
	'untracked',
	'deleted',
	'renamed',
	'mode-only',
	'unchanged-binary',
	'changed-binary',
	'binary-to-text',
	'invalid-checked-text',
	'changed-symlink',
	'unchanged-symlink',
	'equal-payload-type',
	'ignored',
	'assume-unchanged',
	'skip-worktree',
	'crlf',
	'clean-filter',
	'stat-cache',
] as const;
export type EdgeCase = (typeof edgeCases)[number];
export interface EdgeFixture {
	readonly name: EdgeCase;
	readonly root: string;
	readonly base: string;
	readonly expectedSelectedFiles: string[];
	readonly expectedCaptureAcquisitions: number | null;
	readonly encodingRefusalPath: string | null;
	readonly gitDiffBlind: boolean;
}
export function prepareEdgeFixture(options: {
	root: string;
	endpoint: string;
	name: EdgeCase;
}): EdgeFixture {
	const { root, endpoint, name } = options;
	assert.ok(!filesystem.existsSync(root), 'Refuse to reuse an existing fixture/input directory');
	prepareRepository({
		root,
		endpoint,
		binaryAsset: ['unchanged-binary', 'changed-binary', 'binary-to-text'].includes(name),
	});
	const config = readObject(path.join(root, configFile));
	objectValue(config['check'])['files'] = ['docs/*.md'];
	// A whole-document piece includes CRLF-only hunks at paragraph boundaries.
	filesystem.unlinkSync(path.join(root, 'evidence/rules/paragraph.yml'));
	filesystem.writeFileSync(
		path.join(root, 'evidence/rules/document.yml'),
		'id: document\nlanguage: Markdown\nrule: {kind: document}\n',
	);
	objectValue(config['evidence'])['kinds'] = {
		document: { rule: 'document', fields: { body: { from: 'remainder' } } },
	};
	const ruleEntries = readObject(path.join(root, 'rules.json'));
	objectValue(ruleEntries['K001'])['evidence'] = 'document';
	writeJson(path.join(root, 'rules.json'), ruleEntries);
	writeJson(path.join(root, configFile), config);
	filesystem.writeFileSync(path.join(root, 'docs/peer.md'), 'BODY unchanged peer.\n');
	filesystem.writeFileSync(path.join(root, '.gitignore'), 'ignored/\n');
	gitCommand(root, ['config', 'core.autocrlf', 'false']);
	if (['changed-symlink', 'unchanged-symlink'].includes(name)) {
		const rules = filesystem.readFileSync(path.join(root, 'rules.json'));
		for (const target of ['rules-a.json', 'rules-b.json'])
			filesystem.writeFileSync(path.join(root, target), rules);
		filesystem.unlinkSync(path.join(root, 'rules.json'));
		filesystem.symlinkSync('rules-a.json', path.join(root, 'rules.json'));
	}
	if (name === 'equal-payload-type') {
		filesystem.writeFileSync(path.join(root, assetFile), 'target.txt');
		filesystem.writeFileSync(path.join(root, 'assets/target.txt'), 'Different followed content.\n');
	}
	if (name === 'crlf')
		filesystem.writeFileSync(path.join(root, '.gitattributes'), '*.md text=auto eol=lf\n');
	if (name === 'clean-filter') {
		filesystem.writeFileSync(path.join(root, '.gitattributes'), '*.md filter=strip-cr\n');
		gitCommand(root, ['config', 'filter.strip-cr.clean', "tr -d '\\r'"]);
		gitCommand(root, ['config', 'filter.strip-cr.smudge', 'cat']);
	}
	if (name === 'stat-cache') {
		gitCommand(root, ['config', 'core.trustctime', 'false']);
		const oldTime = new Date(Date.now() - 600000);
		filesystem.utimesSync(path.join(root, checkedFile), oldTime, oldTime);
	}
	gitCommand(root, ['add', '-A']);
	gitCommand(root, ['commit', '-qm', 'Raw capture edge baseline']);
	const base = gitCommand(root, ['rev-parse', 'HEAD']);
	const checked = path.join(root, checkedFile);
	let expectedSelectedFiles: string[] = [checkedFile];
	let expectedCaptureAcquisitions: number | null = 2;
	let encodingRefusalPath: string | null = null;
	let gitDiffBlind = false;
	switch (name) {
		case 'staged-only':
			filesystem.writeFileSync(checked, 'BODY staged change.\n');
			gitCommand(root, ['add', checkedFile]);
			break;
		case 'unstaged-only':
			filesystem.writeFileSync(checked, 'BODY unstaged change.\n');
			break;
		case 'untracked':
			filesystem.writeFileSync(path.join(root, 'docs/new.md'), 'BODY untracked.\n');
			expectedSelectedFiles = ['docs/new.md'];
			expectedCaptureAcquisitions = 1;
			break;
		case 'deleted':
			filesystem.unlinkSync(checked);
			expectedSelectedFiles = [];
			expectedCaptureAcquisitions = 1;
			break;
		case 'renamed':
			gitCommand(root, ['mv', checkedFile, 'docs/renamed.md']);
			expectedSelectedFiles = ['docs/renamed.md'];
			break;
		case 'mode-only':
			filesystem.chmodSync(checked, 0o755);
			expectedSelectedFiles = [];
			expectedCaptureAcquisitions = 0;
			break;
		case 'unchanged-binary':
			filesystem.writeFileSync(checked, 'BODY beside equal binary.\n');
			break;
		case 'changed-binary':
			filesystem.writeFileSync(
				path.join(root, assetFile),
				Buffer.concat([pngBytes, Buffer.from([0xfe])]),
			);
			encodingRefusalPath = assetFile;
			expectedCaptureAcquisitions = null;
			expectedSelectedFiles = [];
			break;
		case 'binary-to-text':
			filesystem.writeFileSync(path.join(root, assetFile), 'Now plain text.\n');
			encodingRefusalPath = assetFile;
			expectedCaptureAcquisitions = null;
			expectedSelectedFiles = [];
			break;
		case 'invalid-checked-text':
			filesystem.writeFileSync(checked, Buffer.from([0xff]));
			encodingRefusalPath = checkedFile;
			expectedCaptureAcquisitions = null;
			expectedSelectedFiles = [];
			break;
		case 'changed-symlink':
			filesystem.unlinkSync(path.join(root, 'rules.json'));
			filesystem.symlinkSync('rules-b.json', path.join(root, 'rules.json'));
			filesystem.writeFileSync(checked, 'BODY with changed raw link.\n');
			expectedSelectedFiles = [checkedFile, 'docs/peer.md'];
			expectedCaptureAcquisitions = 4;
			break;
		case 'unchanged-symlink':
			filesystem.writeFileSync(checked, 'BODY with unchanged raw link.\n');
			break;
		case 'equal-payload-type':
			filesystem.unlinkSync(path.join(root, assetFile));
			filesystem.symlinkSync('target.txt', path.join(root, assetFile));
			expectedSelectedFiles = [];
			expectedCaptureAcquisitions = 0;
			break;
		case 'ignored':
			filesystem.mkdirSync(path.join(root, 'ignored'));
			filesystem.writeFileSync(path.join(root, 'ignored/image.bin'), Buffer.from([0xff]));
			expectedSelectedFiles = [];
			expectedCaptureAcquisitions = 0;
			break;
		case 'assume-unchanged':
			gitCommand(root, ['update-index', '--assume-unchanged', checkedFile]);
			filesystem.writeFileSync(checked, 'BODY hidden flag change.\n');
			gitDiffBlind = true;
			break;
		case 'skip-worktree':
			gitCommand(root, ['update-index', '--skip-worktree', checkedFile]);
			filesystem.writeFileSync(checked, 'BODY hidden sparse change.\n');
			gitDiffBlind = true;
			break;
		case 'crlf':
		case 'clean-filter':
			filesystem.writeFileSync(checked, 'BODY initial.\r\n');
			gitDiffBlind = true;
			break;
		case 'stat-cache': {
			const metadata = filesystem.statSync(checked);
			const prior = filesystem.readFileSync(checked);
			const changed = Buffer.from(prior);
			// Keep the mechanical positive while changing same-size raw bytes.
			changed[prior.length - 2] = 0x21;
			assert.notDeepEqual(changed, prior);
			filesystem.writeFileSync(checked, changed);
			filesystem.utimesSync(checked, metadata.atime, metadata.mtime);
			gitDiffBlind = true;
			break;
		}
	}
	if (gitDiffBlind)
		assert.equal(
			gitCommand(root, ['diff', '--name-only', base, '--', checkedFile]),
			'',
			'Blind-spot fixture did not actually hide the raw change from Git',
		);
	return {
		name,
		root,
		base,
		expectedSelectedFiles,
		expectedCaptureAcquisitions,
		encodingRefusalPath,
		gitDiffBlind,
	};
}
