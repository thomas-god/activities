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

	let svgElement: SVGElement;
	let path: SVGPathElement;

	// Define x scale and axis
	let x_min = $derived(values.at(0)![0]);
	let x_max = $derived(values.at(-1)![0]);
	let max_zoom = $derived((x_max - x_min) / 60);
	let x = $derived(d3.scaleLinear([x_min, x_max], [marginLeft, width - marginRight]));
	let xAxis = $derived(
		(
			g: d3.Selection<SVGGElement, unknown, null, undefined>,
			x: d3.ScaleLinear<number, number, never>
		) => g.call(d3.axisBottom(x).tickFormat((time, _) => formatDuration(time.valueOf())))
	);
	let gx: SVGGElement;
	$effect(() => {
		d3.select(gx).call(xAxis, x);
	});

	// Define y scale and axis
	let gy: SVGGElement;
	let y_range = $derived(d3.extent(values, (p) => p[1])) as [number, number];
	let y = $derived(d3.scaleLinear(y_range, [height - marginBottom, marginTop]));
	let yAxis = $derived(d3.axisLeft(y));
	$effect(() => {
		d3.select(gy).call(yAxis);
	});

	// Define line generator
	let line = $derived((data: [number, number][], xScale: d3.ScaleLinear<number, number, never>) =>
		d3
			.line()
			.x((point) => xScale(point[0]))
			.y((point) => y(point[1]))(data)
	);

	// Create the zoom behavior
	let zoom = $derived(
		d3
			.zoom()
			.scaleExtent([1, max_zoom])
			.extent([
				[marginLeft, 0],
				[width - marginRight, height]
			])
			.translateExtent([
				[marginLeft, -Infinity],
				[width - marginRight, Infinity]
			])
			.on('zoom', zoomed)
	);

	function zoomed(event: d3.D3ZoomEvent<SVGElement, any>) {
		const zoomedXScale = event.transform.rescaleX(x);
		d3.select(path).attr('d', line(values, zoomedXScale));
		d3.select(gx).call(xAxis, zoomedXScale);
	}

	const resetZoom = () => {
		d3.select(svgElement).call(zoom.scaleTo, 1);
	};

	$effect(() => {
		d3.select(svgElement).call(zoom);
	});
</script>

<button class="btn" onclick={resetZoom}>Reset zoom</button>
<svg bind:this={svgElement} {width} {height} viewBox={`0 0 ${width} ${height}`}>
	<clipPath id="clip-path">
		<rect
			x={marginLeft}
			y={marginRight}
			width={width - marginLeft - marginRight}
			height={height - marginTop - marginBottom}
		/>
	</clipPath>
	<path
		bind:this={path}
		clip-path="url(#clip-path)"
		fill="none"
		stroke="currentColor"
		stroke-width="1.5"
		d={line(values, x)}
	/>
	<g bind:this={gx} transform="translate(0 {height - marginBottom})" />
	<g bind:this={gy} transform="translate({marginLeft} 0)" />
</svg>
