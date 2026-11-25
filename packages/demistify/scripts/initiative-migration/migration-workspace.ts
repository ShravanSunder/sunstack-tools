import { relative, sep } from 'node:path';

/** One project of the initiative, as it is before the run or as the run creates it. */
export interface MigrationProject {
	readonly name: string;
	readonly directory: string;
	readonly designFile: string;
	readonly created: boolean;
}

/** Authored files read from the tree; the run never writes generated files. */
export interface SourceWorkspace {
	read(file: string): string | undefined;
	write(file: string, text: string): void;
	remove(file: string): void;
}

export interface PageDeclaration {
	readonly id: string;
	readonly title: string;
	readonly href: string;
	readonly icon: string;
	/** Why the project now declares the page; printed, not written. */
	readonly reason: string;
}

export const sdkFilename = 'design-sdk.generated.ts';

export function relativeHref(fromDirectory: string, file: string): string {
	return relative(fromDirectory, file).split(sep).map(encodeURIComponent).join('/');
}
