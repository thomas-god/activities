import { expect, it } from 'vitest';
import { paceToString, speedToPace } from './speed';

it('Should convert a speed from km/h to pace in min/km', () => {
	expect(speedToPace(12)).toEqual(5);
	expect(speedToPace(15)).toEqual(4);
});

it('Should convert a pace to its string representation', () => {
	expect(paceToString(5)).toEqual('5:00/km');
	expect(paceToString(4.5)).toEqual('4:30/km');
	expect(paceToString(10)).toEqual('10:00/km');
	expect(paceToString(5.56)).toEqual('5:34/km');
	expect(paceToString(5.56, true)).toEqual('05:34/km');
	expect(paceToString(10, true)).toEqual('10:00/km');
});
