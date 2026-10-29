// Records exchanged by the release observation scripts. Unknown JSON stays opaque.
import { z } from 'zod';

export const packageMetadataSchema = z.looseObject({
	name: z.string(),
	version: z.string(),
	bin: z.looseObject({ sunstack: z.string() }).optional(),
	artifactDownloadUrls: z.array(z.string()).optional(),
});
export type PackageMetadata = z.infer<typeof packageMetadataSchema>;
export const dependencySchema = z.looseObject({ version: z.string(), integrity: z.string() });
export const packageLockSchema = z.looseObject({
	packages: z.looseObject({ 'node_modules/detect-libc': dependencySchema }),
});
const releaseVersionSchema = z.looseObject({
	version: z.string(),
	binary_sha256: z.record(z.string(), z.string()).optional(),
	archive_sha256: z.record(z.string(), z.string()).optional(),
});
export const declarationSchema = z.looseObject({
	older: releaseVersionSchema,
	newer: releaseVersionSchema,
});
export type ReleaseDeclaration = z.infer<typeof declarationSchema>;
export const heldRequestSchema = z.looseObject({
	state: z.unknown(),
	questions: z.record(z.string(), z.unknown()),
});
export type HeldRequest = z.infer<typeof heldRequestSchema>;
export const lintConfigSchema = z.looseObject({ backend: z.looseObject({ endpoint: z.string() }) });
export const reportSchema = z.looseObject({
	status: z.looseObject({ kind: z.string() }).optional(),
	evidence_seen: z.record(z.string(), z.unknown()).optional(),
});
export type LintReport = z.infer<typeof reportSchema>;
export const provenanceSchema = z.looseObject({
	repository: z.string(),
	version: z.string(),
	formula_sha256: z.string(),
});
export const identitySchema = z.looseObject({
	tag: z.string(),
	version: z.string(),
	binary_sha256: z.string(),
	asset_sha256: z.record(z.string(), z.string()),
});
export const workStepSchema = z.object({ pid: z.number(), group: z.number(), binary: z.string() });
export type WorkStep = z.infer<typeof workStepSchema>;
export const targetSchema = z.object({
	repository: z.string(),
	run: z.string(),
	head: z.string(),
	workflow: z.string(),
});
export type ControllerTarget = z.infer<typeof targetSchema>;
export const repositorySchema = z.looseObject({ full_name: z.string(), private: z.boolean() });
export type RepositoryMetadata = z.infer<typeof repositorySchema>;
export const runSchema = z.looseObject({
	id: z.number(),
	repository: repositorySchema,
	head_sha: z.string(),
	name: z.string(),
	event: z.string(),
	status: z.string(),
});
export type RunMetadata = z.infer<typeof runSchema>;
export interface ProcessMember {
	pid: number;
	parent: number;
	group: number;
	state: string;
	started: string;
	command: string;
}
export interface OwnershipCensus {
	kind: 'established' | 'unestablished';
	initial_root: ProcessMember | null;
	initial_group_members: ProcessMember[];
}
export type OwnedObserver = (() => ProcessMember[]) & { ownership?: () => OwnershipCensus };
export interface ProcessEnding {
	code: number | null;
	signal: NodeJS.Signals | null;
	elapsed_ms: number;
}
export interface StopRequest {
	signal: NodeJS.Signals;
	destination: string;
	expectation: string;
}
export interface NativeIdentity {
	pid: number;
	file: string;
	sha256: string;
}
export interface ProcessObservation {
	command: string[];
	cwd: string;
	pid: number | null;
	root_exit: ProcessEnding | null;
	root_close: ProcessEnding | null;
	error: string | null;
	elapsed_ms: number;
	admitted_request: HeldRequest | null;
	signal_at_ms: number | null;
	stop: StopRequest | null;
	owned_tree_at_readiness: ProcessMember[];
	native_identity_at_readiness: NativeIdentity[];
	timeline: unknown[];
	ownership: OwnershipCensus;
	survivors_before_cleanup: ProcessMember[] | null;
	unestablished_group_members_before_cleanup: ProcessMember[] | null;
	sockets_before_cleanup: number;
}
export interface CapturedProcess {
	stdout: Buffer;
	stderr: Buffer;
	observation: ProcessObservation;
}
export interface CaptureOptions {
	command: string[];
	cwd: string;
	environment: NodeJS.ProcessEnv;
	stem: string;
	stop?: StopRequest;
	expectedNativeSha?: string;
	timeoutMs?: number;
}
export type RawCommand = (
	command: string[],
	cwd: string,
	environment: NodeJS.ProcessEnv,
	stem: string,
) => Promise<CapturedProcess>;

export function errorCode(error: unknown): unknown {
	return error !== null && typeof error === 'object' && 'code' in error ? error.code : undefined;
}

export function errorMessage(error: unknown): string {
	return error instanceof Error ? error.message : String(error);
}

export function errorStack(error: unknown): string {
	return error instanceof Error
		? error.stack && error.stack.length > 0
			? error.stack
			: error.message
		: String(error);
}
