import { createHash } from 'node:crypto';

import { Tiktoken } from 'js-tiktoken/lite';
import encodingRanks from 'js-tiktoken/ranks/o200k_base';

export interface AuthorUsageFile {
	readonly path: string;
	readonly source: string;
}
export interface AuthorFileMeasurement {
	readonly path: string;
	readonly bytes: number;
	readonly tokens: number;
	readonly sha256: string;
}
export interface UsageMeasurement {
	readonly usageBytes: number;
	readonly usageTokens: number;
	readonly tokenEncoding: 'o200k_base';
	readonly tokenizer: 'js-tiktoken@1.0.21';
	readonly authorFiles: readonly AuthorFileMeasurement[];
}

const sourceTokenizer = new Tiktoken(encodingRanks);
const usagePath =
	/^experiments\/(?:visual-plan|quick-json|markdown-directives|graph-diff|html-islands)\/(?!upstream\/)[\w/-]+\.(?:mdx|md|json|html|d2|mmd)$/;

export function measureUsageFiles(files: readonly AuthorUsageFile[]): UsageMeasurement {
	if (files.length === 0) throw new Error('At least one author usage file is required.');
	const filePaths = new Set<string>();
	const authorFiles = files.map((file): AuthorFileMeasurement => {
		if (!usagePath.test(file.path)) throw new Error(`Not a document usage file: ${file.path}`);
		if (filePaths.has(file.path)) throw new Error(`Duplicate author file: ${file.path}`);
		filePaths.add(file.path);
		return {
			path: file.path,
			bytes: Buffer.byteLength(file.source),
			tokens: sourceTokenizer.encode(file.source, [], []).length,
			sha256: createHash('sha256').update(file.source).digest('hex'),
		};
	});
	return {
		usageBytes: authorFiles.reduce((total, file): number => total + file.bytes, 0),
		usageTokens: authorFiles.reduce((total, file): number => total + file.tokens, 0),
		tokenEncoding: 'o200k_base',
		tokenizer: 'js-tiktoken@1.0.21',
		authorFiles,
	};
}
