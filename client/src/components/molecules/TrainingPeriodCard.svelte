<script lang="ts">
	import { dayjs } from '$lib/duration';
	import {
		getSportCategory,
		SportCategories,
		sportCategoryIcons,
		type Sport,
		type SportCategory
	} from '$lib/sport';

	let {
		period
	}: {
		period: {
			id: string;
			name: string;
			start: string;
			end: string | null;
			sports: { sports: Sport[]; categories: SportCategory[] };
		};
	} = $props();

	let sportIcons = $derived.by(() => {
		const icons: Set<string> = new Set();

		for (const category of period.sports.categories) {
			if (SportCategories.includes(category)) {
				icons.add(sportCategoryIcons[category]);
			}
		}

		for (const sport of period.sports.sports) {
			const category = getSportCategory(sport);
			if (category !== null) {
				icons.add(sportCategoryIcons[category]);
			}
		}

		return Array.from(icons);
	});
</script>

<a href={`/training/period/${period.id}`} class="item flex flex-1 items-center py-1">
	<div class="icon"><img src="/icons/calendar.svg" class="h-6 w-6" alt="Calendar icon" /></div>
	<div class="flex-1">
		<div class="flex flex-col">
			<div class=" font-semibold">{period.name}</div>
			<div class="flex flex-row items-center gap-2 text-xs font-light">
				<div>
					{dayjs(period.start).format('MMM D, YYYY')} · {period.end === null
						? 'Ongoing'
						: dayjs(period.end).format('MMM D, YYYY')}
				</div>
				<div class="flex flex-row items-center gap-2">
					{#each sportIcons as icon}
						<div class="text-sm">{icon}</div>
					{:else}
						<div class="text-sm italic opacity-70">All sports</div>
					{/each}
				</div>
			</div>
		</div>
	</div>
</a>

<style>
	.item:hover {
		background: var(--color-base-200);
	}

	.item {
		box-sizing: border-box;
		border-left: 4px solid transparent;
		border-radius: 8px;
	}

	.icon {
		width: 40px;
		height: 40px;
		border-radius: 8px;
		display: flex;
		align-items: center;
		justify-content: center;
		margin-right: 4px;
		font-size: 20px;
		flex-shrink: 0;
	}
</style>
