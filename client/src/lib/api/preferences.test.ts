import { describe, it, expect } from 'vitest';
import { PreferenceSchema } from './preferences';

describe('PreferenceResponse Schema', () => {
	it('should parse favorite_metric preference', () => {
		const data = {
			key: 'favorite_metric',
			value: 'metric-123'
		};

		const result = PreferenceSchema.parse(data);
		expect(result.key).toBe('favorite_metric');
		expect(result.value).toBe('metric-123');
	});

	it('should reject invalid preference key', () => {
		const data = {
			key: 'invalid_key',
			value: 'some-value'
		};

		expect(() => PreferenceSchema.parse(data)).toThrow();
	});

	it('should reject missing value', () => {
		const data = {
			key: 'favorite_metric'
		};

		expect(() => PreferenceSchema.parse(data)).toThrow();
	});
});
