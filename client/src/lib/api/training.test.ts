import { describe, it, expect } from 'vitest';
import { CreateHooperIndexSchema, HooperIndexSchema, UpdateHooperIndexSchema } from './training';

describe('HooperIndexSchema', () => {
	it('parses all measures', () => {
		const result = HooperIndexSchema.parse({
			fatigue: 1,
			sleep: 2,
			pain: 3,
			stress: 4,
			mood: 5
		});

		expect(result).toEqual({ fatigue: 1, sleep: 2, pain: 3, stress: 4, mood: 5 });
	});

	it('parses null measures', () => {
		const result = HooperIndexSchema.parse({
			fatigue: null,
			sleep: null,
			pain: null,
			stress: null,
			mood: null
		});

		expect(result.fatigue).toBeNull();
		expect(result.mood).toBeNull();
	});

	it('rejects out of range values', () => {
		for (const value of [0, 11, -1]) {
			expect(() =>
				HooperIndexSchema.parse({
					fatigue: value,
					sleep: null,
					pain: null,
					stress: null,
					mood: null
				})
			).toThrow();
		}
	});

	it('rejects non integer values', () => {
		expect(() =>
			HooperIndexSchema.parse({
				fatigue: 3.5,
				sleep: null,
				pain: null,
				stress: null,
				mood: null
			})
		).toThrow();
	});
});

describe('CreateHooperIndexSchema', () => {
	it('parses a date together with measures', () => {
		const result = CreateHooperIndexSchema.parse({
			date: '2026-01-15',
			fatigue: 5,
			sleep: null,
			pain: null,
			stress: null,
			mood: null
		});

		expect(result.date).toBe('2026-01-15');
		expect(result.fatigue).toBe(5);
	});

	it('requires a date', () => {
		expect(() =>
			CreateHooperIndexSchema.parse({
				fatigue: 5,
				sleep: null,
				pain: null,
				stress: null,
				mood: null
			})
		).toThrow();
	});
});

describe('UpdateHooperIndexSchema', () => {
	it('allows absent, null and value fields', () => {
		const result = UpdateHooperIndexSchema.parse({ fatigue: 5, sleep: null });

		expect(result.fatigue).toBe(5);
		expect(result.sleep).toBeNull();
		expect(result.pain).toBeUndefined();
	});

	it('accepts an empty patch', () => {
		expect(UpdateHooperIndexSchema.parse({})).toEqual({});
	});

	it('rejects out of range values', () => {
		expect(() => UpdateHooperIndexSchema.parse({ mood: 0 })).toThrow();
	});
});
