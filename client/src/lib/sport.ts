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

export const sportCategoryDisplay = (category: SportCategory): string => {
	switch (category) {
		case 'Running':
			return 'Running';
		case 'Walking':
			return 'Walking';
		case 'Cycling':
			return 'Cycling';
		case 'Rowing':
			return 'Rowing';
		case 'Swimming':
			return 'Swimming';
		case 'WaterSports':
			return 'Water sports';
		case 'Ski':
			return 'Skiing';
		case 'Cardio':
			return 'Gym & Fitness';
		case 'Climbing':
			return 'Climbing';
		case 'Racket':
			return 'Racket';
		case 'TeamSports':
			return 'Team sports';
	}
};

export const sportDisplay = (sport: Sport): string => {
	switch (sport) {
		case 'Running':
			return 'Running';
		case 'TrailRunning':
			return 'Trail Running';
		case 'IndoorRunning':
			return 'Indoor Running';
		case 'TrackRunning':
			return 'Track Running';
		case 'Walking':
			return 'Walking';
		case 'Hiking':
			return 'Hiking';
		case 'Mountaineering':
			return 'Mountaineering';
		case 'IndoorWalking':
			return 'Indoor Walking';
		case 'Snowshoeing':
			return 'Snowshoeing';
		case 'Cycling':
			return 'Cycling';
		case 'IndoorCycling':
			return 'Indoor Cycling';
		case 'MountainBiking':
			return 'Mountain Biking';
		case 'Cyclocross':
			return 'Cyclocross';
		case 'TrackCycling':
			return 'Track Cycling';
		case 'EBiking':
			return 'E-Biking';
		case 'GravelCycling':
			return 'Gravel Cycling';
		case 'Rowing':
			return 'Rowing';
		case 'IndoorRowing':
			return 'Indoor Rowing';
		case 'Swimming':
			return 'Swimming';
		case 'OpenWaterSwimming':
			return 'Open Water Swimming';
		case 'StandUpPaddleboarding':
			return 'Stand Up Paddleboarding';
		case 'Surfing':
			return 'Surfing';
		case 'Wakeboarding':
			return 'Wakeboarding';
		case 'WaterSkiing':
			return 'Water Skiing';
		case 'Windsurfing':
			return 'Windsurfing';
		case 'Kitesurfing':
			return 'Kitesurfing';
		case 'Wakesurfing':
			return 'Wakesurfing';
		case 'Sailing':
			return 'Sailing';
		case 'Snorkeling':
			return 'Snorkeling';
		case 'Whitewater':
			return 'Whitewater Sports';
		case 'Paddling':
			return 'Paddling';
		case 'Kayaking':
			return 'Kayaking';
		case 'Rafting':
			return 'Rafting';
		case 'AlpineSki':
			return 'Alpine Skiing';
		case 'CrossCountrySkiing':
			return 'Cross Country Skiing';
		case 'Snowboarding':
			return 'Snowboarding';
		case 'InlineSkating':
			return 'Inline Skating';
		case 'Hiit':
			return 'HIIT';
		case 'CardioTraining':
			return 'Cardio Training';
		case 'StrengthTraining':
			return 'Strength Training';
		case 'Yoga':
			return 'Yoga';
		case 'Pilates':
			return 'Pilates';
		case 'Climbing':
			return 'Climbing';
		case 'IndoorClimbing':
			return 'Indoor Climbing';
		case 'Bouldering':
			return 'Bouldering';
		case 'Soccer':
			return 'Soccer';
		case 'Baseball':
			return 'Baseball';
		case 'Cricket':
			return 'Cricket';
		case 'AmericanFootball':
			return 'American Football';
		case 'Basketball':
			return 'Basketball';
		case 'Rugby':
			return 'Rugby';
		case 'Hockey':
			return 'Hockey';
		case 'Lacrosse':
			return 'Lacrosse';
		case 'Volleyball':
			return 'Volleyball';
		case 'Racket':
			return 'Racket Sports';
		case 'Tennis':
			return 'Tennis';
		case 'Pickleball':
			return 'Pickleball';
		case 'Padel':
			return 'Padel';
		case 'Squash':
			return 'Squash';
		case 'Badminton':
			return 'Badminton';
		case 'Racquetball':
			return 'Racquetball';
		case 'TableTennis':
			return 'Table Tennis';
		case 'Boxing':
			return 'Boxing';
		case 'MixedMartialArts':
			return 'Mixed Martial Arts';
		case 'Golf':
			return 'Golf';
		case 'Other':
			return 'Other';
	}
};
