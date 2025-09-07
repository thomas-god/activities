<script lang="ts">
	import * as d3 from 'd3';
	import type { Timeseries } from '../routes/activity/[activity_id]/+page';

	let { timeseries }: { timeseries: Timeseries } = $props();
	let width = 640;
	let height = 400;
	let marginTop = 20;
	let marginRight = 20;
	let marginBottom = 20;
	let marginLeft = 20;

	let power = $derived(
		timeseries
			.filter(({ metrics }) => {
				for (const [metric, _] of metrics) {
					if (metric === 'Power') {
						return true;
					}
				}
				return false;
			})
			.map(({ time, metrics }) => {
				const [_, power] = metrics.find(([metric, _]) => metric === 'Power')!;
				return [time, power] as [number, number];
			})
	);

	let x = $derived(
		d3.scaleLinear([power.at(0)![0], power.at(-1)![0]], [marginLeft, width - marginRight])
	);
	let y_range = $derived(d3.extent(power, (p) => p[1])) as [number, number];
	let y = $derived(d3.scaleLinear(y_range, [height - marginBottom, marginTop]));
	let line = $derived(
		d3
			.line()
			.x((point) => x(point[0]))
			.y((point) => y(point[1]))
	);
</script>

<svg {width} {height}>
	<path fill="none" stroke="currentColor" stroke-width="1.5" d={line(power)} />
</svg>
