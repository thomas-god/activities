<script lang="ts">
	import * as d3 from 'd3';
	import { formatDuration } from '$lib/duration';
	import TimseriesLine, { type LineOrder } from '../molecules/TimseriesLine.svelte';
	import { matchMetric, textColors } from '$lib/colors';
	import { untrack } from 'svelte';

	export interface Metric {
		name: string;
		unit: string;
		values: Array<number | null>;
	}

	export interface TimeseriesChartProps {
		time: number[];
		distance?: (number | null)[];
		metrics: Array<Metric>;
		width: number;
		height: number;
	}

	let { time, metrics, height, width, distance }: TimeseriesChartProps = $props();
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
		untrack(() => {
			// Use of untrack to prevent svelte infinite loop, as zooming updates zoomedDomain
			d3.select(svgElement).call(zoom.translateTo, zoomedXScale(zoomedDomain[0] - x_min), 0, [
				timeScale[0],
				0
			]);
			d3.select(svgElement).call(
				zoom.scaleTo,
				(x_max - x_min) / (zoomedDomain[1] - zoomedDomain[0])
			);
		});
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
				name: matchMetric(metric.name),
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

	// We intentionally capture initial values for zoom comparison
	// svelte-ignore state_referenced_locally
	let zoomedDomain = $state.raw([x_min, x_max]);
	let zoomedIn = $derived(zoomedDomain[0] !== x_min || zoomedDomain[1] !== x_max);

	function zoomed(event: d3.D3ZoomEvent<SVGElement, any>) {
		zoomedDomain = [
			xScale.invert(event.transform.invertX(xScale.range()[0])),
			xScale.invert(event.transform.invertX(xScale.range()[1]))
		];
		zoomedXScale = event.transform.rescaleX(xScale);
		d3.select(gx).call(xAxis, zoomedXScale);
	}

	const resetZoom = () => {
		d3.select(svgElement).call(zoom.scaleTo, 1);
		zoomedIn = false;
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
		let offset = tooltipXOffset === undefined ? zoomedXScale.range()[0] : tooltipXOffset;

		const values = metricsProps.map((metric) => {
			const nearestValue =
				metric.values[bisector.center(metric.values, zoomedXScale.invert(offset!))];
			return {
				metric: metric.name,
				value: nearestValue[1],
				unit: metric.unit,
				order: metric.order
			};
		});

		const nearestDistance =
			distance === undefined ? undefined : distance.at(zoomedXScale.invert(offset));

		return { time: zoomedXScale.invert(offset!), distance: nearestDistance, values };
	});

	let smoothing = $state(5);
</script>

<!-- <input type="range" min="1" max="30" bind:value={smoothing} class="range" /> -->
<p class="flex justify-center pt-2 text-xs sm:text-base">
	{#if zoomedIn}
		<button onclick={resetZoom}>🔄</button>
	{/if}
	<span class="px-1.5">
		⌚ {formatDuration(nearestValues.time)}
	</span>
	{#if nearestValues.distance !== undefined && nearestValues.distance !== null}
		<span class="px-1.5">
			📏 {nearestValues.distance.toFixed(2)} km
		</span>
	{/if}
	{#each nearestValues.values as value}
		<span class={`px-1.5 ${textColors[value.metric]}`}>
			{value.metric}: {value.value.toFixed(2)}
			{value.unit}
		</span>
	{/each}
</p>
<svg
	bind:this={svgElement}
	{width}
	{height}
	viewBox={`0 0 ${width} ${height}`}
	role="img"
	onmousemove={tooltipCallback}
	style="max-width: 100%; height: auto; display: block;"
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
			metric={props.name}
			{smoothing}
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
