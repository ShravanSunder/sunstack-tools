/** Shared artifact home for the browser tests and their project verification entrypoint. */
export const browserEvidenceRoot = 'tmp/verification/demistify/browser';

let evidenceRunId: string | undefined;

export function browserScreenshotPath(filename: string): string {
	evidenceRunId ??= crypto.randomUUID();
	return `../../../../${browserEvidenceRoot}/${evidenceRunId}/${filename}`;
}
