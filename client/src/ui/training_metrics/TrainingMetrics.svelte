<script lang="ts">
	import type { TrainingMetric } from '$lib/api';
	import { map, none, unwrapOr, type Option } from '$lib/Options';
	import TrainingMetricsCarousel from './internal/TrainingMetricsCarousel.svelte';
	import TrainingMetricsList from './internal/TrainingMetricsList.svelte';

	let {
		metrics,
		height,
		onMetricUpdate,
		screenWidth,
		timeDomain = none(),
		breakpoint = none()
	}: {
		metrics: TrainingMetric[];
		height: number;
		onMetricUpdate: () => void;
		screenWidth: number;
		timeDomain?: Option<{ start: string; end: string | null }>;
		breakpoint?: Option<number>;
	} = $props();
</script>

{#if metrics.length > 0}
	{#if unwrapOr( map(breakpoint, (br) => screenWidth < br), false )}
		<TrainingMetricsCarousel {metrics} {height} {onMetricUpdate} {timeDomain} />
	{:else}
		<TrainingMetricsList {metrics} {height} {onMetricUpdate} {timeDomain} />
	{/if}
{:else}
	<div class="mt-4 text-center text-sm tracking-wide italic opacity-60">No training metrics</div>
{/if}
