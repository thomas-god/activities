import dayjs from 'dayjs';
import duration from 'dayjs/plugin/duration';
import relativeTime from 'dayjs/plugin/relativeTime';
import utc from 'dayjs/plugin/utc';
import timezone from 'dayjs/plugin/timezone';
import localizedFormat from 'dayjs/plugin/localizedFormat';
import isoWeek from 'dayjs/plugin/isoWeek';

dayjs.extend(duration);
dayjs.extend(relativeTime);
dayjs.extend(utc);
dayjs.extend(timezone);
dayjs.extend(localizedFormat);
dayjs.extend(isoWeek);

export { dayjs };

const ONE_MINUTE_IN_SECONDS = 60;
const ONE_HOUR_IN_SECONDS = ONE_MINUTE_IN_SECONDS * 60;
const ONE_DAY_IN_SECONDS = 24 * ONE_HOUR_IN_SECONDS;

export const formatDuration = (time: number): string => {
	let remaining = Math.floor(time);

	const days = Math.floor(remaining / ONE_DAY_IN_SECONDS);
	remaining = remaining - days * ONE_DAY_IN_SECONDS;

	const hours = Math.floor(remaining / ONE_HOUR_IN_SECONDS);
	remaining = remaining - hours * ONE_HOUR_IN_SECONDS;

	const minutes = Math.floor(remaining / ONE_MINUTE_IN_SECONDS);
	const seconds = remaining - minutes * ONE_MINUTE_IN_SECONDS;

	if (days === 0 && hours === 0) {
		return `${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`;
	}

	if (days === 0) {
		return `${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`;
	}

	return `${days.toString()}d:${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`;
};

export const formatDurationCompactWithUnits = (time: number): string => {
	let remaining = Math.floor(time);

	const days = Math.floor(remaining / ONE_DAY_IN_SECONDS);
	remaining = remaining - days * ONE_DAY_IN_SECONDS;

	const hours = Math.floor(remaining / ONE_HOUR_IN_SECONDS);
	remaining = remaining - hours * ONE_HOUR_IN_SECONDS;

	const minutes = Math.floor(remaining / ONE_MINUTE_IN_SECONDS);
	const seconds = remaining - minutes * ONE_MINUTE_IN_SECONDS;

	if (time === 0) {
		return '';
	}

	if (days === 0 && hours === 0) {
		return `${minutes.toString().padStart(2, '0')}m`;
	}

	if (days === 0) {
		return `${hours.toString()}h${minutes.toString().padStart(2, '0')}m`;
	}

	return `${days.toString()}d${hours.toString().padStart(2, '0')}h`;
};

export const formatRelativeDuration = (value: dayjs.Dayjs, reference: dayjs.Dayjs): string => {
	return dayjs.duration(value.diff(reference)).humanize(true);
};

export const formatDateTime = (
	value: string,
	timezone = 'Europe/Paris',
	format = 'DD-MM-YYYY HH:mm:ss'
): string => {
	return dayjs(value).tz(timezone).format(format);
};

export const localiseDate = (value: number | string): string => {
	return dayjs(value).format('ll');
};

export const localiseDateTime = (value: number | string): string => {
	return dayjs(value).format('llll');
};

export const formatWeekInterval = (start: number | string): string => {
	const startDate = dayjs(start).startOf('isoWeek');
	const endDate = startDate.endOf('isoWeek');

	if (startDate.month() === endDate.month()) {
		return `${startDate.format('MMM D')}-${endDate.format('D')}`;
	}

	return `${startDate.format('MMM D')}-${endDate.format('MMM D')}`;
};
