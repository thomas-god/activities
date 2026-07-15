import { describe, expect, it } from 'vitest';
import {
	formatDuration,
	formatRelativeDuration,
	formatDateTime,
	formatWeekInterval,
	formatDurationHoursMinutes,
	formatDurationCompactWithUnits
} from './duration';
import dayjs from 'dayjs';

it('Should format a number of seconds into hh:mm:ss', () => {
	expect(formatDuration(0)).toEqual('00:00');
	expect(formatDuration(45)).toEqual('00:45');
	expect(formatDuration(59)).toEqual('00:59');
	expect(formatDuration(60)).toEqual('01:00');
	expect(formatDuration(61)).toEqual('01:01');
	expect(formatDuration(70)).toEqual('01:10');

	expect(formatDuration(3599)).toEqual('59:59');
	expect(formatDuration(3600)).toEqual('01:00:00');
	expect(formatDuration(3601)).toEqual('01:00:01');

	expect(formatDuration(86399)).toEqual('23:59:59');
	expect(formatDuration(86400)).toEqual('1d:00:00:00');
	expect(formatDuration(86401)).toEqual('1d:00:00:01');
	expect(formatDuration(86400 * 10)).toEqual('10d:00:00:00');
});

it('Should format a date duration from a reference in local', () => {
	let reference = dayjs();

	expect(formatRelativeDuration(reference.subtract(10, 'second'), reference)).toEqual(
		'a few seconds ago'
	);
	expect(formatRelativeDuration(reference.subtract(10, 'minutes'), reference)).toEqual(
		'10 minutes ago'
	);
	expect(formatRelativeDuration(reference.subtract(1, 'hour'), reference)).toEqual('an hour ago');
	expect(formatRelativeDuration(reference.subtract(2, 'hour'), reference)).toEqual('2 hours ago');
	expect(formatRelativeDuration(reference.subtract(2, 'days'), reference)).toEqual('2 days ago');
	expect(
		formatRelativeDuration(reference.subtract(2, 'days').subtract(3, 'hour'), reference)
	).toEqual('2 days ago');
	expect(formatRelativeDuration(reference.subtract(7, 'days'), reference)).toEqual('7 days ago');
});

it('Should format a date string in default timezone and format', () => {
	const dateStr = '2023-09-12T15:30:45Z';
	expect(formatDateTime(dateStr)).toEqual('12-09-2023 17:30:45');
});

it('Should format a date string in a custom timezone', () => {
	const dateStr = '2023-09-12T15:30:45Z';
	expect(formatDateTime(dateStr, 'America/New_York')).toEqual('12-09-2023 11:30:45');
});

it('Should format a date string in a custom format', () => {
	const dateStr = '2023-09-12T15:30:45Z';
	expect(formatDateTime(dateStr, 'Europe/Paris', 'YYYY/MM/DD HH:mm')).toEqual('2023/09/12 17:30');
});

it('Should handle empty string gracefully', () => {
	expect(formatDateTime('')).toEqual(dayjs('').tz('Europe/Paris').format('DD-MM-YYYY HH:mm:ss'));
});

describe('Formating a week-based time interval', () => {
	it('Should return the first and last dates of the week', () => {
		let date = '2025-10-13'; // a monday

		expect(formatWeekInterval(date)).toEqual('Oct 13-19');
	});

	it('Should align to the start of the week', () => {
		let date = '2025-10-14'; // not a monday

		expect(formatWeekInterval(date)).toEqual('Oct 13-19');
	});

	it('Should handle week over two months', () => {
		let date = '2025-10-02'; // end of september and start of october

		expect(formatWeekInterval(date)).toEqual('Sep 29-Oct 5');
	});

	it('Should work when year changes', () => {
		let date = '2026-01-02'; // end of september and start of october

		expect(formatWeekInterval(date)).toEqual('Dec 29-Jan 4');
	});
});

describe('formatDurationHoursMinutes', () => {
	it('Should format duration', () => {
		expect(formatDurationHoursMinutes(1800)).toEqual('30m');
		expect(formatDurationHoursMinutes(1801)).toEqual('30m');
		expect(formatDurationHoursMinutes(3600)).toEqual('1h 00m');
		expect(formatDurationHoursMinutes(3600 * 10)).toEqual('10h 00m');
		expect(formatDurationHoursMinutes(3600 * 100)).toEqual('100h 00m');
	});
});

describe('formatDurationCompactWithUnits', () => {
	it('Should return 0m for zero seconds', () => {
		expect(formatDurationCompactWithUnits(0)).toEqual('0m');
	});

	it('Should format seconds as minutes (rounded down)', () => {
		// Less than a minute shows 0m
		expect(formatDurationCompactWithUnits(30)).toEqual('0m');
		expect(formatDurationCompactWithUnits(59)).toEqual('0m');
	});

	it('Should format minutes only (no hours)', () => {
		expect(formatDurationCompactWithUnits(60)).toEqual('1m');
		expect(formatDurationCompactWithUnits(120)).toEqual('2m');
		expect(formatDurationCompactWithUnits(300)).toEqual('5m');
		expect(formatDurationCompactWithUnits(1800)).toEqual('30m');
		expect(formatDurationCompactWithUnits(3599)).toEqual('59m');
	});

	it('Should format hours and minutes with zero-padded minutes', () => {
		expect(formatDurationCompactWithUnits(3600)).toEqual('1h00');
		expect(formatDurationCompactWithUnits(3660)).toEqual('1h01');
		expect(formatDurationCompactWithUnits(3660 + 9)).toEqual('1h01'); // Seconds ignored
		expect(formatDurationCompactWithUnits(5400)).toEqual('1h30');
		expect(formatDurationCompactWithUnits(7200)).toEqual('2h00');
		expect(formatDurationCompactWithUnits(36000)).toEqual('10h00');
		expect(formatDurationCompactWithUnits(86399)).toEqual('23h59');
	});

	it('Should format days and hours with zero-padded hours', () => {
		expect(formatDurationCompactWithUnits(86400)).toEqual('1d00h');
		expect(formatDurationCompactWithUnits(86400 + 3600)).toEqual('1d01h');
		expect(formatDurationCompactWithUnits(86400 + 3600 * 5)).toEqual('1d05h');
		expect(formatDurationCompactWithUnits(86400 + 3600 * 12)).toEqual('1d12h');
		expect(formatDurationCompactWithUnits(86400 * 2)).toEqual('2d00h');
		expect(formatDurationCompactWithUnits(86400 * 10)).toEqual('10d00h');
		expect(formatDurationCompactWithUnits(86400 * 10 + 3600 * 23)).toEqual('10d23h');
	});

	it('Should ignore seconds in all formats', () => {
		// Minutes format
		expect(formatDurationCompactWithUnits(61)).toEqual('1m');
		expect(formatDurationCompactWithUnits(119)).toEqual('1m');

		// Hours format
		expect(formatDurationCompactWithUnits(3661)).toEqual('1h01');
		expect(formatDurationCompactWithUnits(3719)).toEqual('1h01');

		// Days format
		expect(formatDurationCompactWithUnits(86400 + 59)).toEqual('1d00h');
		expect(formatDurationCompactWithUnits(86400 + 3600 + 59)).toEqual('1d01h');
	});

	it('Should handle decimal inputs by flooring', () => {
		expect(formatDurationCompactWithUnits(60.9)).toEqual('1m');
		expect(formatDurationCompactWithUnits(3600.9)).toEqual('1h00');
		expect(formatDurationCompactWithUnits(86400.9)).toEqual('1d00h');
	});
});
