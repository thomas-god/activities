import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';

import { none, some, type Option } from '$lib/Options';
import type { RPEValue } from '$lib/rpe';
import RpeFilterV2 from './RpeFilterV2.svelte';

afterEach(() => {
	cleanup();
});

describe('RpeFilterV2', () => {
	it('does not render anything when rpes are not set', () => {
		const { container } = render(RpeFilterV2, {
			props: { rpes: none<RPEValue[]>() as Option<RPEValue[]> }
		});

		expect(container.childElementCount).toBe(0);
		expect(screen.queryByText(/RPEs:/)).toBeNull();
	});

	it('shows the selected rpes and updates when editing', async () => {
		const user = userEvent.setup();

		render(RpeFilterV2, {
			props: { rpes: some<RPEValue[]>([8, 3]) as Option<RPEValue[]> }
		});

		expect(screen.getByText('RPEs: 3, 8')).toBeTruthy();
		expect(screen.queryAllByRole('checkbox')).toHaveLength(0);

		await user.click(screen.getByRole('button', { name: 'Pen editing icon' }));

		const threeCheckbox = screen.getByLabelText('3');
		expect(threeCheckbox).toBeChecked();

		await user.click(threeCheckbox);

		expect(screen.getByText('RPEs: 8')).toBeTruthy();
		expect(screen.getByLabelText('3')).not.toBeChecked();
	});

	it('clears the filter when delete is clicked', async () => {
		const user = userEvent.setup();
		const { container } = render(RpeFilterV2, {
			props: { rpes: some<RPEValue[]>([5]) as Option<RPEValue[]> }
		});

		await user.click(screen.getByRole('button', { name: 'Bin delete icon' }));

		expect(container.childElementCount).toBe(0);
		expect(screen.queryByText(/RPEs:/)).toBeNull();
	});
});
