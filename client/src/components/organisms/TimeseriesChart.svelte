<script lang="ts">
	import * as d3 from 'd3';
	import { formatDuration } from '$lib/duration';
	import TimseriesLine, { type LineOrder } from '../molecules/TimseriesLine.svelte';
	import { formatMetricValue, matchMetric, textColors } from '$lib/colors';
	import { untrack } from 'svelte';
	import type { ActivityWithTimeseries } from '$lib/api';

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
		selectedLap: ActivityWithTimeseries['timeseries']['laps'][number] | null;
	}

	export function zoomToLap(lap: ActivityWithTimeseries['timeseries']['laps'][number] | null) {
		if (lap === null) {
			resetZoom();
			return;
		}

		let lapStart: number;
		let lapEnd: number;

		if (xAxisMode === 'time') {
			lapStart = lap.start;
			lapEnd = lap.end;
		} else {
			const startIdx = timeBisector.center(time, lap.start);
			const endIdx = timeBisector.center(time, lap.end);
			const startDist = distance?.[startIdx] ?? null;
			const endDist = distance?.[endIdx] ?? null;
			if (startDist === null || endDist === null) return;
			lapStart = startDist;
			lapEnd = endDist;
		}

		const lapDuration = lapEnd - lapStart;
		if (lapDuration <= 0) return;

		// Compute transform so lapStart maps to the left edge and lapEnd to the right edge
		const k = Math.min((x_max - x_min) / lapDuration, max_zoom);
		const tx = timeScale[0] - k * xScale(lapStart);
		d3.select(svgElement).call(zoom.transform, d3.zoomIdentity.translate(tx, 0).scale(k));
	}

	let { time, metrics, height, width, distance, selectedLap }: TimeseriesChartProps = $props();
	let marginTop = 20;
	let marginRight = 20;
	let marginBottom = 20;
	let axisWidth = 30;

	let hasDistance = $derived(distance !== undefined && distance.some((d) => d !== null));
	let xAxisMode: 'time' | 'distance' = $state('time');

	let numberOfMetrics = $derived(Math.min(metrics.length, 3));
	let timeScale = $derived.by(() => {
		if (numberOfMetrics === 1) {
			return [axisWidth, width - 0.5 * axisWidth];
		} else if (numberOfMetrics === 2) {
			return [axisWidth, width - axisWidth];
		} else {
			return [2 * axisWidth, width - axisWidth];
		}
	});
	let margins = $derived.by(() => {
		if (numberOfMetrics === 1) {
			return { left: axisWidth, right: 0.5 * axisWidth };
		} else if (numberOfMetrics === 2) {
			return { left: axisWidth, right: axisWidth };
		} else {
			return { left: axisWidth * 2, right: axisWidth };
		}
	});

	// Start by defining the x (time) scale and axis, common to all metrics
	let gx: SVGGElement;
	let x_min = $derived(
		xAxisMode === 'time' ? time[0] : (distance!.find((d) => d !== null) as number)
	);
	let x_max = $derived(
		xAxisMode === 'time' ? time[time.length - 1] : (distance!.findLast((d) => d !== null) as number)
	);
	let max_zoom = $derived(
		xAxisMode === 'time' ? Math.max(1, (x_max - x_min) / 60) : Math.max(1, (x_max - x_min) / 0.1)
	);
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
					.tickFormat((val, _) =>
						xAxisMode === 'time' ? formatDuration(val.valueOf()) : val.valueOf().toFixed(1)
					)
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
			const xArr = xAxisMode === 'time' ? time : (distance ?? time);
			let values = extractMetricValues(xArr, metric.values);

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

	const extractMetricValues = (xArr: (number | null)[], values: Array<number | null>) => {
		let v = [];
		for (let i = 0; i < xArr.length; i++) {
			let x = xArr[i];
			let val = values[i];
			if (x !== null && val !== null) {
				v.push([x, val]);
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

	const setXAxisMode = (mode: 'time' | 'distance') => {
		if (xAxisMode === mode) return;
		xAxisMode = mode;
		d3.select(svgElement).call(zoom.transform, d3.zoomIdentity);
	};

	$effect(() => {
		d3.select(svgElement).call(zoom);
	});

	let tooltipXOffset: number | undefined = $state(undefined);
	const tooltipCallback = (event: MouseEvent) => {
		tooltipXOffset = Math.min(Math.max(event.offsetX, margins.left), width - margins.right);
	};

	const bisector = d3.bisector<[number, number], number>((point) => point[0]);
	let dataLookup = $derived.by(() => {
		const arr: { time: number; distance: number | null; xVal: number }[] = [];
		for (let i = 0; i < time.length; i++) {
			const xVal = xAxisMode === 'time' ? time[i] : (distance?.[i] ?? null);
			if (xVal !== null) {
				arr.push({ time: time[i], distance: distance?.[i] ?? null, xVal });
			}
		}
		return arr;
	});
	const dataLookupBisector = d3.bisector<
		{ time: number; distance: number | null; xVal: number },
		number
	>((p) => p.xVal);
	let nearestValues = $derived.by(() => {
		let offset = tooltipXOffset === undefined ? zoomedXScale.range()[0] : tooltipXOffset;
		const xValue = zoomedXScale.invert(offset!);
		const nearest = dataLookup[dataLookupBisector.center(dataLookup, xValue)];

		const values = metricsProps.map((metric) => {
			const nearestValue = metric.values[bisector.center(metric.values, xValue)];
			return {
				metric: metric.name,
				value: nearestValue[1],
				unit: metric.unit,
				order: metric.order
			};
		});

		return {
			time: nearest?.time ?? xValue,
			distance: nearest?.distance ?? null,
			values
		};
	});

	let smoothing = $state(5);

	const timeBisector = d3.bisector<number, number>((t) => t);
	let lapXValues = $derived.by(() => {
		if (selectedLap === null) return null;
		if (xAxisMode === 'time') {
			return [selectedLap.start, selectedLap.end];
		} else {
			const startIdx = timeBisector.center(time, selectedLap.start);
			const endIdx = timeBisector.center(time, selectedLap.end);
			const startDist = distance?.[startIdx] ?? null;
			const endDist = distance?.[endIdx] ?? null;
			if (startDist === null || endDist === null) return null;
			return [startDist, endDist];
		}
	});
</script>

<!-- <input type="range" min="1" max="30" bind:value={smoothing} class="range" /> -->
<div class="flex flex-wrap justify-center pt-2 text-xs sm:text-base">
	<span class="inline-flex items-center gap-1 px-1.5">
		<img src="/icons/clock.svg" class="h-4 w-4" alt="Clock icon" />{formatDuration(
			nearestValues.time
		)}
	</span>
	{#if nearestValues.distance !== undefined && nearestValues.distance !== null}
		<span class="inline-flex items-center gap-1 px-1.5">
			<img
				src="/icons/distance.svg"
				class="h-4 w-4"
				alt="Distance icon"
			/>{nearestValues.distance.toFixed(2)} km
		</span>
	{/if}
	{#each nearestValues.values as value}
		<span class={`px-1.5 ${textColors[value.metric]}`}>
			{value.metric}: {formatMetricValue(value.value, value.metric)}
			{value.unit}
		</span>
	{/each}
</div>
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
			width={timeScale[1] - timeScale[0]}
			height={height - marginTop - marginBottom}
		/>
	</clipPath>

	{#if lapXValues !== null}
		{@const lapX0 = Math.max(zoomedXScale(lapXValues[0]), margins.left)}
		{@const lapX1 = Math.min(zoomedXScale(lapXValues[1]), width - margins.right)}
		{#if lapX1 > lapX0}
			<rect
				x={lapX0}
				y={marginTop}
				width={lapX1 - lapX0}
				height={height - marginTop - marginBottom}
				fill="currentColor"
				class="text-base-content/10"
			/>
		{/if}
	{/if}
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
{#if hasDistance || zoomedIn}
	<div class="flex justify-start gap-2 pt-2 pl-2 text-xs sm:text-sm">
		{#if hasDistance}
			<span class="inline-flex overflow-hidden rounded border border-base-300">
				<button
					onclick={() => setXAxisMode('time')}
					class="inline-flex items-center gap-1 px-1.5 py-0.5 {xAxisMode === 'time'
						? 'bg-base-300 font-semibold'
						: 'opacity-50'}"
					title="Show by time"
				>
					<img src="/icons/clock.svg" class="h-3.5 w-3.5" alt="" />Time
				</button>
				<button
					onclick={() => setXAxisMode('distance')}
					class="inline-flex items-center gap-1 px-1.5 py-0.5 {xAxisMode === 'distance'
						? 'bg-base-300 font-semibold'
						: 'opacity-50'}"
					title="Show by distance"
				>
					<img src="/icons/distance.svg" class="h-3.5 w-3.5" alt="" />Distance
				</button>
			</span>
		{/if}
		{#if zoomedIn}
			<button
				onclick={resetZoom}
				class="inline-flex items-center gap-1 opacity-70 hover:opacity-100"
				><img src="/icons/undo.svg" class="h-4 w-4" alt="Reset zoom" />Reset zoom</button
			>
		{/if}
	</div>
{/if}
