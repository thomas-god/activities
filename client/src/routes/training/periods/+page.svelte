<script lang="ts">
	import { dayjs } from '$lib/duration';
	import {
		getSportCategory,
		SportCategories,
		sportCategoryIcons,
		type SportCategory
	} from '$lib/sport';
	import type { TrainingPeriodDetails } from '../period/[period_id]/+page';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	let periods = $derived(data.periods.toSorted((a, b) => (a.start < b.start ? 1 : -1)));

	let sportIcons = (sports: TrainingPeriodDetails['sports']): string[] => {
		const icons: Set<string> = new Set();

		for (const category of sports.categories) {
			if (SportCategories.includes(category)) {
				icons.add(sportCategoryIcons[category as SportCategory]);
			}
		}

		for (const sport of sports.sports) {
			const category = getSportCategory(sport);
			if (category !== null) {
				icons.add(sportCategoryIcons[category]);
			}
		}

		return Array.from(icons);
	};
</script>

<div class="mx-auto flex flex-col gap-4">
	<div class="rounded-box bg-base-100 rounded-t-none shadow-md">
		<div class="p-2 px-4 text-sm italic tracking-wide opacity-60">
			Training periods are spans of time with a specific training focus (base training, race, etc.)
		</div>
		<div>
			{#each periods as period}
				{@const icons = sportIcons(period.sports)}
				<a href={`/training/period/${period.id}`} class={`item flex flex-1 items-center p-3 `}>
					<div class={`icon `}>🗓️</div>
					<div class="flex-1">
						<div class="flex flex-col">
							<div class="mb-1 font-semibold">{period.name}</div>
							<div class="text-xs font-light">
								{dayjs(period.start).format('MMM D, YYYY')} . {period.end === null
									? 'Ongoing'
									: dayjs(period.end).format('MMM D, YYYY')}
							</div>
						</div>
					</div>
					<div class="flex flex-row">
						{#each icons as icon}
							<div class="text-lg">{icon}</div>
						{:else}
							<div class="italic opacity-70">All sports</div>
						{/each}
					</div>
				</a>
			{:else}
				<div class="italic text-sm text-center tracking-wide opacity-60">No training periods</div>
			{/each}
		</div>
	</div>
</div>

<style>
	.item:hover {
		background: #f7fafc;
	}

	.item {
		box-sizing: border-box;
		border-left: 4px solid transparent;
		border-radius: 8px;
	}

	.item.cycling {
		border-left-color: var(--color-cycling);
	}

	.item.running {
		border-left-color: var(--color-running);
	}

	.item.other {
		border-left-color: var(--color-other);
	}

	.icon {
		width: 40px;
		height: 40px;
		border-radius: 8px;
		display: flex;
		align-items: center;
		justify-content: center;
		margin-right: 16px;
		font-size: 20px;
		flex-shrink: 0;
		background: var(--color-other-background);
		color: var(--color-other);
	}
</style>
