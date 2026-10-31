import { z } from 'zod';

export const channelOptionsSchema = z.object({
	commit: z.string(), assets: z.string(), 'native-root': z.string(), fixtures: z.string(),
	evidence: z.string(), 'dependency-tar': z.string(), channel: z.enum(['npm', 'shell', 'brew']),
	pnpm: z.string().optional(),
});
export type ChannelOptions = z.infer<typeof channelOptionsSchema>;
const artifactVersionSchema = z.looseObject({
	version: z.string(), binary_sha256: z.record(z.string(), z.string()),
	archive_sha256: z.record(z.string(), z.string()),
});
export const artifactDeclarationSchema = z.object({ older: artifactVersionSchema, newer: artifactVersionSchema });
export const lintReportSchema = z.looseObject({
	status: z.looseObject({ kind: z.string() }),
	evidence_seen: z.record(z.string(), z.json()).optional(),
});
export const urlRepackProofSchema = z.looseObject({ before: z.record(z.string(), z.string()) });
export interface ChannelRow {
	readonly id: string; readonly version: string; readonly route: string; readonly case: string;
	readonly transcript: string; status: string; error?: string | undefined;
}
export type ChannelError =
	| { readonly label: string; readonly route: string; readonly cleanup: string | undefined }
	| { readonly matrix: 'incomplete'; readonly expected: number; readonly actual: number };
export interface ChannelObservation { readonly rows: ChannelRow[]; readonly errors: ChannelError[]; }
