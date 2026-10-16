/** Only content-hashed dependencies are immutable; stable major entry files revalidate. */
export function immutableAssetPath(path: string): boolean {
	return /^\/?v[1-9]\d*\/chunks\/[^/]+-[A-Za-z0-9_-]{8,}\.(?:m?js|css)$/.test(path);
}
