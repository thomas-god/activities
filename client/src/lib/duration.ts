import dayjs from 'dayjs';
import duration from 'dayjs/plugin/duration';
import relativeTime from 'dayjs/plugin/relativeTime';
import utc from 'dayjs/plugin/utc';
import timezone from 'dayjs/plugin/timezone';

dayjs.extend(duration);
dayjs.extend(relativeTime);
dayjs.extend(utc);
dayjs.extend(timezone);

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
