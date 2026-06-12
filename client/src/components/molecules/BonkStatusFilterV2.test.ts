import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';

import { none, some, type Option } from '$lib/Options';
import type { BonkStatus } from '$lib/nutrition';
import BonkStatusFilterV2 from './BonkStatusFilterV2.svelte';

afterEach(() => {
	cleanup();
});

describe('BonkStatusFilterV2', () => {
	it('does not render anything when bonk status is not set', () => {
		const { container } = render(BonkStatusFilterV2, {
			props: { bonkStatus: none<BonkStatus>() as Option<BonkStatus> }
		});

		expect(container.childElementCount).toBe(0);
		expect(screen.queryByText(/Bonk status:/)).toBeNull();
	});

	it('shows the selected bonk status and updates when editing', async () => {
		const user = userEvent.setup();

		render(BonkStatusFilterV2, {
			props: { bonkStatus: some<BonkStatus>('none') as Option<BonkStatus> }
		});

		expect(screen.getByText('Bonk status: No bonk')).toBeTruthy();
		expect(screen.queryAllByRole('radio')).toHaveLength(0);

		await user.click(screen.getByRole('button', { name: 'Pen editing icon' }));

		const bonkedRadio = screen.getByLabelText('Bonked');
		expect(screen.getByLabelText('No bonk')).toBeChecked();

		await user.click(bonkedRadio);

		expect(screen.getByText('Bonk status: Bonked')).toBeTruthy();
		expect(screen.getByLabelText('Bonked')).toBeChecked();
		expect(screen.getByLabelText('No bonk')).not.toBeChecked();
	});

	it('clears the filter when delete is clicked', async () => {
		const user = userEvent.setup();
		const { container } = render(BonkStatusFilterV2, {
			props: { bonkStatus: some<BonkStatus>('bonked') as Option<BonkStatus> }
		});

		await user.click(screen.getByRole('button', { name: 'Bin delete icon' }));

		expect(container.childElementCount).toBe(0);
		expect(screen.queryByText(/Bonk status:/)).toBeNull();
	});
});
