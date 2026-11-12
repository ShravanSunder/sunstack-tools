/** Content-hashed dependencies and CLI archives are immutable; stable major entries revalidate. */
export function immutableAssetPath(path: string): boolean {
	return /^\/?v[1-9]\d*\/(?:chunks\/[^/]+-[A-Za-z0-9_-]{8,}\.(?:m?js|css)|demistify-cli-[a-f0-9]{64}\.tgz)$/.test(
		path,
	);
}
