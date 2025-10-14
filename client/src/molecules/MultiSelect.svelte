<script lang="ts">
	import { type Metric } from '$lib/colors';

	let {
		availableOptions,
		selectedOptions = $bindable(),
		maxSelected
	}: {
		availableOptions: Array<{ option: Metric; display: string }>;
		selectedOptions: Array<{ option: Metric; display: string }>;
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

<div class="flex flex-wrap gap-1">
	{#each options as option}
		<input
			class={`btn ${option.option.toLocaleLowerCase()}`}
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
	.btn.heartrate:disabled:checked,
	.btn.heartrate:checked {
		background-color: var(--color-heart-rate-chart);
		color: white;
		border-color: transparent;
	}

	.btn.power:disabled:checked,
	.btn.power:checked {
		background-color: var(--color-power-chart);
		color: white;
		border-color: transparent;
	}

	.btn.speed:disabled:checked,
	.btn.speed:checked {
		background-color: var(--color-speed-chart);
		color: white;
		border-color: transparent;
	}

	.btn.altitude:disabled:checked,
	.btn.altitude:checked {
		background-color: var(--color-elevation-chart);
		color: white;
		border-color: transparent;
	}

	.btn.cadence:disabled:checked,
	.btn.cadence:checked {
		background-color: var(--color-cadence-chart);
		color: white;
		border-color: transparent;
	}
</style>
