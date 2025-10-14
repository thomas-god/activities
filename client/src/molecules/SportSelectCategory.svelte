<script lang="ts">
	import { type Sport, type SportCategory } from '$lib/sport';

	let {
		category,
		sports,
		sportIsSelected,
		categoryIsSelected,
		toggleSport,
		toggleCategory
	}: {
		category: SportCategory;
		sports: Sport[];
		sportIsSelected: (sport: Sport) => boolean;
		categoryIsSelected: (category: SportCategory) => boolean;
		toggleSport: (sport: Sport, category: SportCategory) => void;
		toggleCategory: (category: SportCategory) => void;
	} = $props();
</script>

<div class="sport-category">
	<div class="sport-category-title">
		<span>
			{category} sports
		</span>
		<input
			type="checkbox"
			class="checkbox"
			checked={categoryIsSelected(category)}
			onclick={() => toggleCategory(category)}
		/>
	</div>
	<div class="sport-category-items">
		{#each sports.toSorted() as sport}
			<input
				type="checkbox"
				class="btn btn-sm"
				aria-label={sport}
				checked={sportIsSelected(sport) || categoryIsSelected(category)}
				onclick={() => toggleSport(sport, category)}
			/>
		{/each}
	</div>
</div>

<style>
	.sport-category {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.sport-category-title {
		font-size: 12px;
		font-weight: 600;
		display: flex;
		flex-direction: row;
		gap: 8px;
	}

	.sport-category-items {
		display: flex;
		flex-direction: row;
		flex-wrap: wrap;
		gap: 6px;
	}

	.btn {
		background-color: var(--color-base-100);
		color: var(--color-base-content);
	}

	.btn:checked {
		background-color: var(--color-base-100);
		color: var(--color-base-content);
	}
</style>
