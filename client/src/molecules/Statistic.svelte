<script lang="ts">
	let {
		value,
		title,
		unit,
		round = 0
	}: {
		value: number | undefined;
		title: string;
		unit: string | undefined;
		round?: number;
	} = $props();
</script>

<div class="stats shadow">
	<div class="stat place-items-center">
		<div class="stat-title">{title}</div>
		<div class="stat-value">
			{#if value === undefined}
				-
			{:else}
				{value.toFixed(round)}
				{#if unit !== undefined}
					{unit}
				{/if}
			{/if}
		</div>
	</div>
</div>

<style>
	.stat-value {
		font-size: 1rem;
		font-weight: 600;
	}
</style>
