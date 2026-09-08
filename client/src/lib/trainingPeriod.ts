import type { ActivityList } from '$lib/api/activities';
import { dayjs } from '$lib/duration';

export type PeriodActivitiesSummary = {
	count: number;
	duration: number;
	distance: number;
	elevation: number;
};

export const periodActivitiesSummary = (activities: ActivityList): PeriodActivitiesSummary => {
	const summary = { count: activities.length, duration: 0, distance: 0, elevation: 0 };

	for (const activity of activities) {
		summary.duration += activityMetricValue(activity.metrics, 'ActiveDuration');
		summary.distance += activityMetricValue(activity.metrics, 'Distance');
		summary.elevation += activityMetricValue(activity.metrics, 'Elevation');
	}

	return summary;
};

const activityMetricValue = (metrics: Record<string, { value: number }>, key: string): number => {
	const metric = metrics[key];
	if (metric === undefined) {
		return 0;
	}
	return metric.value;
};

export const formatPeriodDuration = (start: string, end: string | null): string => {
	const startDate = dayjs(start);
	const endDate = end ? dayjs(end) : dayjs();
	// Add 1 to include the last day (end date is inclusive)
	const days = endDate.diff(startDate, 'day') + 1;

	if (days === 1) return '1 day';
	if (days < 7) return `${days} days`;

	const weeks = Math.floor(days / 7);
	const remainingDays = days % 7;

	if (remainingDays === 0) {
		return weeks === 1 ? '1 week' : `${weeks} weeks`;
	}

	const weeksText = weeks === 1 ? '1 week' : `${weeks} weeks`;
	const daysText = remainingDays === 1 ? '1 day' : `${remainingDays} days`;
	return `${weeksText} ${daysText}`;
};

export const formatDistance = (meters: number): string => {
	if (meters === 0) return '0 km';
	const km = meters / 1000;
	return `${Math.round(km).toLocaleString('fr-fr')} km`;
};

export const formatElevation = (meters: number): string => {
	if (meters === 0) return '0 m';
	return `${Math.round(meters).toLocaleString('fr-fr')} m`;
};
