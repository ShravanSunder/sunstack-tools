// The cargo-dist package supplies binary.js; only this consumed surface is needed here.
interface BinaryPackage {
	exists(): boolean;
	install(suppressLogs: boolean): void | Promise<unknown>;
	readonly installDirectory: string;
	readonly binaries: { readonly sunstack: string };
}

export function getPackage(): BinaryPackage;
