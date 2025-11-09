<script lang="ts">
	import TrainingPeriodCard from '$components/molecules/TrainingPeriodCard.svelte';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	let periods = $derived(data.periods.toSorted((a, b) => (a.start < b.start ? 1 : -1)));
</script>

<div class="mx-auto flex flex-col gap-4">
	<div class="rounded-box bg-base-100 shadow-md">
		<div class="p-2 px-4 text-sm tracking-wide italic opacity-60">
			Training periods are spans of time with a specific training focus (base training, race, etc.)
		</div>
		<div class="mb-2 flex flex-col gap-2 px-2">
			{#each periods as period}
				<TrainingPeriodCard {period} />
			{:else}
				<div class="text-center text-sm italic tracking-wide opacity-60">No training periods</div>
			{/each}
		</div>
	</div>
</div>
