import { RequirementsIsland } from './requirements.ts';

export class CallStackIsland extends RequirementsIsland {
	protected override readonly readerKind = 'call-stack' as const;
}
