<script lang="ts">
	import { invalidate } from '$app/navigation';
	import { PUBLIC_APP_URL } from '$env/static/public';
	import CreateTrainingMetric from '../../organisms/CreateTrainingMetric.svelte';
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

			metrics.push({
				values: values,
				title: `${metric.metric} (${metric.granularity})`,
				unit: metric.unit,
				id: metric.id
			});
		}
		return metrics;
	});

	const createMetricCallback = async (payload: string): Promise<void> => {
		await fetch(`${PUBLIC_APP_URL}/api/training/metric`, {
			body: payload,
			method: 'POST',

			headers: { 'Content-Type': 'application/json' }
		});
		invalidate('app:training-metrics');
	};

	const deleteMetricCallback = async (metric: string): Promise<void> => {
		await fetch(`${PUBLIC_APP_URL}/api/training/metric/${metric}`, {
			method: 'DELETE'
		});
		invalidate('app:training-metrics');
	};
</script>

<div class="mx-2 mt-5 rounded-box bg-base-100 shadow-md sm:mx-auto sm:w-2xl">
	<CreateTrainingMetric callback={createMetricCallback} />
</div>

{#each metrics as metric}
	<div
		bind:clientWidth={chartWidth}
		class="mx-2 mt-5 rounded-box bg-base-100 shadow-md sm:mx-auto sm:w-2xl"
	>
		<div class="relative p-4 text-center">
			<div>
				{metric.title}
			</div>
			<button
				class="btn absolute right-4 bottom-[8px] border-0 bg-base-100 p-0 shadow-none hover:outline-2 hover:outline-base-300"
				onclick={() => deleteMetricCallback(metric.id)}>🗑️</button
			>
		</div>
		<TrainingMetricsChart
			height={250}
			width={chartWidth}
			values={metric.values}
			unit={metric.unit}
		/>
	</div>
{/each}
