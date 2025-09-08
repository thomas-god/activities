import { expect, describe, it } from 'vitest';
import { formatDuration } from './duration';

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
