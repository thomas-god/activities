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
		training_context: { weight: 70.0 },
		timeseries: {
			time: [0, 1, 2],
			active_time: [0, 1, 2],
			metrics: {
				Power: { unit: 'W', values: [120, null, 130] }
			},
			laps: []
		}
	};

	it('parses the training context', () => {
		const result = ActivityWithTimeseriesSchema.parse(payload);

		expect(result.training_context.weight).toBe(70.0);
	});

	it('parses a null training context weight', () => {
		const result = ActivityWithTimeseriesSchema.parse({
			...payload,
			training_context: { weight: null }
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
				training_context: { weight: '70' }
			})
		).toThrow();
	});
});
