/**
 * Workout Type utility functions and constants
 */

export const WORKOUT_TYPE_VALUES = ['easy', 'tempo', 'intervals', 'long_run', 'race', "cross_training"] as const;
export type WorkoutType = (typeof WORKOUT_TYPE_VALUES)[number];

/**
 * All workout types with their display labels
 */
export const WORKOUT_TYPE_LABELS: { value: WorkoutType; label: string }[] = [
	{ value: 'easy', label: 'Easy' },
	{ value: 'tempo', label: 'Tempo' },
	{ value: 'intervals', label: 'Intervals' },
	{ value: 'long_run', label: 'Long Run' },
	{ value: 'race', label: 'Race' },
	{value: "cross_training", label: "Cross training"}
];

/**
 * Get the display label for a workout type
 * @param value - The workout type or null
 * @returns The formatted label string
 */
export const getWorkoutTypeLabel = (value: WorkoutType | null): string => {
	if (value === null) return 'Not set';
	const type = WORKOUT_TYPE_LABELS.find((t) => t.value === value);
	return type?.label ?? value;
};

/**
 * Get the color class for a workout type (for badges)
 * @param value - The workout type or null
 * @returns The CSS class name for the color
 */
export const getWorkoutTypeColor = (value: WorkoutType): string => {
	switch (value) {
		case 'easy':
			return 'workout-easy';
		case 'tempo':
			return 'workout-tempo';
		case 'intervals':
			return 'workout-intervals';
		case 'long_run':
			return 'workout-long-run';
		case 'race':
			return 'workout-race';
		case "cross_training":
			return "workout-cross-training"
	}
};

export const getWorkoutTypeClass = (value: WorkoutType): string => {
	switch (value) {
		case 'easy':
			return 'workout-easy';
		case 'tempo':
			return 'workout-tempo';
		case 'intervals':
			return 'workout-intervals';
		case 'long_run':
			return 'workout-long-run';
		case 'race':
			return 'workout-race';
		case "cross_training":
			return "workout-cross-training"
	}
};
