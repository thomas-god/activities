import { describe, expect, it } from 'vitest';
import type { Activity } from './api';
import {
	filterActivities,
	filtersFromSearchParams,
	applyFiltersToSearchParams,
	type ActivitiesFilters
} from './filters';

const makeActivity = (overrides: Partial<Activity> = {}): Activity => ({
	id: 'test-id',
	name: null,
	sport: 'Running',
	sport_category: null,
	start_time: '2024-01-01T00:00:00Z',
	rpe: null,
	workout_type: null,
	feedback: null,
	nutrition: null,
	statistics: {},
	...overrides
});

const emptyRange = { min: null, max: null };
const noFilters = {
	rpe: [],
	workoutTypes: [],
	sportCategories: [],
	durationRange: emptyRange,
	distanceRange: emptyRange,
	elevationRange: emptyRange
};

describe('filterActivities', () => {
	describe('empty filters', () => {
		it('returns all activities unfiltered when all filters are empty', () => {
			const activities = [makeActivity({ rpe: 5 }), makeActivity({ workout_type: 'easy' })];

			expect(filterActivities(activities, noFilters)).toEqual(activities);
		});
	});

	describe('RPE filter', () => {
		it('includes activities whose RPE is in the filter list', () => {
			const activity = makeActivity({ rpe: 7 });

			const result = filterActivities([activity], { ...noFilters, rpe: [7, 8] });

			expect(result).toEqual([activity]);
		});

		it('excludes activities whose RPE is not in the filter list', () => {
			const activity = makeActivity({ rpe: 5 });

			const result = filterActivities([activity], { ...noFilters, rpe: [7, 8] });

			expect(result).toEqual([]);
		});

		it('excludes activities with RPE not set when the RPE filter is set', () => {
			const activity = makeActivity({ rpe: null });

			const result = filterActivities([activity], { ...noFilters, rpe: [5] });

			expect(result).toEqual([]);
		});
	});

	describe('workout type filter', () => {
		it('includes activities whose workout type is in the filter list', () => {
			const activity = makeActivity({ workout_type: 'tempo' });

			const result = filterActivities([activity], { ...noFilters, workoutTypes: ['tempo'] });

			expect(result).toEqual([activity]);
		});

		it('excludes activities whose workout type is not in the filter list', () => {
			const activity = makeActivity({ workout_type: 'easy' });

			const result = filterActivities([activity], { ...noFilters, workoutTypes: ['tempo'] });

			expect(result).toEqual([]);
		});

		it('excludes activities with no workout type set when the workout type filter is set', () => {
			const activity = makeActivity({ workout_type: null });

			const result = filterActivities([activity], { ...noFilters, workoutTypes: ['easy'] });

			expect(result).toEqual([]);
		});
	});

	describe('sport category filter', () => {
		it('includes activities whose sport_category field matches the filter', () => {
			const activity = makeActivity({ sport_category: 'Running' });

			const result = filterActivities([activity], {
				...noFilters,
				sportCategories: ['Running']
			});

			expect(result).toEqual([activity]);
		});

		it('excludes activities whose sport_category field does not match the filter', () => {
			const activity = makeActivity({ sport_category: 'Cycling' });

			const result = filterActivities([activity], {
				...noFilters,
				sportCategories: ['Running']
			});

			expect(result).toEqual([]);
		});

		it('falls back to deriving the category from the sport when sport_category is null', () => {
			// Running sport maps to Running category via getSportCategory
			const activity = makeActivity({ sport: 'Running', sport_category: null });

			const result = filterActivities([activity], {
				...noFilters,
				sportCategories: ['Running']
			});

			expect(result).toEqual([activity]);
		});

		it('excludes activities whose derived sport category does not match the filter', () => {
			const activity = makeActivity({ sport: 'Running', sport_category: null });

			const result = filterActivities([activity], {
				...noFilters,
				sportCategories: ['Cycling']
			});

			expect(result).toEqual([]);
		});
	});

	describe('multiple filters (AND logic)', () => {
		it('includes an activity that matches all active filters', () => {
			const activity = makeActivity({ rpe: 6, workout_type: 'tempo', sport_category: 'Running' });

			const result = filterActivities([activity], {
				...noFilters,
				rpe: [6],
				workoutTypes: ['tempo'],
				sportCategories: ['Running']
			});

			expect(result).toEqual([activity]);
		});

		it('excludes an activity that matches some but not all active filters', () => {
			const matchesRpeOnly = makeActivity({ id: '1', rpe: 6, workout_type: 'race' });
			const matchesWorkoutOnly = makeActivity({ id: '2', rpe: 3, workout_type: 'easy' });

			const result = filterActivities([matchesRpeOnly, matchesWorkoutOnly], {
				...noFilters,
				rpe: [6],
				workoutTypes: ['easy']
			});

			expect(result).toEqual([]);
		});

		it('ignores unset filters', () => {
			const activity = makeActivity({ rpe: 6, workout_type: 'tempo', sport_category: 'Cycling' });

			const result = filterActivities([activity], {
				...noFilters,
				rpe: [6],
				workoutTypes: ['tempo']
				// sportCategories not set — should not affect result
			});

			expect(result).toEqual([activity]);
		});
	});

	describe('duration range filter', () => {
		it('includes an activity whose duration is within the range', () => {
			const activity = makeActivity({ statistics: { Duration: 3600 } });

			const result = filterActivities([activity], {
				...noFilters,
				durationRange: { min: 1800, max: 7200 }
			});

			expect(result).toEqual([activity]);
		});

		it('excludes an activity whose duration is below the minimum', () => {
			const activity = makeActivity({ statistics: { Duration: 600 } });

			const result = filterActivities([activity], {
				...noFilters,
				durationRange: { min: 1800, max: null }
			});

			expect(result).toEqual([]);
		});

		it('excludes an activity whose duration is above the maximum', () => {
			const activity = makeActivity({ statistics: { Duration: 10000 } });

			const result = filterActivities([activity], {
				...noFilters,
				durationRange: { min: null, max: 7200 }
			});

			expect(result).toEqual([]);
		});

		it('excludes an activity with no Duration statistic when a duration range is set', () => {
			const activity = makeActivity({ statistics: {} });

			const result = filterActivities([activity], {
				...noFilters,
				durationRange: { min: 0, max: null }
			});

			expect(result).toEqual([]);
		});

		it('includes an activity when only min is set and the value equals min', () => {
			const activity = makeActivity({ statistics: { Duration: 1800 } });

			const result = filterActivities([activity], {
				...noFilters,
				durationRange: { min: 1800, max: null }
			});

			expect(result).toEqual([activity]);
		});
	});

	describe('distance range filter', () => {
		it('includes an activity whose distance is within the range', () => {
			const activity = makeActivity({ statistics: { Distance: 8000 } });

			const result = filterActivities([activity], {
				...noFilters,
				distanceRange: { min: 5000, max: 10000 }
			});

			expect(result).toEqual([activity]);
		});

		it('excludes an activity whose distance is below the minimum', () => {
			const activity = makeActivity({ statistics: { Distance: 2000 } });

			const result = filterActivities([activity], {
				...noFilters,
				distanceRange: { min: 5000, max: null }
			});

			expect(result).toEqual([]);
		});

		it('excludes an activity whose distance is above the maximum', () => {
			const activity = makeActivity({ statistics: { Distance: 15000 } });

			const result = filterActivities([activity], {
				...noFilters,
				distanceRange: { min: null, max: 10000 }
			});

			expect(result).toEqual([]);
		});

		it('excludes an activity with no Distance statistic when a distance range is set', () => {
			const activity = makeActivity({ statistics: {} });

			const result = filterActivities([activity], {
				...noFilters,
				distanceRange: { min: 0, max: null }
			});

			expect(result).toEqual([]);
		});
	});

	describe('elevation range filter', () => {
		it('includes an activity whose elevation is within the range', () => {
			const activity = makeActivity({ statistics: { Elevation: 300 } });

			const result = filterActivities([activity], {
				...noFilters,
				elevationRange: { min: 100, max: 500 }
			});

			expect(result).toEqual([activity]);
		});

		it('excludes an activity whose elevation is below the minimum', () => {
			const activity = makeActivity({ statistics: { Elevation: 50 } });

			const result = filterActivities([activity], {
				...noFilters,
				elevationRange: { min: 100, max: null }
			});

			expect(result).toEqual([]);
		});

		it('excludes an activity whose elevation is above the maximum', () => {
			const activity = makeActivity({ statistics: { Elevation: 800 } });

			const result = filterActivities([activity], {
				...noFilters,
				elevationRange: { min: null, max: 500 }
			});

			expect(result).toEqual([]);
		});

		it('excludes an activity with no Elevation statistic when an elevation range is set', () => {
			const activity = makeActivity({ statistics: {} });

			const result = filterActivities([activity], {
				...noFilters,
				elevationRange: { min: 0, max: null }
			});

			expect(result).toEqual([]);
		});
	});
});

