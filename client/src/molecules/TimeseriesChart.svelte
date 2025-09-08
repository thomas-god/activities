<script lang="ts">
	import * as d3 from 'd3';
	import type { Timeseries } from '../routes/activity/[activity_id]/+page';
	import { formatDuration } from '$lib/duration';

	export interface TimeseriesChartProps {
		timeseries: Timeseries;
		targetMetric: String;
	}

	let { timeseries, targetMetric }: TimeseriesChartProps = $props();
	let width = 640;
	let height = 400;
	let marginTop = 20;
	let marginRight = 20;
	let marginBottom = 20;
	let marginLeft = 30;

	let values = $derived(
		timeseries.reduce(
			(filtered, current) => {
				for (const [metric, value] of current.metrics) {
					if (metric === targetMetric) {
						filtered.push([current.time, value]);
						break;
					}
				}
				return filtered;
			},
			[] as [number, number][]
		)
	);

	let x = $derived(
		d3.scaleLinear([values.at(0)![0], values.at(-1)![0]], [marginLeft, width - marginRight])
	);
	let y_range = $derived(d3.extent(values, (p) => p[1])) as [number, number];
	let y = $derived(d3.scaleLinear(y_range, [height - marginBottom, marginTop]));
	let line = $derived(
		d3
			.line()
			.x((point) => x(point[0]))
			.y((point) => y(point[1]))
	);

	let gx: SVGGElement;
	let gy: SVGGElement;

	$inspect(Math.max(...values.map(([_, v]) => v)));

	const updateXAxis = (selection: d3.Selection<SVGGElement, any, any, any>) => {
		const axis = d3.axisBottom(x);
		axis.tickFormat((time, _) => formatDuration(time.valueOf()));
		axis(selection);
	};

	$effect(() => {
		d3.select<SVGGElement, any>(gx).call(updateXAxis);
	});
	$effect(() => {
		d3.select(gy).call(d3.axisLeft(y));
	});
</script>

<svg {width} {height}>
	<path fill="none" stroke="currentColor" stroke-width="1.5" d={line(values)} />
	<g bind:this={gx} transform="translate(0 {height - marginBottom})" />
	<g bind:this={gy} transform="translate({marginLeft} 0)" />
</svg>
