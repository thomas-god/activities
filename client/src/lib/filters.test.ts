import { describe, expect, it } from 'vitest';
import type { Activity, ActivityList } from './api';
import { filterActivities } from './filters';

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
