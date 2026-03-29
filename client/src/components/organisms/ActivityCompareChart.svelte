<script lang="ts">
	import * as d3 from 'd3';
	import { formatDuration } from '$lib/duration';
	import type { ActivityWithTimeseries } from '$lib/api';
	import { paceInSecondToString } from '$lib/speed';

	interface Props {
		activities: ActivityWithTimeseries[];
		metric: string;
		width: number;
		height: number;
	}

	let { activities, metric, width, height }: Props = $props();

	const marginTop = 16;
	const marginBottom = 28;
	const marginLeft = 48;
	const marginRight = 12;

	let coeff = $derived.by(() => {
		if (metric === 'Pace') {
			return 1000; // s:m -> s:km
		}
		return 1;
	});

	let fmtYTick = $derived.by(() => {
		if (metric === 'Pace') {
			return (v: number) => paceInSecondToString(v);
		}
		if (metric === 'Speed') {
			return (v: number) => v.toFixed(2);
		}
		return (v: number) => v.toFixed(0);
	});

	// Stable unique clip-path id per component instance
	const clipId = `compare-clip-${Math.random().toString(36).slice(2)}`;

	// D3 categorical color palette – up to 10 activities
	const palette = d3.schemeTableau10 as readonly string[];

	type ActivitySeries = {
		activity: ActivityWithTimeseries;
		values: [number, number][]; // [elapsed_seconds, metric_value]
		color: string;
		label: string;
	};

	let series = $derived.by((): ActivitySeries[] => {
		const result: ActivitySeries[] = [];
		let colorIdx = 0;
		for (const activity of activities) {
			const metricData = activity.timeseries.metrics[metric];
			if (!metricData) continue;
			const { active_time } = activity.timeseries;
			// Find the first non-null active_time to use as the origin
			const at0 = active_time.find((t) => t !== null) ?? 0;
			const values: [number, number][] = [];
			for (let i = 0; i < active_time.length; i++) {
				const at = active_time[i];
				const v = metricData.values[i];
				// Skip paused samples (null active_time) and missing metric values
				if (at !== null && v !== null) {
					values.push([at - at0, v]);
				}
			}
			if (values.length === 0) continue;
			result.push({
				activity,
				values: values.map(([t, v]) => [t, v * coeff]),
				color: palette[colorIdx % palette.length],
				label: activity.name ?? activity.start_time.slice(0, 10)
			});
			colorIdx++;
		}
		return result;
	});

	let unit = $derived.by(() => {
		const unit =
			activities.find((a) => a.timeseries.metrics[metric])?.timeseries.metrics[metric]?.unit ?? '';
		if (metric === 'Pace') {
			return 'min:km'; // s:m -> s:km
		}
		return unit;
	});

	let innerWidth = $derived(width - marginLeft - marginRight);
	let innerHeight = $derived(height - marginTop - marginBottom);

	let xMax = $derived.by(() => {
		let max = 0;
		for (const s of series) {
			const last = s.values.at(-1)?.[0] ?? 0;
			if (last > max) max = last;
		}
		return max || 1;
	});

	let xScale = $derived(d3.scaleLinear([0, xMax], [marginLeft, marginLeft + innerWidth]));

	let yExtent = $derived.by((): [number, number] => {
		let min = Infinity;
		let max = -Infinity;
		for (const s of series) {
			for (const [, v] of s.values) {
				if (v < min) min = v;
				if (v > max) max = v;
			}
		}
		if (!isFinite(min)) return [0, 1];
		const pad = (max - min) * 0.05 || 1;
		return [min - pad, max + pad];
	});

	let yScale = $derived(d3.scaleLinear(yExtent, [marginTop + innerHeight, marginTop]));

	let gx: SVGGElement;
	let gy: SVGGElement;

	$effect(() => {
		d3.select(gx).call(
			d3
				.axisBottom(xScale)
				.ticks(Math.min(8, Math.floor(innerWidth / 70)))
				.tickFormat((v) => formatDuration(v.valueOf()))
		);
	});

	$effect(() => {
		d3.select(gy).call(
			d3
				.axisLeft(yScale)
				.ticks(5)
				.tickFormat((v, _idx) => fmtYTick(v.valueOf()))
		);
	});

	let linePaths = $derived.by(() => {
		const lineGen = d3
			.line<[number, number]>()
			.x(([t]) => xScale(t))
			.y(([, v]) => yScale(v));
		return series.map((s) => ({
			d: lineGen(s.values) ?? '',
			color: s.color,
			label: s.label
		}));
	});

	// Tooltip
	const bisector = d3.bisector<[number, number], number>((p) => p[0]);
	let mouseX: number | null = $state(null);

	let tooltipData = $derived.by(() => {
		if (mouseX === null) return null;
		const xValue = xScale.invert(mouseX);
		return series.map((s) => {
			const idx = bisector.center(s.values, xValue);
			const point = s.values[idx];
			return {
				color: s.color,
				label: s.label,
				xVal: point[0],
				yVal: point[1]
			};
		});
	});

	// Clamp tooltip horizontally so it stays within the chart
	let tooltipLeft = $derived.by(() => {
		if (mouseX === null) return 0;
		const tooltipWidth = 160;
		const rightEdge = marginLeft + innerWidth;
		if (mouseX + 12 + tooltipWidth > rightEdge) {
			return mouseX - 12 - tooltipWidth;
		}
		return mouseX + 12;
	});

	function onMouseMove(e: MouseEvent) {
		const svg = e.currentTarget as SVGSVGElement;
		const rect = svg.getBoundingClientRect();
		const x = e.clientX - rect.left;
		if (x >= marginLeft && x <= marginLeft + innerWidth) {
			mouseX = x;
		} else {
			mouseX = null;
		}
	}

	function onMouseLeave() {
		mouseX = null;
	}
