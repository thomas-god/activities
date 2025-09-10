import { expect, it } from 'vitest';
import { formatDuration, formatRelativeDuration } from './duration';
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
