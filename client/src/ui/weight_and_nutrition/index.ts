import type { WeightAndNutrition } from '$lib/api';

/** Every measure that makes up a weight and nutrition entry. */
export const weightAndNutritionMeasures = [
	'weight',
	'fat',
	'muscle',
	'bmi',
	'calories',
	'lipid',
	'carbs',
	'protein',
	'water',
	'alcohol'
] as const;

export type WeightAndNutritionMeasure = (typeof weightAndNutritionMeasures)[number];

/**
 * A logical group of measures. Each category exposes a single `main` measure that is always
 * visible, and a set of `subs` measures that can be collapsed to keep the form compact.
 */
export interface WeightAndNutritionCategory {
	key: 'weight' | 'nutrition' | 'hydration';
	label: string;
	main: WeightAndNutritionMeasure;
	subs: readonly WeightAndNutritionMeasure[];
}

export const weightAndNutritionCategories: readonly WeightAndNutritionCategory[] = [
	{ key: 'weight', label: 'Weight', main: 'weight', subs: ['fat', 'muscle', 'bmi'] },
	{ key: 'nutrition', label: 'Nutrition', main: 'calories', subs: ['lipid', 'carbs', 'protein'] },
	{ key: 'hydration', label: 'Hydration', main: 'water', subs: ['alcohol'] }
];

/** Human readable labels for each measure. */
export const weightAndNutritionLabels: Record<WeightAndNutritionMeasure, string> = {
	weight: 'Weight',
	fat: 'Fat',
	muscle: 'Muscle',
	bmi: 'BMI',
	calories: 'Calories',
	lipid: 'Lipid',
	carbs: 'Carbs',
	protein: 'Protein',
	water: 'Water',
	alcohol: 'Alcohol'
};

/** Display units for each measure, matching the API's `WeightAndNutritionSource::unit`. */
export const weightAndNutritionUnits: Record<WeightAndNutritionMeasure, string> = {
	weight: 'kg',
	fat: 'kg',
	muscle: 'kg',
	bmi: '',
	calories: 'kcal',
	lipid: 'g',
	carbs: 'g',
	protein: 'g',
	water: 'L',
	alcohol: 'u'
};

/** A weight and nutrition entry with every measure unset. */
export const emptyWeightAndNutrition = (): WeightAndNutrition => ({
	weight: null,
	fat: null,
	muscle: null,
	bmi: null,
	calories: null,
	lipid: null,
	carbs: null,
	protein: null,
	water: null,
	alcohol: null
});
