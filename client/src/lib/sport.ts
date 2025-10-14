export const SportCategories = [
	'Running',
	'Cycling',
	'Swimming',
	'Walking',
	'Rowing',
	'WaterSports',
	'Ski',
	'Cardio',
	'Climbing',
	'TeamSports',
	'Racket'
] as const;

export type SportCategory = (typeof SportCategories)[number];

export const sportCategoryIcons: Record<SportCategory, string> = {
	Climbing: '🧗',
	Running: '🏃',
	Cycling: '🚴',
	Cardio: '💪',
	Racket: '🎾',
	Rowing: '🚣',
	Ski: '🎿',
	Swimming: '🏊',
	TeamSports: '🥅',
	Walking: '🚶',
	WaterSports: '🌊'
};

export const getSportCategoryIcon = (category: SportCategory | null): string => {
	if (category === null) {
		return '🔥';
	}
	return sportCategoryIcons[category];
};
