/**
 * RPE (Rate of Perceived Exertion) utility functions and constants
 *
 * RPE Categories:
 * - Easy: 1-3
 * - Moderate: 4-6
 * - Hard: 7-8
 * - Very Hard: 9
 * - Max: 10
 */

/**
 * All valid RPE values (1-10)
 */
export const RPE_VALUES = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10] as const;

/**
 * Get the display label for an RPE value as a scale out of 10
 * @param value - The RPE value (1-10) or null
 * @returns The formatted label string
 */
export const getRpeLabelAsScale = (value: number | null): string => {
	if (value === null) return 'Not set';
	return `${value}/10`;
};

/**
 * Get the color class for an RPE value (for badges)
 * @param value - The RPE value (1-10) or null
 * @returns The CSS class name for the color
 */
export const getRpeColor = (value: number): string => {
	if (value <= 3) return 'rpe-easy';
	if (value <= 6) return 'rpe-moderate';
	if (value <= 8) return 'rpe-hard';
	if (value === 9) return 'rpe-very-hard';
	return 'rpe-max';
};

export const getRpeClass = (value: number): string => {
	if (value <= 3) return 'rpe-easy';
	if (value <= 6) return 'rpe-moderate';
	if (value <= 8) return 'rpe-hard';
	if (value === 9) return 'rpe-very-hard';
	return 'rpe-max';
};
