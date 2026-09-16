import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { fetchHooperIndex, saveHooperIndex, type HooperIndex } from '$lib/api';
import HooperIndexForm from './HooperIndexForm.svelte';
import { emptyHooperIndex, hooperMeasures } from './index';
import dayjs from 'dayjs';

vi.mock('$lib/api', () => ({
	fetchHooperIndex: vi.fn(),
	saveHooperIndex: vi.fn()
}));

const mockedFetch = vi.mocked(fetchHooperIndex);
const mockedSave = vi.mocked(saveHooperIndex);

/** The date the form defaults to when it is rendered without any interaction. */
const today = () => dayjs().format('YYYY-MM-DD');

const slider = (measure: string): HTMLInputElement =>
	screen.getByLabelText(measure) as HTMLInputElement;

const displayedValue = (measure: string): string =>
	screen.getByTestId(`hooper-${measure}-value`).textContent?.trim() ?? '';

/** Renders the form and waits for the initial load to complete. */
const renderForm = async (values: HooperIndex = emptyHooperIndex()) => {
	mockedFetch.mockResolvedValue(values);
	const result = render(HooperIndexForm);
	await screen.findByLabelText('fatigue');
	return result;
};

beforeEach(() => {
	mockedFetch.mockReset();
	mockedSave.mockReset();
	mockedSave.mockResolvedValue(true);
});

afterEach(() => {
	cleanup();
});

describe('HooperIndexForm', () => {
	it('loads the index for the current date and renders one slider per measure', async () => {
		mockedFetch.mockResolvedValue({ ...emptyHooperIndex(), fatigue: 6 });

		render(HooperIndexForm);

		await screen.findByLabelText('fatigue');

		expect(mockedFetch).toHaveBeenCalledWith(today());
		expect(screen.getAllByRole('slider')).toHaveLength(hooperMeasures.length);
		expect(displayedValue('fatigue')).toBe('6');
		expect(slider('sleep')).toHaveClass('range-empty');
		expect(displayedValue('sleep')).toBe('Not set');
	});

	it('reloads the index when the date changes', async () => {
		await renderForm();

		await fireEvent.input(screen.getByLabelText('Date'), { target: { value: '2026-02-11' } });

		await waitFor(() => expect(mockedFetch).toHaveBeenCalledWith('2026-02-11'));
	});

	it('reports a load failure', async () => {
		mockedFetch.mockRejectedValue(new Error('boom'));

		render(HooperIndexForm);

		expect(await screen.findByText('Failed to load the values for this date.')).toBeInTheDocument();
	});

	it('sets a measure as soon as the slider is used', async () => {
		await renderForm();

		await fireEvent.input(slider('fatigue'), { target: { value: '8' } });

		expect(slider('fatigue').value).toBe('8');
		expect(slider('fatigue')).not.toHaveClass('range-empty');
		expect(displayedValue('fatigue')).toBe('8');
	});

	it('clears a single measure without touching the others', async () => {
		const user = userEvent.setup();
		await renderForm({ ...emptyHooperIndex(), fatigue: 7, mood: 4 });

		await user.click(screen.getByRole('button', { name: 'Clear fatigue' }));

		expect(slider('fatigue')).toHaveClass('range-empty');
		expect(displayedValue('fatigue')).toBe('Not set');
		expect(displayedValue('mood')).toBe('4');
	});

	it('clears every measure with the Clear button', async () => {
		const user = userEvent.setup();
		await renderForm({ ...emptyHooperIndex(), fatigue: 7, mood: 4 });

		await user.click(screen.getByRole('button', { name: 'Clear' }));

		expect(displayedValue('fatigue')).toBe('Not set');
		expect(displayedValue('mood')).toBe('Not set');
	});

	it('restores the loaded values with the Reset button', async () => {
		const user = userEvent.setup();
		await renderForm({ ...emptyHooperIndex(), fatigue: 7, mood: 4 });

		expect(screen.getByRole('button', { name: 'Reset' })).toBeDisabled();

		await fireEvent.input(slider('fatigue'), { target: { value: '9' } });
		await user.click(screen.getByRole('button', { name: 'Clear' }));

		expect(screen.getByRole('button', { name: 'Reset' })).toBeEnabled();

		await user.click(screen.getByRole('button', { name: 'Reset' }));

		expect(displayedValue('fatigue')).toBe('7');
		expect(displayedValue('mood')).toBe('4');
		expect(screen.getByRole('button', { name: 'Reset' })).toBeDisabled();
		expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
	});

	it('keeps the save button disabled until a measure changes', async () => {
		await renderForm({ ...emptyHooperIndex(), fatigue: 7 });

		expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();

		await fireEvent.input(slider('fatigue'), { target: { value: '8' } });
		expect(screen.getByRole('button', { name: 'Save' })).toBeEnabled();

		await fireEvent.input(slider('fatigue'), { target: { value: '7' } });
		expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
	});

	it('disables the save button while a save request is in flight', async () => {
		const user = userEvent.setup();
		let resolveSave!: (value: boolean) => void;
		mockedSave.mockImplementation(() => new Promise<boolean>((resolve) => (resolveSave = resolve)));

		await renderForm();

		await fireEvent.input(slider('fatigue'), { target: { value: '8' } });
		await user.click(screen.getByRole('button', { name: 'Save' }));

		expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();

		resolveSave(true);

		await waitFor(() => expect(mockedFetch).toHaveBeenCalledTimes(2));
		expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
	});

	it('saves the modified values for the current date', async () => {
		const user = userEvent.setup();

		await renderForm();

		await fireEvent.input(slider('fatigue'), { target: { value: '7' } });
		await fireEvent.input(slider('mood'), { target: { value: '4' } });

		await user.click(screen.getByRole('button', { name: 'Save' }));

		await waitFor(() =>
			expect(mockedSave).toHaveBeenCalledWith(today(), {
				...emptyHooperIndex(),
				fatigue: 7,
				mood: 4
			})
		);
	});
});
