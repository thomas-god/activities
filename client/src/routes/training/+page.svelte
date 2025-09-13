<script lang="ts">
	import TrainingMetricsChart from '../../organisms/TrainingMetricsChart.svelte';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	let chartWidth: number = $state(0);

	let metrics = $derived.by(() => {
		let metrics = [];
		for (let i = 0; i < data.metrics.length; i++) {
			let metric = data.metrics.at(i);
			if (metric === undefined) {
				continue;
			}
			let values = [];
			for (const dt in metric.values) {
				values.push({ time: dt, value: metric.values[dt] });
			}

			metrics.push({ values: values, title: `${metric.metric} (${metric.granularity})` });
		}
		return metrics;
	});
</script>

{#each metrics as metric}
	<div
		bind:clientWidth={chartWidth}
		class="rounded-box bg-base-100 sm:w-2xl mx-2 mt-5 shadow-md sm:mx-auto"
	>
		<p class="text-center">{metric.title}</p>
		<TrainingMetricsChart
			height={250}
			width={chartWidth}
			values={metric.values}
			title={metric.title}
		/>
	</div>
{/each}
