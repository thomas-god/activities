<script lang="ts">
	import { isSome, type Option, none, isNone, map, some, unwrapOr } from '$lib/Options';
	import {
		SportCategories,
		sportCategoryDisplay,
		sportDisplay,
		sportsPerCategory,
		type Sport,
		type SportCategory
	} from '$lib/sport';

	let {
		sports = $bindable(),
		categories = $bindable(),
		allowDelete = true
	}: {
		sports: Option<Sport[]>;
		categories: Option<SportCategory[]>;
		allowDelete?: boolean;
	} = $props();
	let editing = $state(false);

	const sportIsSelected = (sport: Sport): boolean => {
		return unwrapOr(
			map(sports, (s) => s.includes(sport)),
			false
		);
	};

	const categoryIsSelected = (category: SportCategory): boolean => {
		return unwrapOr(
			map(categories, (s) => s.includes(category)),
			false
		);
	};

	const toggleSport = (sport: Sport, category: SportCategory) => {
		if (isNone(sports) || isNone(categories)) {
			return;
		}

		const selectedSports = sports.value;
		const selectedCategories = categories.value;
		const sportIsSelected = selectedSports.includes(sport);
		const categoryIsSelected = selectedCategories.includes(category);

		if (!categoryIsSelected) {
			// We just toggle the sport from the sports.value list
			if (sportIsSelected) {
				sports = some(selectedSports.filter((current_sport) => current_sport !== sport));
			} else {
				sports = some([...selectedSports, sport]);
			}
		} else {
			// We toggle the category OFF and add the other sport of the category (excluding this one)
			// to the list of selected sports
			categories = some(
				selectedCategories.filter((current_category) => current_category !== category)
			);
			const otherSports = sportsPerCategory[category].filter(
				(current_sport) => current_sport !== sport
			);
			sports = some(selectedSports.concat(otherSports));
		}
	};

	const toggleCategory = (category: SportCategory) => {
		if (isNone(sports) || isNone(categories)) {
			return;
		}
		const selectedSports = sports.value;
		const selectedCategories = categories.value;
		const categoryIsSelected = selectedCategories.includes(category);

		if (!categoryIsSelected) {
			// Toggle the category ON and remove its sports from the list of selected sports
			categories = some([...selectedCategories, category]);
			sports = some(
				selectedSports.filter(
					(current_sport) => !sportsPerCategory[category].includes(current_sport)
				)
			);
		} else {
			// Just toggle the category OFF
			categories = some(
				selectedCategories.filter((current_category) => current_category !== category)
			);
		}
	};

	let display = $derived.by(() => {
		let values: string[] = [];

		if (isSome(sports)) {
			values = values.concat(sports.value.map((s) => sportDisplay(s)).toSorted());
		}

		if (isSome(categories))
			values = values.concat(categories.value.map((c) => sportCategoryDisplay(c)).toSorted());

		return values.length > 0 ? values.join(', ') : 'All sports';
	});
</script>

{#if isSome(sports) && isSome(categories)}
	<div class="flex flex-col gap-1 rounded border border-black/30 p-1.5 shadow">
		<div class="flex flex-row items-center justify-between">
			<div class="flex-1 text-wrap break-words">
				Sports: {display}
			</div>
			<div class="join shrink-0">
				<button class="btn join-item btn-xs" onclick={() => (editing = !editing)}>
					<img src="/icons/edit.svg" alt="Pen editing icon" class="inline h-5 w-5" />
				</button>
				{#if allowDelete}
					<button
						class="btn join-item btn-xs"
						onclick={() => {
							sports = none();
							categories = none();
						}}
					>
						<img src="/icons/delete.svg" alt="Bin delete icon" class="inline h-5 w-5" />
					</button>
				{/if}
			</div>
		</div>
		{#if editing}
			<div class="divider my-0.5 px-2"></div>
			<div class="flex max-h-64 flex-col gap-2 overflow-scroll px-1">
				{#each SportCategories as category}
					<div class="flex flex-col gap-1 p-1">
						<div
							class="flex flex-row items-center gap-1.5 border-b-1 border-b-black/45 pb-0.5 pl-0.5"
						>
							<span class="font-semibold">
								{#if ['WaterSports', 'TeamSports'].includes(category)}
									{sportCategoryDisplay(category)}
								{:else}
									{sportCategoryDisplay(category)} sports
								{/if}
							</span>
							<input
								type="checkbox"
								class="checkbox rounded-sm checkbox-xs checkbox-primary"
								checked={categoryIsSelected(category)}
								onclick={() => toggleCategory(category)}
							/>
						</div>
						<div class="flex flex-row flex-wrap gap-1">
							{#each sportsPerCategory[category].toSorted() as sport}
								<input
									type="checkbox"
									class="btn btn-soft btn-xs"
									aria-label={sportDisplay(sport)}
									checked={sportIsSelected(sport) || categoryIsSelected(category)}
									onclick={() => toggleSport(sport, category)}
								/>
							{/each}
						</div>
					</div>
				{/each}
			</div>
		{/if}
	</div>
{/if}
