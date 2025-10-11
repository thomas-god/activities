<script lang="ts">
	let { options, selectedOptions = $bindable() }: { options: string[]; selectedOptions: string[] } =
		$props();

	let values = $state(
		options.map((option) => {
			return { option, selected: false };
		})
	);

	let selectedAsString = $derived(
		selectedOptions.length > 0 ? selectedOptions.join(', ') : 'All sports'
	);

	const updateSelectedOptions = (value: { option: string; selected: boolean }) => {
		if (value.selected) {
			value.selected = false;
			selectedOptions = selectedOptions.filter((option) => option !== value.option);
		} else {
			value.selected = true;
			selectedOptions.push(value.option);
		}
	};
</script>

<div class="flex flex-row items-start gap-1 p-1">
	<label class="label" for="metric-sport-filter"> Sports: </label>
	<span>{selectedAsString}</span>
</div>

<div class="flex flex-row flex-wrap gap-2 p-1">
	{#each values as value}
		<input
			type="checkbox"
			class="btn btn-sm"
			aria-label={value.option}
			onclick={() => updateSelectedOptions(value)}
		/>
	{/each}
</div>

<style>
	.btn {
		background-color: var(--color-base-100);
		color: var(--color-base-content);
	}

	.btn:checked {
		background-color: var(--color-base-100);
		color: var(--color-base-content);
	}
</style>
