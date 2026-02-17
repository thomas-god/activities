<script lang="ts">
	import { type Sport, type SportCategory } from '$lib/sport';
	import SportsSelect from './SportsSelect.svelte';

	let {
		enabled = $bindable(false),
		selectedSports = $bindable([]),
		selectedSportCategories = $bindable([])
	}: {
		enabled?: boolean;
		selectedSports?: Sport[];
		selectedSportCategories?: SportCategory[];
	} = $props();
</script>

<div class="mb-2 font-semibold">
	<span class="pr-2"> Filter activities by sports </span>
	<input type="checkbox" bind:checked={enabled} class="toggle toggle-sm" />
</div>
{#if enabled}
	<div class="max-h-96 overflow-scroll">
		<SportsSelect bind:selectedSports bind:selectedSportCategories />
	</div>
{/if}
