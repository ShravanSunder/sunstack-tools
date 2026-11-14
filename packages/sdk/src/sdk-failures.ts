// Closed adapter failures: no effects before dispatch, unknown effects after.
import type { SdkFailure } from './contracts/contracts.ts';

export type BeforeDispatchFailure = Extract<SdkFailure, { phase: 'before_dispatch' }>;
export type AfterDispatchFailure = Extract<SdkFailure, { phase: 'after_dispatch' }>;

export function beforeDispatch(
	reason: BeforeDispatchFailure['reason'],
	message: string,
): BeforeDispatchFailure {
	return { phase: 'before_dispatch', effects: 'none', reason, message };
}

export function afterDispatch(
	reason: AfterDispatchFailure['reason'],
	message: string,
): AfterDispatchFailure {
	return { phase: 'after_dispatch', effects: 'unknown', reason, message };
}
