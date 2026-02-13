import { expect, it } from 'vitest';
import { paceInSecondToString, paceToString, speedToPace } from './speed';

it('Should convert a speed from km/h to pace in min', () => {
	expect(speedToPace(12)).toEqual(5);
	expect(speedToPace(15)).toEqual(4);
});

it('Should convert a pace to its string representation', () => {
	expect(paceToString(5)).toEqual('5:00');
	expect(paceToString(4.5)).toEqual('4:30');
	expect(paceToString(10)).toEqual('10:00');
	expect(paceToString(5.56)).toEqual('5:34');
	expect(paceToString(5.56, true)).toEqual('05:34');
	expect(paceToString(10, true)).toEqual('10:00');
});

it('Should convert a pace in seconds to its string representation', () => {
	expect(paceInSecondToString(300)).toEqual('5:00');
	expect(paceInSecondToString(270)).toEqual('4:30');
	expect(paceInSecondToString(600)).toEqual('10:00');
	expect(paceInSecondToString(334)).toEqual('5:34');
	expect(paceInSecondToString(334, true)).toEqual('05:34');
	expect(paceInSecondToString(600, true)).toEqual('10:00');
});
