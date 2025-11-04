import { describe, it, expect } from 'vitest';
import { PreferenceResponseSchema } from './preferences';

describe('PreferenceResponse Schema', () => {
	it('should parse favorite_metric preference', () => {
		const data = {
			key: 'favorite_metric',
			value: 'metric-123'
		};

		const result = PreferenceResponseSchema.parse(data);
		expect(result.key).toBe('favorite_metric');
		expect(result.value).toBe('metric-123');
	});

	it('should reject invalid preference key', () => {
		const data = {
			key: 'invalid_key',
			value: 'some-value'
		};

		expect(() => PreferenceResponseSchema.parse(data)).toThrow();
	});

	it('should reject missing value', () => {
		const data = {
			key: 'favorite_metric'
		};

		expect(() => PreferenceResponseSchema.parse(data)).toThrow();
	});
});
