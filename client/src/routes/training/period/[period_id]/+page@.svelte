<script lang="ts">
	import { dayjs } from '$lib/duration';
	import { getSportCategory, sportCategoryIcons, type SportCategory } from '$lib/sport';
	import type { PageProps } from './$types';
	import type { TrainingPeriodDetails } from './+page';

	let { data }: PageProps = $props();

	const period = data.periodDetails;

	const sportIcons = (sports: TrainingPeriodDetails['sports']): string[] => {
		const icons: Set<string> = new Set();

		for (const category of sports.categories) {
			if (category in sportCategoryIcons) {
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

<div class="mx-auto mt-4 flex flex-col gap-4">
	<div class="rounded-box bg-base-100 rounded-t-none p-4 shadow-md">
		<div class="flex items-center gap-4">
			<div class="text-3xl">🗓️</div>
			<div class="flex-1">
				<div class="text-xl font-semibold">{period.name}</div>
				<div class="text-sm opacity-70">
					{dayjs(period.start).format('MMM D, YYYY')} · {period.end === null
						? 'Ongoing'
						: dayjs(period.end).format('MMM D, YYYY')}
				</div>
			</div>
			<div class="flex items-center gap-2">
				{#each sportIcons(period.sports) as icon}
					<div class="text-lg">{icon}</div>
				{:else}
					<div class="italic opacity-70">All sports</div>
				{/each}
			</div>
		</div>

		{#if period.note}
			<div class="bg-base-200 mt-4 rounded p-3">{period.note}</div>
		{/if}
	</div>

	<!-- Placeholder for future charts or aggregated metrics -->
	<div class="rounded-box bg-base-100 p-4 shadow-md">
		<div class="text-sm italic opacity-60">
			Training period summary and metrics will be shown here.
		</div>
	</div>
</div>

<style>
	.rounded-box {
		border-radius: 8px;
	}
</style>
