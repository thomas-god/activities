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
			best_duration_curves: [
				{
					kind: 'twelve_week',
					label: null,
					start: '2025-06-11',
					end: '2025-09-03',
					curves: [
						{
							curve_type: 'Power',
							unit: 'W',
							values: [260, 250, 240, 230, 220, 210, 200, 190, 180, null, null, null]
						}
					]
				},
				{
					kind: 'training_period',
					label: 'Base training',
					start: '2025-07-07',
					end: '2025-09-03',
					curves: [
						{
							curve_type: 'Power',
							unit: 'W',
							values: [270, 260, 250, 240, 230, 220, 210, 200, 190, null, null, null]
						}
					]
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

	it('parses the training context references', () => {
		const result = ActivityWithTimeseriesSchema.parse(payload);

		expect(result.training_context.weight).toBe(70.0);
		expect(result.training_context.best_duration_curves).toHaveLength(2);

		const [twelveWeek, period] = result.training_context.best_duration_curves;
		expect(twelveWeek.kind).toBe('twelve_week');
		expect(twelveWeek.label).toBeNull();
		expect(twelveWeek.curves[0].curve_type).toBe('Power');
		expect(twelveWeek.curves[0].unit).toBe('W');
		expect(twelveWeek.curves[0].values[0]).toBe(260);

		expect(period.kind).toBe('training_period');
		expect(period.label).toBe('Base training');
		expect(period.start).toBe('2025-07-07');
		expect(period.end).toBe('2025-09-03');
	});

	it('parses an empty training context best duration curve list', () => {
		const result = ActivityWithTimeseriesSchema.parse({
			...payload,
			training_context: { weight: 70.0, best_duration_curves: [] }
		});

		expect(result.training_context.best_duration_curves).toHaveLength(0);
	});

	it('parses a null training context weight', () => {
		const result = ActivityWithTimeseriesSchema.parse({
			...payload,
			training_context: { weight: null, best_duration_curves: [] }
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
				training_context: { weight: '70', best_duration_curves: [] }
			})
		).toThrow();
	});

	it('rejects an unknown reference kind', () => {
		expect(() =>
			ActivityWithTimeseriesSchema.parse({
				...payload,
				training_context: {
					weight: 70.0,
					best_duration_curves: [
						{
							kind: 'all_time',
							label: null,
							start: '2025-06-11',
							end: '2025-09-03',
							curves: []
						}
					]
				}
			})
		).toThrow();
	});

	it('rejects a non numeric best duration curve value', () => {
		expect(() =>
			ActivityWithTimeseriesSchema.parse({
				...payload,
				training_context: {
					weight: 70.0,
					best_duration_curves: [
						{
							kind: 'twelve_week',
							label: null,
							start: '2025-06-11',
							end: '2025-09-03',
							curves: [
								{
									curve_type: 'Power',
									unit: 'W',
									values: ['high', null, null, null, null, null, null, null, null, null, null, null]
								}
							]
						}
					]
				}
			})
		).toThrow();
	});
});
