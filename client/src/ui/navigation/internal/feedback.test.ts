import { describe, expect, it } from 'vitest';

import { emptyHooperIndex, hooperMeasureExtremes, hooperMeasures } from './feedback';

describe('emptyHooperIndex', () => {
	it('has every measure unset', () => {
		const values = emptyHooperIndex();

		for (const measure of hooperMeasures) {
			expect(values[measure]).toBeNull();
		}
	});
});

describe('hooperMeasureExtremes', () => {
	it('labels both ends of every measure', () => {
		for (const measure of hooperMeasures) {
			expect(hooperMeasureExtremes[measure].low).not.toHaveLength(0);
			expect(hooperMeasureExtremes[measure].high).not.toHaveLength(0);
		}
	});

	it('uses opposite meanings at each end of a measure', () => {
		expect(hooperMeasureExtremes.fatigue).toEqual({ low: 'Fresh', high: 'Exhausted' });
		expect(hooperMeasureExtremes.sleep).toEqual({ low: 'Terrible', high: 'Excellent' });
	});
});
