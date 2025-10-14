<script lang="ts">
	let {
		values,
		legends,
		title,
		unit,
		round = 0
	}: {
		values: number[];
		legends: string[];
		title: string;
		unit: string | undefined;
		round?: number;
	} = $props();
</script>

<div class="stats bg-base-100 shadow">
	<div class="stat place-items-start">
		<div class="stat-title">{title}</div>
		<div class="stat-value grid grid-rows-[1fr_0.75rem] gap-x-3 text-xl font-semibold">
			{#each values as value, idx}
				<div style={`grid-column: ${idx + 1}/${idx + 2}; grid-row: 1;`}>
					{value.toFixed(round)}
					{#if unit !== undefined}
						<span class="text-sm font-normal opacity-60">
							{unit}
						</span>
					{/if}
				</div>
			{/each}
			{#each legends as legend, idx}
				<div
					class="stat-value-small italic"
					style={`grid-column: ${idx + 1}/${idx + 2}; grid-row: 2;`}
				>
					{legend}
				</div>
			{/each}
		</div>
	</div>
</div>

<style>
	.stat-value-small {
		font-size: small;
		font-weight: 400;
		opacity: 60%;
	}
</style>