</script>

<div class="relative">
	<svg
		{width}
		{height}
		onmousemove={onMouseMove}
		onmouseleave={onMouseLeave}
		role="img"
		aria-label="Activity comparison chart"
		style="cursor:crosshair"
	>
		<defs>
			<clipPath id={clipId}>
				<rect x={marginLeft} y={marginTop} width={innerWidth} height={innerHeight} />
			</clipPath>
		</defs>

		<!-- X axis -->
		<g bind:this={gx} transform="translate(0,{marginTop + innerHeight})" />

		<!-- Y axis + unit label -->
		<g bind:this={gy} transform="translate({marginLeft},0)" />
		{#if unit}
			<text
				x={marginLeft - 6}
				y={marginTop - 4}
				text-anchor="end"
				font-size="11"
				class="fill-current opacity-60">{unit}</text
			>
		{/if}

		<!-- One line per activity -->
		{#each linePaths as { d, color }}
			<path {d} fill="none" stroke={color} stroke-width="1.5" clip-path="url(#{clipId})" />
		{/each}

		<!-- Crosshair + dots -->
		{#if mouseX !== null && tooltipData !== null}
			<line
				x1={mouseX}
				x2={mouseX}
				y1={marginTop}
				y2={marginTop + innerHeight}
				stroke="currentColor"
				stroke-width="1"
				opacity="0.35"
				stroke-dasharray="4 3"
			/>
			{#each tooltipData as row, i}
				{@const yPx = yScale(row.yVal)}
				{#if yPx >= marginTop && yPx <= marginTop + innerHeight}
					<circle cx={mouseX} cy={yPx} r="3.5" fill={row.color} stroke="white" stroke-width="1.5" />
				{/if}
			{/each}
		{/if}

		{#if series.length === 0}
			<text
				x={marginLeft + innerWidth / 2}
				y={marginTop + innerHeight / 2}
				text-anchor="middle"
				dominant-baseline="middle"
				font-size="13"
				class="fill-current opacity-50">No data for this metric</text
			>
		{/if}
	</svg>

	<!-- Floating tooltip -->
	{#if mouseX !== null && tooltipData !== null}
		<div
			class="pointer-events-none absolute top-0 z-10 min-w-[140px] rounded border border-base-300 bg-base-100 px-2 py-1.5 text-xs shadow"
			style="left:{tooltipLeft}px; top:{marginTop}px"
		>
			<p class="mb-1 text-base-content/60">{formatDuration(xScale.invert(mouseX))}</p>
			{#each tooltipData as row}
				<div class="flex items-center gap-1.5">
					<span
						class="inline-block h-2 w-2 shrink-0 rounded-full"
						style="background-color:{row.color}"
					></span>
					<span class="truncate opacity-75">{row.label}</span>
					<span class="ml-auto font-medium">{fmtYTick(row.yVal)} {unit}</span>
				</div>
			{/each}
		</div>
	{/if}
</div>

<!-- Legend -->
{#if linePaths.length > 0}
	<div class="mt-1 flex flex-wrap gap-x-4 gap-y-1 text-sm" style="padding-left:{marginLeft}px">
		{#each linePaths as { color, label }}
			<span class="flex items-center gap-1.5">
				<span class="inline-block h-0.5 w-5 rounded-full" style="background-color:{color}"></span>
				{label}
			</span>
		{/each}
	</div>
{/if}
