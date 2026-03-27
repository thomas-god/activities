import type { Activity } from './api';
import { getSportCategory, type SportCategory } from './sport';
import type { WorkoutType } from './workout-type';

export interface ActivitiesFilters {
	rpe: number[];
	workoutTypes: WorkoutType[];
	sportCategories: SportCategory[];
}

export const filterActivities = (
	activities: Activity[],
	filters: ActivitiesFilters
): Activity[] => {
	const noActiveFilters =
		filters.rpe.length === 0 &&
		filters.workoutTypes.length === 0 &&
		filters.sportCategories.length === 0;

	if (noActiveFilters) {
		return activities;
	}

	return activities.filter((activity) => {
		if (filters.rpe.length > 0) {
			if (activity.rpe === null || !filters.rpe.includes(activity.rpe)) {
				return false;
			}
		}

		if (filters.workoutTypes.length > 0) {
			if (activity.workout_type === null || !filters.workoutTypes.includes(activity.workout_type)) {
				return false;
			}
		}

		if (filters.sportCategories.length > 0) {
			const activityCategory = activity.sport_category || getSportCategory(activity.sport);
			if (activityCategory === null || !filters.sportCategories.includes(activityCategory)) {
				return false;
			}
		}

		return true;
	});
};
