import { z } from 'zod';

import referenceText from './public-part-style-reference.json?raw';

/** Fixed observed values precede the public-part CSS correction; candidate code supplies none. */
export const publicPartStyleReference = z
	.object({
		basis: z.string(),
		profiles: z
			.array(
				z.object({
					width: z.number(),
					theme: z.string(),
					motion: z.string(),
					source: z.string(),
					sha256: z.string(),
					samples: z
						.array(
							z.object({
								stage: z.string(),
								selector: z.string(),
								display: z.string(),
								margin: z.string(),
								padding: z.string(),
								color: z.string(),
								background: z.string(),
								font: z.string(),
								radius: z.string(),
							}),
						)
						.length(16),
				}),
			)
			.length(8),
	})
	.parse(JSON.parse(referenceText));
