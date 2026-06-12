import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';

import { none, some, type Option } from '$lib/Options';
import WorkoutTypeFilterV2 from './WorkoutTypeFilterV2.svelte';
import type { WorkoutType } from '$lib/workout-type';

afterEach(() => {
	cleanup();
});

describe('WorkoutTypeFilterV2', () => {
	it('does not render anything when workout types are not set', () => {
		const { container } = render(WorkoutTypeFilterV2, {
			props: { workoutTypes: none<WorkoutType[]>() }
		});

		expect(container.childElementCount).toBe(0);
		expect(screen.queryByText(/Workout types:/)).toBeNull();
	});

	it('shows the selected workout types and updates when editing', async () => {
		const user = userEvent.setup();

		render(WorkoutTypeFilterV2, {
			props: { workoutTypes: some<WorkoutType[]>(['tempo', 'easy']) }
		});

		expect(screen.getByText('Workout types: Easy, Tempo')).toBeTruthy();
		expect(screen.queryAllByRole('checkbox')).toHaveLength(0);

		await user.click(screen.getByRole('button', { name: 'Pen editing icon' }));

		const tempoCheckbox = screen.getByLabelText('Tempo');
		expect(tempoCheckbox).toBeTruthy();
		expect(tempoCheckbox).toBeChecked();

		await user.click(tempoCheckbox);

		expect(screen.getByText('Workout types: Easy')).toBeTruthy();
		expect(screen.getByLabelText('Tempo')).not.toBeChecked();
	});

	it('clears the filter when delete is clicked', async () => {
		const user = userEvent.setup();
		const { container } = render(WorkoutTypeFilterV2, {
			props: { workoutTypes: some<WorkoutType[]>(['race']) }
		});

		await user.click(screen.getByRole('button', { name: 'Bin delete icon' }));

		expect(container.childElementCount).toBe(0);
		expect(screen.queryByText(/Workout types:/)).toBeNull();
	});
});
