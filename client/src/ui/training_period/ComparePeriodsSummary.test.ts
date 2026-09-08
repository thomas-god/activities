import { render, screen, cleanup } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import type { Activity, ActivityList } from '$lib/api/activities';
import type { TrainingPeriodDetails } from '$lib/api';
import { resolve } from '$app/paths';

import ComparePeriodsSummary from './ComparePeriodsSummary.svelte';

const makeActivity = (metrics: Record<string, { value: number; unit: string }>): Activity => ({
	id: 'activity-1',
	name: null,
	sport: 'Running',
	sport_category: 'Running',
	start_time: '2026-02-10T10:00:00+01:00',
	rpe: null,
	workout_type: null,
	feedback: null,
	nutrition: null,
	metrics
});

const makePeriod = (overrides: Partial<TrainingPeriodDetails> = {}): TrainingPeriodDetails => ({
	id: 'period-1',
	start: '2026-02-01',
	end: '2026-04-30',
	name: 'Base block',
	sports: { sports: [], categories: [] },
	note: null,
	activities: [] as ActivityList,
	...overrides
});

afterEach(() => {
	vi.useRealTimers();
	cleanup();
});

describe('ComparePeriodsSummary', () => {
	it('renders a column per period, linking to the period page', () => {
		render(ComparePeriodsSummary, {
			props: {
				periods: [makePeriod(), makePeriod({ id: 'period-2', name: 'Build block' })]
			}
		});

		expect(screen.getByRole('link', { name: 'Base block' })).toHaveAttribute(
			'href',
			resolve('/training/period/period-1')
		);
		expect(screen.getByRole('link', { name: 'Build block' })).toHaveAttribute(
			'href',
			resolve('/training/period/period-2')
		);
	});

	it('summarizes the main statistics of each period', () => {
		// Pin "now" so the ongoing period's length is deterministic
		vi.useFakeTimers({ now: new Date('2026-05-03T12:00:00') });
		render(ComparePeriodsSummary, {
			props: {
				periods: [
					makePeriod({
						activities: [
							makeActivity({
								ActiveDuration: { value: 3600, unit: 's' },
								Distance: { value: 10000, unit: 'm' },
								Elevation: { value: 250, unit: 'm' }
							}),
							makeActivity({
								ActiveDuration: { value: 1800, unit: 's' },
								Distance: { value: 5200, unit: 'm' },
								Elevation: { value: 120, unit: 'm' }
							})
						]
					}),
					makePeriod({
						id: 'period-2',
						name: 'Build block',
						start: '2026-05-01',
						end: null
					})
				]
			}
		});

		expect(screen.getByText('Feb 1, 2026 – Apr 30, 2026')).toBeInTheDocument();
		expect(screen.getByText('May 1, 2026 – Ongoing')).toBeInTheDocument();
		expect(screen.getByText('12 weeks 5 days')).toBeInTheDocument();
		expect(screen.getByText('3 days')).toBeInTheDocument();
		expect(screen.getByText('2')).toBeInTheDocument();
		expect(screen.getByText('1h 30m')).toBeInTheDocument();
		expect(screen.getByText('15 km')).toBeInTheDocument();
		expect(screen.getByText('370 m')).toBeInTheDocument();
	});

	it('shows a dash for missing values', () => {
		render(ComparePeriodsSummary, {
			props: {
				periods: [makePeriod({ activities: [makeActivity({})] })]
			}
		});

		const dashes = screen.getAllByText('—');
		expect(dashes.length).toBeGreaterThan(0);
	});
});
