<script lang="ts">
	import LineChart from '../../../molecules/LineChart.svelte';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	let summary = $derived.by(() => {
		if (data.activity) {
			return {
				sport: data.activity.sport,
				duration: data.activity.duration,
				start_time: data.activity.start_time
			};
		}
		return undefined;
	});
</script>

{JSON.stringify(summary)}

{#if data.activity}
	<LineChart timeseries={data.activity.timeseries} />
{/if}
