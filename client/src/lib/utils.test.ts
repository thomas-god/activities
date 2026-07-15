import { describe, expect, it } from 'vitest';
import { toTitleCase } from './utils';

describe('toTitleCase', () => {
	it('should convert snake_case to Title Case', () => {
		expect(toTitleCase('hello_world')).toBe('Hello World');
		expect(toTitleCase('my_variable_name')).toBe('My Variable Name');
		expect(toTitleCase('total_elapsed_time')).toBe('Total Elapsed Time');
		expect(toTitleCase('max_heart_rate')).toBe('Max Heart Rate');
	});

	it('should convert camelCase to Title Case', () => {
		expect(toTitleCase('helloWorld')).toBe('Hello World');
		expect(toTitleCase('myVariableName')).toBe('My Variable Name');
		expect(toTitleCase('totalElapsedTime')).toBe('Total Elapsed Time');
		expect(toTitleCase('maxHeartRate')).toBe('Max Heart Rate');
	});

	it('should convert PascalCase to Title Case', () => {
		expect(toTitleCase('HelloWorld')).toBe('Hello World');
		expect(toTitleCase('MyVariableName')).toBe('My Variable Name');
		expect(toTitleCase('TotalElapsedTime')).toBe('Total Elapsed Time');
	});

	it('should handle mixed snake_case and camelCase', () => {
		expect(toTitleCase('my_variableName')).toBe('My Variable Name');
		expect(toTitleCase('total_elapsedTime')).toBe('Total Elapsed Time');
	});

	it('should handle single word inputs', () => {
		expect(toTitleCase('hello')).toBe('Hello');
		expect(toTitleCase('world')).toBe('World');
		expect(toTitleCase('test')).toBe('Test');
	});

	it('should handle uppercase single word', () => {
		// Each capital letter is treated as a separate word
		expect(toTitleCase('HELLO')).toBe('H E L L O');
		expect(toTitleCase('WORLD')).toBe('W O R L D');
	});

	it('should handle already formatted Title Case', () => {
		// Existing spaces are preserved, capital letters add extra spaces
		expect(toTitleCase('Hello World')).toBe('Hello  World');
		expect(toTitleCase('My Variable Name')).toBe('My  Variable  Name');
	});

	it('should handle empty string', () => {
		expect(toTitleCase('')).toBe('');
	});

	it('should handle strings with multiple underscores', () => {
		// Multiple underscores create multiple spaces that get collapsed by split/join
		expect(toTitleCase('hello__world')).toBe('Hello  World');
		expect(toTitleCase('my___variable___name')).toBe('My   Variable   Name');
	});

	it('should handle strings with consecutive capital letters', () => {
		expect(toTitleCase('HTTPRequest')).toBe('H T T P Request');
		expect(toTitleCase('XMLParser')).toBe('X M L Parser');
		expect(toTitleCase('URLPath')).toBe('U R L Path');
	});

	it('should handle mixed case with numbers', () => {
		expect(toTitleCase('value1')).toBe('Value1');
		expect(toTitleCase('my_value_2')).toBe('My Value 2');
		expect(toTitleCase('test123Value')).toBe('Test123 Value');
	});

	it('should trim leading and trailing spaces', () => {
		expect(toTitleCase('  hello_world  ')).toBe('Hello World');
		expect(toTitleCase(' helloWorld ')).toBe('Hello World');
	});

	it('should handle strings with leading or trailing underscores', () => {
		expect(toTitleCase('_hello_world')).toBe('Hello World');
		expect(toTitleCase('hello_world_')).toBe('Hello World');
		expect(toTitleCase('_hello_world_')).toBe('Hello World');
	});

	it('should handle metric names commonly used in the app', () => {
		expect(toTitleCase('avg_speed')).toBe('Avg Speed');
		expect(toTitleCase('max_heart_rate')).toBe('Max Heart Rate');
		expect(toTitleCase('totalDistance')).toBe('Total Distance');
		expect(toTitleCase('elevationGain')).toBe('Elevation Gain');
	});
});
