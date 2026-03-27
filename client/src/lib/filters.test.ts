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

const noFilters = { rpe: [], workoutTypes: [], sportCategories: [] };

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
});

describe('filtersFromSearchParams', () => {
	it('returns empty filters when params are absent', () => {
		const params = new URLSearchParams();
		expect(filtersFromSearchParams(params)).toEqual({
			rpe: [],
			workoutTypes: [],
			sportCategories: []
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
			sportCategories: ['Running']
		});
	});
});

describe('applyFiltersToSearchParams', () => {
	it('sets rpe param when rpe filter is non-empty', () => {
		const params = new URLSearchParams();
		applyFiltersToSearchParams(params, { rpe: [3, 7], workoutTypes: [], sportCategories: [] });
		expect(params.get('rpe')).toBe('3,7');
	});

	it('deletes rpe param when rpe filter is empty', () => {
		const params = new URLSearchParams('rpe=5');
		applyFiltersToSearchParams(params, { rpe: [], workoutTypes: [], sportCategories: [] });
		expect(params.has('rpe')).toBe(false);
	});

	it('sets workout_type param when workoutTypes filter is non-empty', () => {
		const params = new URLSearchParams();
		applyFiltersToSearchParams(params, {
			rpe: [],
			workoutTypes: ['easy', 'tempo'],
			sportCategories: []
		});
		expect(params.get('workout_type')).toBe('easy,tempo');
	});

	it('deletes workout_type param when workoutTypes filter is empty', () => {
		const params = new URLSearchParams('workout_type=easy');
		applyFiltersToSearchParams(params, { rpe: [], workoutTypes: [], sportCategories: [] });
		expect(params.has('workout_type')).toBe(false);
	});

	it('sets sport_category param when sportCategories filter is non-empty', () => {
		const params = new URLSearchParams();
		applyFiltersToSearchParams(params, { rpe: [], workoutTypes: [], sportCategories: ['Running'] });
		expect(params.get('sport_category')).toBe('Running');
	});

	it('deletes sport_category param when sportCategories filter is empty', () => {
		const params = new URLSearchParams('sport_category=Running');
		applyFiltersToSearchParams(params, { rpe: [], workoutTypes: [], sportCategories: [] });
		expect(params.has('sport_category')).toBe(false);
	});

	it('sets all three params when all filters are non-empty', () => {
		const params = new URLSearchParams();
		applyFiltersToSearchParams(params, {
			rpe: [5],
			workoutTypes: ['tempo'],
			sportCategories: ['Cycling']
		});
		expect(params.get('rpe')).toBe('5');
		expect(params.get('workout_type')).toBe('tempo');
		expect(params.get('sport_category')).toBe('Cycling');
	});

	it('round-trips: applyFiltersToSearchParams then filtersFromSearchParams returns the original filters', () => {
		const original: ActivitiesFilters = {
			rpe: [4, 8],
			workoutTypes: ['race'],
			sportCategories: ['Running']
		};
		const params = new URLSearchParams();
		applyFiltersToSearchParams(params, original);
		expect(filtersFromSearchParams(params)).toEqual(original);
	});
});
