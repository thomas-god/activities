export const SportCategories = [
	'Running',
	'Cycling',
	'Cardio',
	'Walking',
	'Ski',
	'Swimming',
	'Rowing',
	'WaterSports',
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

export const getSportCategoryBgColorClass = (category: SportCategory | null): string => {
	if (category === 'Running') {
		return 'bg-running';
	}
	if (category === 'Cycling') {
		return 'bg-cycling';
	}
	return 'bg-other';
};

export const getSportCategory = (sport: Sport): SportCategory | null => {
	for (const [category, sports] of Object.entries(sportsPerCategory)) {
		if (sports.includes(sport)) {
			return category as SportCategory;
		}
	}

	return null;
};

export const sportsPerCategory: Record<SportCategory, Sport[]> = {
	Running: ['Running', 'TrailRunning', 'IndoorRunning', 'TrackRunning'],
	Walking: ['Walking', 'Hiking', 'Mountaineering', 'IndoorWalking', 'Snowshoeing'],
	Cycling: [
		'Cycling',
		'Cyclocross',
		'EBiking',
		'IndoorCycling',
		'MountainBiking',
		'GravelCycling',
		'TrackCycling'
	],
	Rowing: ['Rowing', 'IndoorRowing'],
	Swimming: ['Swimming', 'OpenWaterSwimming'],
	WaterSports: [
		'StandUpPaddleboarding',
		'Surfing',
		'Wakeboarding',
		'WaterSkiing',
		'Windsurfing',
		'Kitesurfing',
		'Wakesurfing',
		'Sailing',
		'Snorkeling',
		'Whitewater',
		'Paddling',
		'Kayaking',
		'Rafting'
	],
	Ski: ['AlpineSki', 'CrossCountrySkiing', 'Snowboarding'],
	Cardio: ['Hiit', 'CardioTraining', 'StrengthTraining', 'Yoga', 'Pilates'],
	Climbing: ['Climbing', 'IndoorClimbing', 'Bouldering'],
	Racket: [
		'Racket',
		'Tennis',
		'Pickleball',
		'Padel',
		'Squash',
		'Badminton',
		'Racquetball',
		'TableTennis'
	],
	TeamSports: [
		'Soccer',
		'Baseball',
		'Cricket',
		'AmericanFootball',
		'Basketball',
		'Rugby',
		'Hockey',
		'Lacrosse',
		'Volleyball'
	]
};

export const sportsWithoutCategory: Sport[] = [
	'Boxing',
	'MixedMartialArts',
	'Golf',
	'Other',
	'InlineSkating'
];

export const sports = [
	'Running',
	'TrailRunning',
	'IndoorRunning',
	'TrackRunning',

	'Walking',
	'Hiking',
	'Mountaineering',
	'IndoorWalking',
	'Snowshoeing',

	'Cycling',
	'IndoorCycling',
	'MountainBiking',
	'Cyclocross',
	'TrackCycling',
	'EBiking',
	'GravelCycling',

	'Rowing',
	'IndoorRowing',

	'Swimming',
	'OpenWaterSwimming',

	'StandUpPaddleboarding',
	'Surfing',
	'Wakeboarding',
	'WaterSkiing',
	'Windsurfing',
	'Kitesurfing',
	'Wakesurfing',
	'Sailing',
	'Snorkeling',

	'Whitewater',
	'Paddling',
	'Kayaking',
	'Rafting',

	'AlpineSki',
	'CrossCountrySkiing',
	'Snowboarding',

	'InlineSkating',

	'Hiit',
	'CardioTraining',
	'StrengthTraining',
	'Yoga',
	'Pilates',

	'Climbing',
	'IndoorClimbing',
	'Bouldering',

	'Soccer',
	'Baseball',
	'Cricket',
	'AmericanFootball',
	'Basketball',
	'Rugby',
	'Hockey',
	'Lacrosse',
	'Volleyball',

	'Racket',
	'Tennis',
	'Pickleball',
	'Padel',
	'Squash',
	'Badminton',
	'Racquetball',
	'TableTennis',

	'Boxing',
	'MixedMartialArts',
	'Golf',

	'Other'
] as const;

export type Sport = (typeof sports)[number];
