import type { Activity } from './api';
import { getSportCategory, type SportCategory } from './sport';
import type { WorkoutType } from './workout-type';

export interface ActivitiesFilters {
	rpe: number[];
	workoutTypes: WorkoutType[];
	sportCategories: SportCategory[];
}

export const emptyFilters = (): ActivitiesFilters => {
	return {
		rpe: [],
		workoutTypes: [],
		sportCategories: []
	};
};

export const filtersFromSearchParams = (params: URLSearchParams): ActivitiesFilters => {
	const filters: ActivitiesFilters = { rpe: [], workoutTypes: [], sportCategories: [] };

	const rpeParam = params.get('rpe');
	if (rpeParam) {
		filters.rpe = rpeParam
			.split(',')
			.map(Number)
			.filter((n) => !isNaN(n) && n >= 1 && n <= 10);
	}

	const wtParam = params.get('workout_type');
	if (wtParam) {
		filters.workoutTypes = wtParam.split(',') as WorkoutType[];
	}

	const scParam = params.get('sport_category');
	if (scParam) {
		filters.sportCategories = scParam.split(',') as SportCategory[];
	}

	return filters;
};

export const applyFiltersToSearchParams = (
	params: URLSearchParams,
	filters: ActivitiesFilters
): void => {
	if (filters.rpe.length > 0) {
		params.set('rpe', filters.rpe.join(','));
	} else {
		params.delete('rpe');
	}

	if (filters.workoutTypes.length > 0) {
		params.set('workout_type', filters.workoutTypes.join(','));
	} else {
		params.delete('workout_type');
	}

	if (filters.sportCategories.length > 0) {
		params.set('sport_category', filters.sportCategories.join(','));
	} else {
		params.delete('sport_category');
	}
};

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
