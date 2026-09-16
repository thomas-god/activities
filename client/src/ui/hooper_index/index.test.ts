import { describe, expect, it } from 'vitest';

import { emptyHooperIndex, hooperMeasures } from './index';

describe('emptyHooperIndex', () => {
	it('has every measure unset', () => {
		const values = emptyHooperIndex();

		for (const measure of hooperMeasures) {
			expect(values[measure]).toBeNull();
		}
	});
});
