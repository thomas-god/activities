import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';

import { none, some, type Option } from '$lib/Options';
import type { Sport, SportCategory } from '$lib/sport';
import SportFilterV2 from './SportFilterV2.svelte';

const getSummaryText = (container: HTMLElement): string => {
	return container.querySelector('.break-words')?.textContent?.trim() ?? '';
};

afterEach(() => {
	cleanup();
});

describe('SportFilterV2', () => {
	it('does not render anything when sports are not set', () => {
		const { container } = render(SportFilterV2, {
			props: {
				sports: none<Sport[]>() as Option<Sport[]>,
				categories: some<SportCategory[]>([]) as Option<SportCategory[]>
			}
		});

		expect(container.childElementCount).toBe(0);
		expect(screen.queryByText(/Sports:/)).toBeNull();
	});

	it('toggles an individual sport when its category is not selected', async () => {
		const user = userEvent.setup();

		const { container } = render(SportFilterV2, {
			props: {
				sports: some<Sport[]>(['Running']) as Option<Sport[]>,
				categories: some<SportCategory[]>([]) as Option<SportCategory[]>
			}
		});

		expect(getSummaryText(container)).toBe('Sports: Running');

		await user.click(screen.getByRole('button', { name: 'Pen editing icon' }));

		const runningCheckbox = screen.getByLabelText('Running');
		expect(runningCheckbox).toBeChecked();

		await user.click(runningCheckbox);

		expect(getSummaryText(container)).toBe('Sports: All sports');
		expect(screen.getByLabelText('Running')).not.toBeChecked();
	});

	it('toggles a category on and replaces its selected sports with the category', async () => {
		const user = userEvent.setup();

		const { container } = render(SportFilterV2, {
			props: {
				sports: some<Sport[]>(['Yoga']) as Option<Sport[]>,
				categories: some<SportCategory[]>([]) as Option<SportCategory[]>
			}
		});

		expect(getSummaryText(container)).toBe('Sports: Yoga');

		await user.click(screen.getByRole('button', { name: 'Pen editing icon' }));

		const cardioSection = screen.getByText('Gym & Fitness sports').closest('div');
		const categoryCheckbox = cardioSection?.querySelector(
			'input[type="checkbox"]'
		) as HTMLInputElement | null;
		expect(categoryCheckbox).not.toBeNull();

		if (!categoryCheckbox) {
			return;
		}

		await user.click(categoryCheckbox);

		expect(getSummaryText(container)).toBe('Sports: Gym & Fitness');
		expect(screen.getByLabelText('Yoga')).toBeChecked();
		expect(screen.getByLabelText('HIIT')).toBeChecked();
		expect(categoryCheckbox).toBeChecked();
	});

	it('only shows categories and sports allowed by existingSportsConstraints', async () => {
		const user = userEvent.setup();

		const { container } = render(SportFilterV2, {
			props: {
				sports: some<Sport[]>(['Running']) as Option<Sport[]>,
				categories: some<SportCategory[]>(['Cardio']) as Option<SportCategory[]>,
				existingSportsConstraints: some({
					sports: ['Running'],
					categories: ['Cardio']
				}) as Option<{ sports: Sport[]; categories: SportCategory[] }>
			}
		});

		expect(getSummaryText(container)).toBe('Sports: Running, Gym & Fitness');

		await user.click(screen.getByRole('button', { name: 'Pen editing icon' }));

		expect(screen.getByText('Gym & Fitness sports')).toBeInTheDocument();
		expect(screen.getByText('Running sports')).toBeInTheDocument();
		expect(screen.queryByText('Cycling sports')).toBeNull();
		expect(screen.getByLabelText('HIIT')).toBeChecked();
		expect(screen.getByLabelText('Running')).toBeChecked();
		expect(screen.queryByLabelText('TrailRunning')).toBeNull();
		expect(screen.queryByLabelText('Cycling')).toBeNull();
	});

	it('turns a category off and selects the other sports when one sport is toggled from that category', async () => {
		const user = userEvent.setup();

		const { container } = render(SportFilterV2, {
			props: {
				sports: some<Sport[]>([]) as Option<Sport[]>,
				categories: some<SportCategory[]>(['Cardio']) as Option<SportCategory[]>
			}
		});

		expect(getSummaryText(container)).toBe('Sports: Gym & Fitness');

		await user.click(screen.getByRole('button', { name: 'Pen editing icon' }));
		await user.click(screen.getByLabelText('Yoga'));

		expect(getSummaryText(container)).toBe(
			'Sports: Cardio Training, HIIT, Pilates, Strength Training'
		);
		expect(screen.getByLabelText('Yoga')).not.toBeChecked();
		expect(screen.getByLabelText('HIIT')).toBeChecked();
	});

	it('clears sports and categories when delete is clicked', async () => {
		const user = userEvent.setup();
		const { container } = render(SportFilterV2, {
			props: {
				sports: some<Sport[]>(['Running']) as Option<Sport[]>,
				categories: some<SportCategory[]>(['Cardio']) as Option<SportCategory[]>
			}
		});

		await user.click(screen.getByRole('button', { name: 'Bin delete icon' }));

		expect(container.childElementCount).toBe(0);
		expect(screen.queryByText(/Sports:/)).toBeNull();
	});
});
