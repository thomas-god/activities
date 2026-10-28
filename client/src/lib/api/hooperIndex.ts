import * as z from 'zod';

/** A subjective Hooper index measure, constrained to the 0..=10 range. */
export const SubjectiveScaleSchema = z.number().int().min(0).max(10);

/**
 * Subjective feedback (Hooper's index): self-assessed measures for fatigue,
 * sleep, pain, stress and mood. A `null` measure means it was not recorded.
 */
export const HooperIndexSchema = z.object({
	fatigue: SubjectiveScaleSchema.nullable(),
	sleep: SubjectiveScaleSchema.nullable(),
	pain: SubjectiveScaleSchema.nullable(),
	stress: SubjectiveScaleSchema.nullable(),
	mood: SubjectiveScaleSchema.nullable()
});

export type HooperIndex = z.infer<typeof HooperIndexSchema>;
