import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { deleteHooperIndex, fetchHooperIndex, saveHooperIndex, type HooperIndex } from '$lib/api';
import HooperIndexForm from './HooperIndexForm.svelte';
import { emptyHooperIndex, hooperMeasures } from './index';
import dayjs from 'dayjs';

vi.mock('$lib/api', () => ({
	fetchHooperIndex: vi.fn(),
	saveHooperIndex: vi.fn(),
	deleteHooperIndex: vi.fn()
}));

const mockedFetch = vi.mocked(fetchHooperIndex);
const mockedSave = vi.mocked(saveHooperIndex);
const mockedDelete = vi.mocked(deleteHooperIndex);

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
	mockedDelete.mockReset();
	mockedSave.mockResolvedValue(true);
	mockedDelete.mockResolvedValue(true);
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

	it('disables the save button while a save request is in flight', async () => {
		const user = userEvent.setup();
		let resolveSave!: (value: boolean) => void;
		mockedSave.mockImplementation(() => new Promise<boolean>((resolve) => (resolveSave = resolve)));

		await renderForm();

		await user.click(screen.getByRole('button', { name: 'Save' }));

		expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
		expect(screen.getByRole('button', { name: 'Delete' })).toBeEnabled();

		resolveSave(true);

		await waitFor(() => expect(screen.getByRole('button', { name: 'Save' })).toBeEnabled());
	});

	it('disables the delete button while a delete request is in flight', async () => {
		const user = userEvent.setup();
		let resolveDelete!: (value: boolean) => void;
		mockedDelete.mockImplementation(
			() => new Promise<boolean>((resolve) => (resolveDelete = resolve))
		);

		await renderForm();

		await user.click(screen.getByRole('button', { name: 'Delete' }));

		expect(screen.getByRole('button', { name: 'Delete' })).toBeDisabled();
		expect(screen.getByRole('button', { name: 'Save' })).toBeEnabled();

		resolveDelete(true);

		await waitFor(() => expect(screen.getByRole('button', { name: 'Delete' })).toBeEnabled());
	});

	it('saves the loaded values for the current date', async () => {
		const user = userEvent.setup();
		const values = { ...emptyHooperIndex(), fatigue: 7, mood: 4 };

		await renderForm(values);

		await user.click(screen.getByRole('button', { name: 'Save' }));

		await waitFor(() => expect(mockedSave).toHaveBeenCalledWith(today(), { ...values }));
	});

	it('deletes the index for the current date, reloads and clears the measures', async () => {
		const user = userEvent.setup();

		mockedFetch
			.mockResolvedValueOnce({ ...emptyHooperIndex(), fatigue: 7 })
			.mockResolvedValue(emptyHooperIndex());

		render(HooperIndexForm);
		await screen.findByLabelText('fatigue');

		await user.click(screen.getByRole('button', { name: 'Delete' }));

		await waitFor(() => expect(mockedDelete).toHaveBeenCalledWith(today()));
		await waitFor(() => expect(displayedValue('fatigue')).toBe('Not set'));
	});
});
