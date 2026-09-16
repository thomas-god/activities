import type { HooperIndex } from '$lib/api';

/** The subjective measures that make up a Hooper index. */
export const hooperMeasures = ['fatigue', 'sleep', 'pain', 'stress', 'mood'] as const;

export type HooperMeasure = (typeof hooperMeasures)[number];

/** Inclusive bounds of a single Hooper index measure. */
export const HOOPER_MEASURE_MIN = 1;
export const HOOPER_MEASURE_MAX = 10;

/** A Hooper index with every measure unset. */
export const emptyHooperIndex = (): HooperIndex => ({
	fatigue: null,
	sleep: null,
	pain: null,
	stress: null,
	mood: null
});
