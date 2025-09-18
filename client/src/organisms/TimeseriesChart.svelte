<script lang="ts">
	import * as d3 from 'd3';
	import { formatDuration } from '$lib/duration';
	import TimseriesLine, { type LineOrder } from '../molecules/TimseriesLine.svelte';

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
	let margins = $derived.by(() => {
		if (numberOfMetrics === 1) {
			return { left: axisWidth, right: 0 };
		} else if (numberOfMetrics === 2) {
			return { left: axisWidth, right: axisWidth };
		} else {
			return { left: axisWidth * 2, right: axisWidth };
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

			let order: LineOrder = 'first';
			if (idx === 1) {
				order = 'second';
			} else if (idx === 2) {
				order = 'third';
			}
			let values = extractMetricValues(time, metric.values);

			metricsProps.push({
				values,
				range: yRange,
				order,
				name: metric.name,
				unit: metric.unit
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

	let tooltipXOffset: number | undefined = $state(undefined);
	const tooltipCallback = (event: MouseEvent) => {
		tooltipXOffset = Math.min(Math.max(event.offsetX, margins.left), width - margins.right);
	};

	const bisector = d3.bisector<[number, number], number>((point) => point[0]);
	let nearestValues = $derived.by(() => {
		if (tooltipXOffset === undefined) {
			return undefined;
		}
		const values = metricsProps.map((metric) => {
			let nearestValue =
				metric.values[bisector.center(metric.values, zoomedXScale.invert(tooltipXOffset))];
			return {
				metric: metric.name,
				value: nearestValue[1],
				unit: metric.unit,
				order: metric.order
			};
		});
		return { time: zoomedXScale.invert(tooltipXOffset), values };
	});

	const tooltipColors: Record<LineOrder, string> = {
		first: 'text-first-chart',
		second: 'text-second-chart',
		third: 'text-third-chart'
	};
</script>

<button class="btn" onclick={resetZoom}>Reset zoom</button>

{#if nearestValues}
	<p class="flex justify-center pt-2 text-sm sm:text-base">
		<span class="px-1.5">
			⌚ {formatDuration(nearestValues.time)} :
		</span>
		{#each nearestValues.values as value}
			<span class={`px-1.5 ${tooltipColors[value.order]}`}>
				{value.metric}: {value.value.toFixed(2)}
				{value.unit}
			</span>
		{/each}
	</p>
{/if}
<svg
	bind:this={svgElement}
	{width}
	{height}
	viewBox={`0 0 ${width} ${height}`}
	role="img"
	class="select-none"
	onmousemove={tooltipCallback}
>
	<clipPath id="clip-path">
		<rect
			x={margins.left}
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
			order={props.order}
			yMargin={axisWidth}
			{width}
		/>
	{/each}

	<g bind:this={gx} transform="translate(0 {height - marginBottom})" />
	{#if tooltipXOffset}
		<line
			stroke="black"
			x1={tooltipXOffset}
			x2={tooltipXOffset}
			y1={marginTop}
			y2={height - marginBottom}
			stroke-dasharray="3, 2"
		/>
	{/if}
</svg>
