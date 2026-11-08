/** Shared artifact home for the browser tests and their project verification entrypoint. */
export const browserEvidenceRoot = 'tmp/verification/demistify/browser';

let evidenceRunId: string | undefined;

function browserEvidencePath(filename: string): string {
	evidenceRunId ??= crypto.randomUUID();
	return `${browserEvidenceRoot}/${evidenceRunId}/${filename}`;
}

export function browserScreenshotPath(filename: string): string {
	return `../../../../${browserEvidencePath(filename)}`;
}

/** Forensic JSON is written in CI too; the built-in writer resolves from the app project root. */
export async function writeBrowserEvidence(filename: string, content: string): Promise<void> {
	const { commands } = await import('vitest/browser');
	await commands.writeFile(`../../${browserEvidencePath(filename)}`, content);
}
