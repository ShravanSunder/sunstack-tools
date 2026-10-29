// Preserve the release scripts' argparse interface while running on plain Node.
export class UsageError extends Error {}

interface ReleaseArgumentOptions {
	words: string[];
	flags: string[];
	required?: string[];
	positional?: string;
	choices?: Record<string, string[]>;
}
interface ReleaseArguments {
	help: boolean;
	values: Record<string, string[]>;
	positionals: string[];
}

export function parseReleaseArguments(options: ReleaseArgumentOptions): ReleaseArguments {
	const values: Record<string, string[]> = {};
	const positionals: string[] = [];
	const unrecognized: string[] = [];
	for (let index = 0; index < options.words.length; index++) {
		const word = options.words[index] ?? '';
		if (word === '-h' || word === '--help') return { help: true, values, positionals };
		if (word === '--') {
			positionals.push(...options.words.slice(index + 1));
			break;
		}
		if (!word.startsWith('-')) {
			positionals.push(word);
			continue;
		}
		const equals = word.indexOf('=');
		const option = equals < 0 ? word : word.slice(0, equals);
		const candidates = options.flags.filter((flag) => `--${flag}`.startsWith(option));
		if (candidates.length > 1)
			throw new UsageError(
				`ambiguous option: ${option} could match ${candidates.map((flag) => `--${flag}`).join(', ')}`,
			);
		const flag = candidates[0];
		if (!flag) {
			unrecognized.push(word);
			continue;
		}
		let value: string;
		if (equals >= 0) value = word.slice(equals + 1);
		else {
			const next = options.words[index + 1];
			if (next === undefined || next.startsWith('-'))
				throw new UsageError(`argument --${flag}: expected one argument`);
			value = next;
			index++;
		}
		const choices = options.choices?.[flag];
		if (choices && !choices.includes(value))
			throw new UsageError(
				`argument --${flag}: invalid choice: '${value}' (choose from ${choices.map((choice) => `'${choice}'`).join(', ')})`,
			);
		(values[flag] ??= []).push(value);
	}
	const missing = (options.required ?? [])
		.filter((flag) => !values[flag])
		.map((flag) => `--${flag}`);
	if (options.positional && !positionals.length) missing.push(options.positional);
	if (missing.length)
		throw new UsageError(`the following arguments are required: ${missing.join(', ')}`);
	if (!options.positional) unrecognized.push(...positionals);
	else if (positionals.length > 1) unrecognized.push(...positionals.slice(1));
	if (unrecognized.length)
		throw new UsageError(`unrecognized arguments: ${unrecognized.join(' ')}`);
	return { help: false, values, positionals };
}

export function printReleaseError(error: unknown, help: string, program: string): void {
	const message = error instanceof Error ? error.message : String(error);
	if (error instanceof UsageError) {
		process.stderr.write(`${help.split('\n\n', 1)[0]}\n${program}: error: ${message}\n`);
		process.exitCode = 2;
	} else {
		process.stderr.write(`${message}\n`);
		process.exitCode = 1;
	}
}
