import { describe, it, expect } from 'vitest';
import { extractNoGroupValues, type MetricsListItemGrouped } from './training';

describe('extractNoGroupValues', () => {
	it('should extract no_group values from grouped metric', () => {
		const groupedMetric: MetricsListItemGrouped = {
			id: 'metric-1',
			metric: 'distance',
			unit: 'km',
			granularity: 'Daily',
			aggregate: 'Sum',
			values: {
				no_group: {
					'2025-09-24': 100.0,
					'2025-09-25': 150.0,
					'2025-09-26': 0.0
				},
				Cycling: {
					'2025-09-24': 50.0,
					'2025-09-25': 75.0
				}
			}
		};

		const result = extractNoGroupValues(groupedMetric);

		expect(result).toEqual({
			id: 'metric-1',
			metric: 'distance',
			unit: 'km',
			granularity: 'Daily',
			aggregate: 'Sum',
			values: {
				'2025-09-24': 100.0,
				'2025-09-25': 150.0,
				'2025-09-26': 0.0
			}
		});
	});

	it('should handle missing no_group with empty object', () => {
		const groupedMetric: MetricsListItemGrouped = {
			id: 'metric-2',
			metric: 'duration',
			unit: 's',
			granularity: 'Weekly',
			aggregate: 'Sum',
			values: {
				Running: {
					'2025-W39': 3600.0
				}
			}
		};

		const result = extractNoGroupValues(groupedMetric);

		expect(result).toEqual({
			id: 'metric-2',
			metric: 'duration',
			unit: 's',
			granularity: 'Weekly',
			aggregate: 'Sum',
			values: {}
		});
	});

	it('should preserve sports array if present', () => {
		const groupedMetric: MetricsListItemGrouped = {
			id: 'metric-3',
			metric: 'elevation',
			unit: 'm',
			granularity: 'Daily',
			aggregate: 'Sum',
			sports: ['Cycling', 'Running'],
			values: {
				no_group: {
					'2025-09-24': 500.0
				}
			}
		};

		const result = extractNoGroupValues(groupedMetric);

		expect(result.sports).toEqual(['Cycling', 'Running']);
	});
});
