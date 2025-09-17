<script lang="ts">
	import * as d3 from 'd3';
	import { formatDuration } from '$lib/duration';
	import TimseriesLine, { type AxisPosition } from '../molecules/TimseriesLine.svelte';

	export interface Metric {
		name: string;
		unit: string;
		values: Array<number | null>;
	}

	export interface TimeseriesChartProps {
		time: number[];
		metrics: Array<Metric>;
		width: number;
		height: number;
	}

	let { time, metrics, height, width }: TimeseriesChartProps = $props();
	let marginTop = 20;
	let marginRight = 20;
	let marginBottom = 20;
	let axisWidth = 30;

	let numberOfMetrics = $derived(Math.min(metrics.length, 3));
	let timeScale = $derived.by(() => {
		if (numberOfMetrics === 1) {
			return [axisWidth, width];
		} else if (numberOfMetrics === 2) {
			return [axisWidth, width - axisWidth];
		} else {
			return [2 * axisWidth, width - axisWidth];
		}
	});

	// Start by defining the x (time) scale and axis, common to all metrics
	let gx: SVGGElement;
	let x_min = $derived(time.at(0)!);
	let x_max = $derived(time.at(-1)!);
	let max_zoom = $derived((x_max - x_min) / 60);
	let xScale = $derived(d3.scaleLinear([x_min, x_max], timeScale));
	let zoomedXScale = $derived(xScale);
	let maxTicks = $derived(Math.min(8, Math.floor(width / 70)));
	const generateTicks = (x: d3.ScaleLinear<number, number, never>, nTicks: number) => {
		const [min, max] = x.domain();
		const di = (max - min) / nTicks;
		const ticks = [];
		for (let i = min; i < max; i = i + di) {
			ticks.push(i);
		}
		return ticks;
	};
	let xAxis = $derived(
		(
			g: d3.Selection<SVGGElement, unknown, null, undefined>,
			x: d3.ScaleLinear<number, number, never>
		) =>
			g.call(
				d3
					.axisBottom(x)
					.tickFormat((time, _) => formatDuration(time.valueOf()))
					.tickValues(generateTicks(x, maxTicks))
			)
	);
	$effect(() => {
		d3.select(gx).call(xAxis, xScale);
	});

	// Compute line props for each metric
	let yRange = $derived([height - marginBottom, marginTop]);
	let metricsProps = $derived.by(() => {
		const metricsProps = [];

		for (let idx = 0; idx < Math.min(metrics.length, 3); idx++) {
			const metric = metrics[idx];

			let position: AxisPosition = 'left';
			let color = 'stroke-chart-one';
			if (idx === 1) {
				position = 'right';
				color = 'stroke-chart-two';
			} else if (idx === 2) {
				position = 'leftOffset';
				color = 'stroke-chart-three';
			}
			let values = extractMetricValues(time, metric.values);

			metricsProps.push({
				values,
				range: yRange,
				position,
				color,
				yMargin: 30
			});
		}
		return metricsProps;
	});

	const extractMetricValues = (time: number[], values: Array<number | null>) => {
		let v = [];
		for (let i = 0; i < time.length; i++) {
			let val = values[i];
			if (val !== null) {
				v.push([time[i], val]);
			}
		}
		return v as Array<[number, number]>;
	};

	let svgElement: SVGElement;

	// Create the zoom behavior
	let zoom = $derived(
		d3
			.zoom<SVGElement, unknown>()
			.scaleExtent([1, max_zoom])
			.extent([
				[timeScale[0], 0],
				[timeScale[1], height]
			])
			.translateExtent([
				[timeScale[0], -Infinity],
				[timeScale[1], Infinity]
			])
			.on('zoom', zoomed)
	);

	function zoomed(event: d3.D3ZoomEvent<SVGElement, any>) {
		zoomedXScale = event.transform.rescaleX(xScale);
		d3.select(gx).call(xAxis, zoomedXScale);
	}

	const resetZoom = () => {
		d3.select(svgElement).call(zoom.scaleTo, 1);
	};

	$effect(() => {
		d3.select(svgElement).call(zoom);
	});

	// let tooltipXOffset: number | undefined = $state(undefined);
	// const bisector = d3.bisector<[number, number], number>((point) => point[0]);
	// let tooltip = $derived.by(() => {
	// 	if (tooltipXOffset === undefined) {
	// 		return undefined;
	// 	}

	// 	// let nearestValues = values[bisector.center(values, xScale.invert(tooltipXOffset))];
	// 	return {
	// 		xOffset: 0, // xScale(nearestValues[0]),
	// 		yOffset: 0, // yScale(nearestValues[1]),
	// 		timestamp: '', // formatDuration(nearestValues[0]),
	// 		value: 0 //nearestValues[1]
	// 	};
	// });
	// const tooltipCallback = (event: MouseEvent) => {
	// 	tooltipXOffset = event.offsetX;
	// };
</script>

<button class="btn" onclick={resetZoom}>Reset zoom</button>
<svg
	bind:this={svgElement}
	{width}
	{height}
	viewBox={`0 0 ${width} ${height}`}
	role="img"
	class="select-none"
>
	<clipPath id="clip-path">
		<rect
			x={axisWidth * Math.min(2, numberOfMetrics)}
			y={marginRight}
			width={width - axisWidth * numberOfMetrics}
			height={height - marginTop - marginBottom}
		/>
	</clipPath>

	{#each metricsProps as props}
		<TimseriesLine
			range={props.range}
			values={props.values}
			xScale={zoomedXScale}
			position={props.position}
			color={props.color}
			yMargin={30}
			{width}
		/>
	{/each}

	<g bind:this={gx} transform="translate(0 {height - marginBottom})" />
	<!-- {#if tooltip}
		<g
			transform={`translate(${tooltip.xOffset},${tooltip.yOffset})`}
			pointer-events="none"
			font-family="sans-serif"
			font-size="10"
			text-anchor="middle"
		>
			<rect x="-27" width="54" y="-30" height="24" class="fill-base-100" />
			<text y="-22">{tooltip.timestamp}</text>
			<text y="-12">{tooltip.value.toFixed(2)} </text>
			<circle r="3.5" class="fill-accent" />
		</g>
	{/if} -->
</svg>