describe('filtersFromSearchParams', () => {
	it('returns empty filters when params are absent', () => {
		const params = new URLSearchParams();
		expect(filtersFromSearchParams(params)).toEqual({
			rpe: [],
			workoutTypes: [],
			sportCategories: [],
			durationRange: { min: null, max: null },
			distanceRange: { min: null, max: null },
			elevationRange: { min: null, max: null }
		});
	});

	it('parses a single RPE value', () => {
		const params = new URLSearchParams('rpe=7');
		expect(filtersFromSearchParams(params).rpe).toEqual([7]);
	});

	it('parses multiple RPE values', () => {
		const params = new URLSearchParams('rpe=3,7,9');
		expect(filtersFromSearchParams(params).rpe).toEqual([3, 7, 9]);
	});

	it('ignores RPE values outside the 1-10 range', () => {
		const params = new URLSearchParams('rpe=0,5,11');
		expect(filtersFromSearchParams(params).rpe).toEqual([5]);
	});

	it('ignores non-numeric RPE values', () => {
		const params = new URLSearchParams('rpe=abc,5,NaN');
		expect(filtersFromSearchParams(params).rpe).toEqual([5]);
	});

	it('parses workout types', () => {
		const params = new URLSearchParams('workout_type=easy,tempo');
		expect(filtersFromSearchParams(params).workoutTypes).toEqual(['easy', 'tempo']);
	});

	it('parses sport categories', () => {
		const params = new URLSearchParams('sport_category=Running,Cycling');
		expect(filtersFromSearchParams(params).sportCategories).toEqual(['Running', 'Cycling']);
	});

	it('parses all three filters together', () => {
		const params = new URLSearchParams('rpe=5&workout_type=tempo&sport_category=Running');
		expect(filtersFromSearchParams(params)).toEqual({
			rpe: [5],
			workoutTypes: ['tempo'],
			sportCategories: ['Running'],
			durationRange: { min: null, max: null },
			distanceRange: { min: null, max: null },
			elevationRange: { min: null, max: null }
		});
	});

	it('parses duration_min and duration_max', () => {
		const params = new URLSearchParams('duration_min=1800&duration_max=3600');
		const filters = filtersFromSearchParams(params);
		expect(filters.durationRange).toEqual({ min: 1800, max: 3600 });
	});

	it('parses only duration_min when duration_max is absent', () => {
		const params = new URLSearchParams('duration_min=600');
		expect(filtersFromSearchParams(params).durationRange).toEqual({ min: 600, max: null });
	});

	it('parses distance_min and distance_max', () => {
		const params = new URLSearchParams('distance_min=5000&distance_max=10000');
		expect(filtersFromSearchParams(params).distanceRange).toEqual({ min: 5000, max: 10000 });
	});

	it('parses elevation_min and elevation_max', () => {
		const params = new URLSearchParams('elevation_min=100&elevation_max=500');
		expect(filtersFromSearchParams(params).elevationRange).toEqual({ min: 100, max: 500 });
	});

	it('ignores non-numeric range values', () => {
		const params = new URLSearchParams('duration_min=abc&duration_max=NaN');
		expect(filtersFromSearchParams(params).durationRange).toEqual({ min: null, max: null });
	});
});

