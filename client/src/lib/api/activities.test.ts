import { describe, it, expect } from 'vitest';
import { ActivitySchema, ActivityWithTimeseriesSchema } from './activities';

describe('ActivitySchema', () => {
	it('parses an activity without training context', () => {
		const result = ActivitySchema.parse({
			id: 'activity_id',
			sport: 'IndoorCycling',
			sport_category: 'Cycling',
			name: null,
			start_time: '2025-09-03T00:00:00Z',
			rpe: null,
			workout_type: null,
			feedback: null,
			nutrition: null,
			metrics: { Duration: { value: 1200, unit: 's' } }
		});

		expect(result.id).toBe('activity_id');
		expect(result.sport).toBe('IndoorCycling');
	});
});

describe('ActivityWithTimeseriesSchema', () => {
	const payload = {
		id: 'activity_id',
		sport: 'IndoorCycling',
		sport_category: 'Cycling',
		name: null,
		start_time: '2025-09-03T00:00:00Z',
		rpe: null,
		workout_type: null,
		feedback: null,
		nutrition: null,
		metrics: { Duration: { value: 1200, unit: 's' } },
		training_context: {
			weight: 70.0,
			best_duration_12w_curves: [
				{
					curve_type: 'Power',
					unit: 'W',
					values: [260, 250, 240, 230, 220, 210, 200, 190, 180, null, null, null]
				}
			]
		},
		timeseries: {
			time: [0, 1, 2],
			active_time: [0, 1, 2],
			metrics: {
				Power: { unit: 'W', values: [120, null, 130] }
			},
			laps: []
		},
		duration_curves: [
			{
				curve_type: 'Power',
				unit: 'W',
				values: [250, 240, 230, 220, 210, 200, 190, 180, null, null, null, null]
			}
		]
	};

	it('parses the training context', () => {
		const result = ActivityWithTimeseriesSchema.parse(payload);

		expect(result.training_context.weight).toBe(70.0);
		expect(result.training_context.best_duration_12w_curves).toHaveLength(1);
		expect(result.training_context.best_duration_12w_curves[0].curve_type).toBe('Power');
		expect(result.training_context.best_duration_12w_curves[0].unit).toBe('W');
		expect(result.training_context.best_duration_12w_curves[0].values[0]).toBe(260);
	});

	it('parses an empty training context best duration curve list', () => {
		const result = ActivityWithTimeseriesSchema.parse({
			...payload,
			training_context: { weight: 70.0, best_duration_12w_curves: [] }
		});

		expect(result.training_context.best_duration_12w_curves).toHaveLength(0);
	});

	it('parses a null training context weight', () => {
		const result = ActivityWithTimeseriesSchema.parse({
			...payload,
			training_context: { weight: null, best_duration_12w_curves: [] }
		});

		expect(result.training_context.weight).toBeNull();
	});

	it('rejects a missing training context', () => {
		/* eslint-disable @typescript-eslint/no-unused-vars */
		const { training_context, ...without_training_context } = payload;

		expect(() => ActivityWithTimeseriesSchema.parse(without_training_context)).toThrow();
	});

	it('rejects a non numeric weight', () => {
		expect(() =>
			ActivityWithTimeseriesSchema.parse({
				...payload,
				training_context: { weight: '70', best_duration_12w_curves: [] }
			})
		).toThrow();
	});

	it('rejects a non numeric best duration curve value', () => {
		expect(() =>
			ActivityWithTimeseriesSchema.parse({
				...payload,
				training_context: {
					weight: 70.0,
					best_duration_12w_curves: [
						{
							curve_type: 'Power',
							unit: 'W',
							values: ['high', null, null, null, null, null, null, null, null, null, null, null]
						}
					]
				}
			})
		).toThrow();
	});
});
