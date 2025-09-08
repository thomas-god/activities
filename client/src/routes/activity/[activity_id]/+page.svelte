<script lang="ts">
	import { formatDuration } from '$lib/duration';
	import TimeseriesChart from '../../../molecules/TimeseriesChart.svelte';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	let summary = $derived.by(() => {
		if (data.activity) {
			return {
				sport: data.activity.sport,
				duration: formatDuration(data.activity.duration),
				start_time: data.activity.start_time
			};
		}
		return undefined;
	});
</script>

{JSON.stringify(summary)}

<div class="m-3">
	{#if data.activity}
		<TimeseriesChart timeseries={data.activity.timeseries} targetMetric={'Power'} />
	{/if}
</div>
