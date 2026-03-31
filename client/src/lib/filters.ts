import type { Activity } from './api';
import { getSportCategory, type SportCategory } from './sport';
import type { WorkoutType } from './workout-type';

export interface RangeFilter {
	min: number | null;
	max: number | null;
}

export interface ActivitiesFilters {
	rpe: number[];
	workoutTypes: WorkoutType[];
	sportCategories: SportCategory[];
	durationRange: RangeFilter;
	distanceRange: RangeFilter;
	elevationRange: RangeFilter;
}

export const emptyFilters = (): ActivitiesFilters => {
	return {
		rpe: [],
		workoutTypes: [],
		sportCategories: [],
		durationRange: { min: null, max: null },
		distanceRange: { min: null, max: null },
		elevationRange: { min: null, max: null }
	};
};

const parseRangeParam = (params: URLSearchParams, key: string): number | null => {
	const raw = params.get(key);
	if (raw === null) return null;
	const n = Number(raw);
	return isNaN(n) ? null : n;
};

export const filtersFromSearchParams = (params: URLSearchParams): ActivitiesFilters => {
	const filters: ActivitiesFilters = {
		rpe: [],
		workoutTypes: [],
		sportCategories: [],
		durationRange: { min: null, max: null },
		distanceRange: { min: null, max: null },
		elevationRange: { min: null, max: null }
	};

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

	filters.durationRange = {
		min: parseRangeParam(params, 'duration_min'),
		max: parseRangeParam(params, 'duration_max')
	};
	filters.distanceRange = {
		min: parseRangeParam(params, 'distance_min'),
		max: parseRangeParam(params, 'distance_max')
	};
	filters.elevationRange = {
		min: parseRangeParam(params, 'elevation_min'),
		max: parseRangeParam(params, 'elevation_max')
	};

	return filters;
};

const applyRangeParam = (
	params: URLSearchParams,
	minKey: string,
	maxKey: string,
	range: RangeFilter
): void => {
	if (range.min !== null) {
		params.set(minKey, String(range.min));
	} else {
		params.delete(minKey);
	}
	if (range.max !== null) {
		params.set(maxKey, String(range.max));
	} else {
		params.delete(maxKey);
	}
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

	applyRangeParam(params, 'duration_min', 'duration_max', filters.durationRange);
	applyRangeParam(params, 'distance_min', 'distance_max', filters.distanceRange);
	applyRangeParam(params, 'elevation_min', 'elevation_max', filters.elevationRange);
};

const applyRangeFilter = (value: number | undefined, range: RangeFilter): boolean => {
	if (range.min !== null || range.max !== null) {
		if (value === undefined) return false;
		if (range.min !== null && value < range.min) return false;
		if (range.max !== null && value > range.max) return false;
	}
	return true;
};

export const filterActivities = (
	activities: Activity[],
	filters: ActivitiesFilters
): Activity[] => {
	const noActiveFilters =
		filters.rpe.length === 0 &&
		filters.workoutTypes.length === 0 &&
		filters.sportCategories.length === 0 &&
		filters.durationRange.min === null &&
		filters.durationRange.max === null &&
		filters.distanceRange.min === null &&
		filters.distanceRange.max === null &&
		filters.elevationRange.min === null &&
		filters.elevationRange.max === null;

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

		if (!applyRangeFilter(activity.statistics['Duration'], filters.durationRange)) return false;
		if (!applyRangeFilter(activity.statistics['Distance'], filters.distanceRange)) return false;
		if (!applyRangeFilter(activity.statistics['Elevation'], filters.elevationRange)) return false;

		return true;
	});
};
