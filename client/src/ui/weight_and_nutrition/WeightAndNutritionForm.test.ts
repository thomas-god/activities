import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import dayjs from 'dayjs';

import { fetchWeightAndNutrition, saveWeightAndNutrition, type WeightAndNutrition } from '$lib/api';
import WeightAndNutritionForm from './WeightAndNutritionForm.svelte';
import { emptyWeightAndNutrition } from './index';

vi.mock('$lib/api', () => ({
	fetchWeightAndNutrition: vi.fn(),
	saveWeightAndNutrition: vi.fn()
}));

const mockedFetch = vi.mocked(fetchWeightAndNutrition);
const mockedSave = vi.mocked(saveWeightAndNutrition);

/** The date the form defaults to when it is rendered without any interaction. */
const today = () => dayjs().format('YYYY-MM-DD');

const field = (label: string): HTMLInputElement =>
	screen.getByLabelText(new RegExp(`^${label}`)) as HTMLInputElement;

const categoryDetails = (category: string): HTMLDetailsElement =>
	screen.getByTestId(`wn-${category}-details`) as HTMLDetailsElement;

/** Renders the form and waits for the initial load to complete. */
const renderForm = async (values: WeightAndNutrition = emptyWeightAndNutrition()) => {
	mockedFetch.mockResolvedValue(values);
	const result = render(WeightAndNutritionForm);
	await screen.findByLabelText(/^Weight/);
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

describe('WeightAndNutritionForm', () => {
	it('loads the values for the current date and renders the main fields', async () => {
		mockedFetch.mockResolvedValue({ ...emptyWeightAndNutrition(), weight: 70.5 });

		render(WeightAndNutritionForm);

		await screen.findByLabelText(/^Weight/);

		expect(mockedFetch).toHaveBeenCalledWith(today());
		expect(field('Weight').value).toBe('70.5');
		expect(field('Calories').value).toBe('');
		expect(field('Water').value).toBe('');
	});

	it('collapses every category sub fields behind a details section', async () => {
		await renderForm();

		expect(categoryDetails('weight').open).toBe(false);
		expect(categoryDetails('nutrition').open).toBe(false);
		expect(categoryDetails('hydration').open).toBe(false);
	});

	it('reloads the values when the date changes', async () => {
		await renderForm();

		await fireEvent.input(screen.getByLabelText('Date'), { target: { value: '2026-02-11' } });

		await waitFor(() => expect(mockedFetch).toHaveBeenCalledWith('2026-02-11'));
	});

	it('reports a load failure', async () => {
		mockedFetch.mockRejectedValue(new Error('boom'));

		render(WeightAndNutritionForm);

		expect(await screen.findByText('Failed to load the values for this date.')).toBeInTheDocument();
	});

	it('sets a main value when typing', async () => {
		await renderForm();

		await fireEvent.input(field('Weight'), { target: { value: '72.5' } });

		expect(field('Weight').value).toBe('72.5');
	});

	it('clears a single measure without touching the others', async () => {
		const user = userEvent.setup();
		await renderForm({ ...emptyWeightAndNutrition(), weight: 70, calories: 2000 });

		await user.click(screen.getByRole('button', { name: 'Clear Weight' }));

		expect(field('Weight').value).toBe('');
		expect(field('Calories').value).toBe('2000');
	});

	it('restores the loaded values with the Reset button', async () => {
		const user = userEvent.setup();
		await renderForm({ ...emptyWeightAndNutrition(), weight: 70, calories: 2000 });

		expect(screen.getByRole('button', { name: 'Reset' })).toBeDisabled();

		await fireEvent.input(field('Weight'), { target: { value: '75' } });

		await user.click(screen.getByRole('button', { name: 'Reset' }));

		expect(field('Weight').value).toBe('70');
		expect(field('Calories').value).toBe('2000');
		expect(screen.getByRole('button', { name: 'Reset' })).toBeDisabled();
		expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
	});

	it('keeps the save button disabled until a measure changes', async () => {
		await renderForm({ ...emptyWeightAndNutrition(), weight: 70 });

		expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();

		await fireEvent.input(field('Weight'), { target: { value: '71' } });
		expect(screen.getByRole('button', { name: 'Save' })).toBeEnabled();

		await fireEvent.input(field('Weight'), { target: { value: '70' } });
		expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
	});

	it('edits a sub field once the details section is expanded', async () => {
		const user = userEvent.setup();
		await renderForm();

		categoryDetails('weight').open = true;

		await fireEvent.input(field('Fat'), { target: { value: '15' } });

		expect(field('Fat').value).toBe('15');

		await user.click(screen.getByRole('button', { name: 'Save' }));

		await waitFor(() =>
			expect(mockedSave).toHaveBeenCalledWith(today(), {
				...emptyWeightAndNutrition(),
				fat: 15
			})
		);
	});

	it('disables the save button while a save request is in flight', async () => {
		const user = userEvent.setup();
		let resolveSave!: (value: boolean) => void;
		mockedSave.mockImplementation(() => new Promise<boolean>((resolve) => (resolveSave = resolve)));

		await renderForm();

		await fireEvent.input(field('Weight'), { target: { value: '75' } });
		await user.click(screen.getByRole('button', { name: 'Save' }));

		expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();

		resolveSave(true);

		await waitFor(() => expect(mockedFetch).toHaveBeenCalledTimes(2));
		expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
	});

	it('saves the modified values for the current date', async () => {
		const user = userEvent.setup();

		await renderForm();

		await fireEvent.input(field('Weight'), { target: { value: '68.5' } });
		await fireEvent.input(field('Calories'), { target: { value: '2200' } });

		await user.click(screen.getByRole('button', { name: 'Save' }));

		await waitFor(() =>
			expect(mockedSave).toHaveBeenCalledWith(today(), {
				...emptyWeightAndNutrition(),
				weight: 68.5,
				calories: 2200
			})
		);
	});
});
