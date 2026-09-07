export type Metric = 'HeartRate' | 'Power' | 'Speed' | 'Altitude' | 'Cadence';

export const metricClass: Record<Metric, string> = {
	HeartRate: 'heart-rate',
	Power: 'power',
	Speed: 'speed',
	Altitude: 'elevation',
	Cadence: 'cadence'
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
	return 'Cadence';
};

export const formatMetricValue = (
	value: number,
	metric: Metric,
	unit: string
): { value: string; unit: string } => {
	if (['HeartRate', 'Power', 'Cadence'].includes(metric)) {
		return { value: value.toFixed(0), unit };
	} else if (metric === 'Altitude' && unit === 'km') {
		return { value: (value * 1000).toFixed(0), unit: 'm' };
	} else {
		return { value: value.toFixed(2), unit };
	}
};