describe('applyFiltersToSearchParams', () => {
	it('sets rpe param when rpe filter is non-empty', () => {
		const params = new URLSearchParams();
		applyFiltersToSearchParams(params, { ...noFilters, rpe: [3, 7] });
		expect(params.get('rpe')).toBe('3,7');
	});

	it('deletes rpe param when rpe filter is empty', () => {
		const params = new URLSearchParams('rpe=5');
		applyFiltersToSearchParams(params, { ...noFilters });
		expect(params.has('rpe')).toBe(false);
	});

	it('sets workout_type param when workoutTypes filter is non-empty', () => {
		const params = new URLSearchParams();
		applyFiltersToSearchParams(params, { ...noFilters, workoutTypes: ['easy', 'tempo'] });
		expect(params.get('workout_type')).toBe('easy,tempo');
	});

	it('deletes workout_type param when workoutTypes filter is empty', () => {
		const params = new URLSearchParams('workout_type=easy');
		applyFiltersToSearchParams(params, { ...noFilters });
		expect(params.has('workout_type')).toBe(false);
	});

	it('sets sport_category param when sportCategories filter is non-empty', () => {
		const params = new URLSearchParams();
		applyFiltersToSearchParams(params, { ...noFilters, sportCategories: ['Running'] });
		expect(params.get('sport_category')).toBe('Running');
	});

	it('deletes sport_category param when sportCategories filter is empty', () => {
		const params = new URLSearchParams('sport_category=Running');
		applyFiltersToSearchParams(params, { ...noFilters });
		expect(params.has('sport_category')).toBe(false);
	});

	it('sets all three params when all filters are non-empty', () => {
		const params = new URLSearchParams();
		applyFiltersToSearchParams(params, {
			...noFilters,
			rpe: [5],
			workoutTypes: ['tempo'],
			sportCategories: ['Cycling']
		});
		expect(params.get('rpe')).toBe('5');
		expect(params.get('workout_type')).toBe('tempo');
		expect(params.get('sport_category')).toBe('Cycling');
	});

	it('sets duration_min and duration_max params when durationRange is set', () => {
		const params = new URLSearchParams();
		applyFiltersToSearchParams(params, {
			...noFilters,
			durationRange: { min: 1800, max: 3600 }
		});
		expect(params.get('duration_min')).toBe('1800');
		expect(params.get('duration_max')).toBe('3600');
	});

	it('omits duration params when durationRange bounds are null', () => {
		const params = new URLSearchParams('duration_min=600&duration_max=7200');
		applyFiltersToSearchParams(params, { ...noFilters });
		expect(params.has('duration_min')).toBe(false);
		expect(params.has('duration_max')).toBe(false);
	});

	it('sets only duration_min when max is null', () => {
		const params = new URLSearchParams();
		applyFiltersToSearchParams(params, { ...noFilters, durationRange: { min: 600, max: null } });
		expect(params.get('duration_min')).toBe('600');
		expect(params.has('duration_max')).toBe(false);
	});

	it('sets distance_min and distance_max params when distanceRange is set', () => {
		const params = new URLSearchParams();
		applyFiltersToSearchParams(params, { ...noFilters, distanceRange: { min: 5000, max: 10000 } });
		expect(params.get('distance_min')).toBe('5000');
		expect(params.get('distance_max')).toBe('10000');
	});

	it('sets elevation_min and elevation_max params when elevationRange is set', () => {
		const params = new URLSearchParams();
		applyFiltersToSearchParams(params, { ...noFilters, elevationRange: { min: 100, max: 500 } });
		expect(params.get('elevation_min')).toBe('100');
		expect(params.get('elevation_max')).toBe('500');
	});

	it('round-trips: applyFiltersToSearchParams then filtersFromSearchParams returns the original filters', () => {
		const original: ActivitiesFilters = {
			rpe: [4, 8],
			workoutTypes: ['race'],
			sportCategories: ['Running'],
			durationRange: { min: 1800, max: 7200 },
			distanceRange: { min: 5000, max: null },
			elevationRange: { min: null, max: 1000 }
		};
		const params = new URLSearchParams();
		applyFiltersToSearchParams(params, original);
		expect(filtersFromSearchParams(params)).toEqual(original);
	});
});
