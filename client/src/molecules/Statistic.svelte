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

<div class="stats bg-base-100 shadow">
	<div class="stat place-items-start">
		<div class="stat-title">{title}</div>
		<div class="stat-value">
			{#if value === undefined}
				-
			{:else}
				{value.toFixed(round)}

				{#if unit !== undefined}
					<span class="stat-value-unit">
						{unit}
					</span>
				{/if}
			{/if}
		</div>
	</div>
</div>

<style>
	.stat-value {
		font-size: x-large;
		font-weight: 600;
	}

	.stat-value-unit {
		font-size: small;
		font-weight: 400;
		opacity: 60%;
	}
</style>
