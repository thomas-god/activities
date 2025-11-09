<script lang="ts">
	import {
		SportCategories,
		sportDisplay,
		sportsPerCategory,
		sportsWithoutCategory,
		type Sport,
		type SportCategory
	} from '$lib/sport';
	import SportSelectCategory from './SportSelectCategory.svelte';

	let {
		selectedSports = $bindable(),
		selectedSportCategories = $bindable()
	}: { selectedSports: Sport[]; selectedSportCategories: SportCategory[] } = $props();

	const sportIsSelected = (sport: Sport): boolean => {
		return selectedSports.includes(sport);
	};

	const categoryIsSelected = (category: SportCategory): boolean => {
		return selectedSportCategories.includes(category);
	};

	const toggleSport = (sport: Sport, category: SportCategory) => {
		const sportIsSelected = selectedSports.includes(sport);
		const categoryIsSelected = selectedSportCategories.includes(category);

		if (!categoryIsSelected) {
			// We just toggle the sport from the selectedSports list
			if (sportIsSelected) {
				selectedSports = selectedSports.filter((current_sport) => current_sport !== sport);
			} else {
				selectedSports.push(sport);
			}
		} else {
			// We toggle the category OFF and add the other sport of the category (excluding this one)
			// to the list of selected sports
			selectedSportCategories = selectedSportCategories.filter(
				(current_category) => current_category !== category
			);
			const otherSports = sportsPerCategory[category].filter(
				(current_sport) => current_sport !== sport
			);
			selectedSports = selectedSports.concat(otherSports);
		}
	};

	const toggleSportWithoutCategory = (sport: Sport) => {
		const sportIsSelected = selectedSports.includes(sport);
		if (sportIsSelected) {
			selectedSports = selectedSports.filter((current_sport) => current_sport !== sport);
		} else {
			selectedSports.push(sport);
		}
	};
	const toggleCategory = (category: SportCategory) => {
		const categoryIsSelected = selectedSportCategories.includes(category);

		if (!categoryIsSelected) {
			// Toggle the category ON and remove its sports from the list of selected sports
			selectedSportCategories.push(category);
			selectedSports = selectedSports.filter(
				(current_sport) => !sportsPerCategory[category].includes(current_sport)
			);
		} else {
			// Just toggle the category OFF
			selectedSportCategories = selectedSportCategories.filter(
				(current_category) => current_category !== category
			);
		}
	};
</script>

<div class="flex h-max flex-col p-1">
	{#each SportCategories as category}
		<SportSelectCategory
			{category}
			sports={sportsPerCategory[category]}
			{categoryIsSelected}
			{sportIsSelected}
			{toggleCategory}
			{toggleSport}
		/>
		<div class="divider"></div>
	{/each}

	<div class="sport-category">
		<div class="sport-category-title">Other sports</div>
		<div class="sport-category-items">
			{#each sportsWithoutCategory.toSorted() as sport}
				<input
					type="checkbox"
					class="btn btn-sm"
					aria-label={sportDisplay(sport)}
					checked={sportIsSelected(sport)}
					onclick={() => toggleSportWithoutCategory(sport)}
				/>
			{/each}
		</div>
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
