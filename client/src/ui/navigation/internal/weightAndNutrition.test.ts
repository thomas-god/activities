import { describe, expect, it } from 'vitest';

import {
	emptyWeightAndNutrition,
	weightAndNutritionCategories,
	weightAndNutritionMeasures
} from './weightAndNutrition';

describe('emptyWeightAndNutrition', () => {
	it('has every measure unset', () => {
		const values = emptyWeightAndNutrition();

		for (const measure of weightAndNutritionMeasures) {
			expect(values[measure]).toBeNull();
		}
	});
});

describe('weightAndNutritionCategories', () => {
	it('exposes each measure exactly once between the main and sub fields', () => {
		const exposed = weightAndNutritionCategories.flatMap((category) => [
			category.main,
			...category.subs
		]);

		expect([...exposed].sort()).toEqual([...weightAndNutritionMeasures].sort());
	});

	it('groups the expected main and sub fields', () => {
		const groups = Object.fromEntries(
			weightAndNutritionCategories.map((category) => [
				category.key,
				{ main: category.main, subs: [...category.subs] }
			])
		);

		expect(groups).toEqual({
			weight: { main: 'weight', subs: ['fat', 'muscle'] },
			nutrition: { main: 'calories', subs: ['lipid', 'carbs', 'protein'] },
			hydration: { main: 'water', subs: ['alcohol'] }
		});
	});
});
