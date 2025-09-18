<script lang="ts">
	let {
		availableOptions,
		selectedOptions = $bindable(),
		maxSelected
	}: {
		availableOptions: Array<{ option: string; display: string }>;
		selectedOptions: Array<{ option: string; display: string }>;
		maxSelected: number;
	} = $props();

	let options = $state(
		availableOptions.map((option) => ({
			selected:
				selectedOptions.find((selectedOption) => selectedOption.display === option.display) !==
				undefined,
			...option
		}))
	);

	let numberSelectedOptions = $derived(
		options.reduce((count, option) => (option.selected ? count + 1 : count), 0)
	);

	const onInput = () => {
		selectedOptions = options.filter((option) => option.selected);
	};
</script>

<div class="flex gap-1">
	{#each options as option}
		<input
			class="btn"
			type="checkbox"
			bind:checked={option.selected}
			disabled={(!option.selected && numberSelectedOptions >= maxSelected) ||
				(option.selected && numberSelectedOptions === 1)}
			aria-label={option.display}
			onchange={onInput}
			name="selectedOptions"
		/>
	{/each}
</div>

<style>
	.btn:disabled:checked {
		background-color: var(--color-primary);
		color: var(--color-primary-content);
	}
</style>
