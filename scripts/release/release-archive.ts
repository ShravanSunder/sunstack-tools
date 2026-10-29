import { readFileSync } from 'node:fs';
import { gunzipSync } from 'node:zlib';

interface ArchiveMember {
	name: string;
	data: Buffer;
	regular: boolean;
}

function tarText(bytes: Buffer): string {
	return bytes.toString('utf8').replace(/\0.*$/s, '');
}

function tarNumber(bytes: Buffer): number {
	if ((bytes[0] ?? 0) & 0x80) {
		let result = BigInt((bytes[0] ?? 0) & 0x7f);
		for (const byte of bytes.subarray(1)) result = result * 256n + BigInt(byte);
		return Number(result);
	}
	return parseInt(tarText(bytes).trim() || '0', 8);
}

function paxFields(bytes: Buffer): Record<string, string> {
	const fields: Record<string, string> = {};
	for (let offset = 0; offset < bytes.length;) {
		const separator = bytes.indexOf(32, offset);
		const length = Number(bytes.subarray(offset, separator).toString());
		if (separator < offset || !Number.isSafeInteger(length) || length <= 0)
			throw new Error('invalid tar extended header');
		const record = bytes.subarray(separator + 1, offset + length - 1).toString();
		const equals = record.indexOf('=');
		fields[record.slice(0, equals)] = record.slice(equals + 1);
		offset += length;
	}
	return fields;
}

// Read regular-file bytes without extracting archive paths into the filesystem.
export function readReleaseArchive(archive: string): ArchiveMember[] {
	const bytes = gunzipSync(readFileSync(archive));
	const members: ArchiveMember[] = [];
	const globalFields: Record<string, string> = {};
	let localFields: Record<string, string> = {};
	let longName: string | undefined;
	for (let offset = 0; offset + 512 <= bytes.length;) {
		const header = bytes.subarray(offset, offset + 512);
		if (header.every((byte) => byte === 0)) break;
		const checksum = header.reduce(
			(sum, byte, index) => sum + (index >= 148 && index < 156 ? 32 : byte),
			0,
		);
		if (checksum !== tarNumber(header.subarray(148, 156)))
			throw new Error('invalid tar header checksum');
		const kind = tarText(header.subarray(156, 157));
		const fields = { ...globalFields, ...localFields };
		const size =
			fields['size'] === undefined ? tarNumber(header.subarray(124, 136)) : Number(fields['size']);
		if (!Number.isSafeInteger(size) || size < 0 || offset + 512 + size > bytes.length)
			throw new Error('invalid tar member size');
		const data = bytes.subarray(offset + 512, offset + 512 + size);
		offset += 512 + Math.ceil(size / 512) * 512;
		if (kind === 'x') {
			localFields = paxFields(data);
			continue;
		}
		if (kind === 'g') {
			Object.assign(globalFields, paxFields(data));
			continue;
		}
		if (kind === 'L') {
			longName = tarText(data);
			continue;
		}
		if (kind === 'K') continue;
		const prefix =
			tarText(header.subarray(257, 263)) === 'ustar' ? tarText(header.subarray(345, 500)) : '';
		const name =
			fields['path'] ??
			longName ??
			`${prefix ? `${prefix}/` : ''}${tarText(header.subarray(0, 100))}`;
		members.push({ name, data, regular: kind === '' || kind === '0' || kind === '7' });
		localFields = {};
		longName = undefined;
	}
	return members;
}
