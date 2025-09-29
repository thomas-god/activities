export type Metric = 'HeartRate' | 'Power' | 'Speed' | 'Altitude' | 'Other';

export const colors: Record<Metric, string> = {
	HeartRate: 'color-heart-rate-chart',
	Power: 'color-power-chart',
	Speed: 'color-speed-chart',
	Altitude: 'color-elevation-chart',
	Other: 'color-other-chart'
};

export const strokeColors: Record<Metric, string> = {
	HeartRate: 'stroke-heart-rate-chart',
	Power: 'stroke-power-chart',
	Speed: 'stroke-speed-chart',
	Altitude: 'stroke-elevation-chart',
	Other: 'stroke-other-chart'
};

export const textColors: Record<Metric, string> = {
	HeartRate: 'text-heart-rate-chart',
	Power: 'text-power-chart',
	Speed: 'text-speed-chart',
	Altitude: 'text-elevation-chart',
	Other: 'text-other-chart'
};

export const matchMetric = (name: string): Metric => {
	if (name === 'HeartRate') {
		return 'HeartRate';
	} else if (name === 'Power') {
		return 'Power';
	} else if (name === 'Speed') {
		return 'Speed';
	} else if (name === 'Altitude') {
		return 'Altitude';
	}
	return 'Other';
};
