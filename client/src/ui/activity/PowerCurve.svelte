<script lang="ts">
	import * as d3 from 'd3';
	import { isSome, none, type Option } from '$lib/Options';

	interface Props {
		curveValues: (number | null)[];
		/** Best values over the 12 weeks before the activity, for comparison
		 *  (dashed line). Empty when there is no comparable curve. */
		bestCurveValues?: (number | null)[];
		activityDuration?: number;
		averageValue?: number | null;
		width: number;
		height: number;
		weight?: Option<number>;
		/** Unit of the curve values, used for formatting (defaults to power). */
		unit?: string;
	}

	let {
		curveValues,
		bestCurveValues = [],
		activityDuration = undefined,
		averageValue = null,
		width,
		height,
		weight = none(),
		unit = 'W'
	}: Props = $props();

	const marginTop = 20;
	const marginRight = 20;
	const marginBottom = 28;
	const marginLeft = 48;

	// Fixed duration set in seconds — ensures curves are comparable across activities
	const FIXED_DURATIONS = [5, 10, 30, 60, 120, 300, 600, 1200, 1800, 3600, 7200, 3600 * 5];

	type Mode = 'absolute' | 'relative';

	let mode = $state<Mode>('absolute');

	let hasWeight = $derived(isSome(weight));
	let effectiveMode = $derived(hasWeight ? mode : 'absolute');

	interface CurvePoint {
		duration: number;
		value: number;
		fromStats: boolean;
	}

	interface BestCurvePoint {
		duration: number;
		value: number;
	}

	// Duration/value pairs from the server-computed curve (null values skipped)
	let curveData = $derived.by(() => {
		const points: CurvePoint[] = FIXED_DURATIONS.map((d, i): CurvePoint | null => {
			const value = curveValues[i];
			return value === null || value === undefined
				? null
				: { duration: d, value, fromStats: false };
		}).filter((point): point is CurvePoint => point !== null);

		// Always add an explicit point for the full activity duration so that a
		// 59-min ride doesn't stop at the 30-min marker. The average value comes
		// from the already computed activity stats.
		// There's some edge cases for which the average value for the activity's duration is
		// greater than the previous point on the curve, making the curve not decreasing
		// (e.g. [10,0,10] -> avg(2) = (10+0)/2 = 5 but avg(3) = (10+0+10)/3 = 6.66 > avg(2))
		const lastFixed = points.length > 0 ? points[points.length - 1].duration : 0;
		if (
			activityDuration !== undefined &&
			averageValue !== null &&
			averageValue !== undefined &&
			activityDuration > lastFixed
		) {
			points.push({ duration: activityDuration, value: averageValue, fromStats: true });
		}

		return points;
	});

	let displayedData = $derived(
		effectiveMode === 'relative' && isSome(weight)
			? curveData.map((point) => ({ ...point, value: point.value / weight.value }))
			: curveData
	);

	// Best values are always defined on the fixed durations only, and the average
	// is unknown for them, so there is no stats-based extension point.
	let bestCurveData = $derived.by(() => {
		const points: BestCurvePoint[] = FIXED_DURATIONS.map((d, i): BestCurvePoint | null => {
			const value = bestCurveValues[i];
			return value === null || value === undefined ? null : { duration: d, value };
		}).filter((point): point is BestCurvePoint => point !== null);
		return points;
	});

	let hasBestCurve = $derived(bestCurveData.length > 0);

	// Same caveat as for the activity curve: the best curve values are divided by
	// the athlete's current weight, while past activities might have been recorded
	// with a different weight.
	let displayedBestData = $derived(
		effectiveMode === 'relative' && isSome(weight)
			? bestCurveData.map((point) => ({ ...point, value: point.value / weight.value }))
			: bestCurveData
	);

	let xScale = $derived(
		d3.scaleLog(
			[FIXED_DURATIONS.at(0)!, FIXED_DURATIONS.at(-1)!],
			[marginLeft, width - marginRight]
		)
	);

	let yScale = $derived.by(() => {
		const maxValue = d3.max([...displayedData, ...displayedBestData], (d) => d.value) ?? 100;
		return d3.scaleLinear([0, maxValue * 1.05], [height - marginBottom, marginTop]);
	});

	let bestLinePath = $derived.by(() => {
		if (displayedBestData.length === 0) return '';
		const gen = d3
			.line<BestCurvePoint>()
			.x((d) => xScale(d.duration))
			.y((d) => yScale(d.value))
			.curve(d3.curveCatmullRom.alpha(0.5));
		return gen(displayedBestData) ?? '';
	});

	let areaPath = $derived.by(() => {
		if (displayedData.length === 0) return '';
		const gen = d3
			.area<CurvePoint>()
			.x((d) => xScale(d.duration))
			.y0(yScale(0))
			.y1((d) => yScale(d.value))
			.curve(d3.curveCatmullRom.alpha(0.5));
		return gen(displayedData) ?? '';
	});

	let linePath = $derived.by(() => {
		if (displayedData.length === 0) return '';
		const gen = d3
			.line<CurvePoint>()
			.x((d) => xScale(d.duration))
			.y((d) => yScale(d.value))
			.curve(d3.curveCatmullRom.alpha(0.5));
		return gen(displayedData) ?? '';
	});

	const formatTickDuration = (s: number): string => {
		if (s < 60) return `${s}s`;
		if (s < 3600) return `${s / 60}min`;
		return `${s / 3600}hr`;
	};

	const formatTooltipDuration = (point: CurvePoint): string => {
		// The stats-based point uses the raw (fractional) activity duration, so
		// truncate it to whole minutes to avoid sub-second noise.
		if (point.fromStats) {
			const totalMinutes = Math.floor(point.duration / 60);
			const h = Math.floor(totalMinutes / 60);
			const m = totalMinutes % 60;
			return h > 0 ? `${h}h ${m.toString().padStart(2, '0')}m` : `${m}m`;
		}

		const s = point.duration;
		const h = Math.floor(s / 3600);
		const m = Math.floor((s % 3600) / 60);
		const sec = s % 60;
		if (h > 0) return `${h}h`;
		if (m > 0) return `${m}m`;
		return `${sec}s`;
	};

	const formatPower = (v: number): string => {
		if (effectiveMode === 'relative') return `${(Math.round(v * 10) / 10).toString()} W/kg`;
		return `${Math.round(v).toString()} ${unit}`;
	};

	const formatTickValue = (v: number): string => {
		if (effectiveMode === 'relative') return `${+v.toFixed(1)}W/kg`;
		// Preserve the compact `250W` style for power, keep a space for other units.
		return unit === 'W' ? `${v}W` : `${v} ${unit}`;
	};

	let yTicks = $derived(yScale.ticks(5));

	// Tooltip
	let tooltipX = $state<number | undefined>(undefined);
	const bisector = d3.bisector<CurvePoint, number>((d) => d.duration);
	let tooltipData = $derived.by(() => {
		if (tooltipX === undefined || displayedData.length === 0) return null;
		const duration = xScale.invert(tooltipX);
		const idx = Math.max(
			0,
			Math.min(bisector.center(displayedData, duration), displayedData.length - 1)
		);
		return displayedData[idx] ?? null;
	});

	// The best curve only covers the fixed durations, so it is only shown when the
	// hovered interval matches one of them (not the stats-based extension point).
	let tooltipBestData = $derived.by(() => {
		if (tooltipData === null || tooltipData.fromStats) return null;
		return displayedBestData.find((point) => point.duration === tooltipData!.duration) ?? null;
	});

	const handleMouseMove = (e: MouseEvent) => {
		tooltipX = Math.min(Math.max(e.offsetX, marginLeft), width - marginRight);
	};

	const handleMouseLeave = () => {
		tooltipX = undefined;
	};
</script>

{#if displayedData.length > 0}
	<div class="flex flex-wrap items-center justify-center pt-2 text-xs sm:text-base">
		{#if tooltipData}
			<span class="px-1.5">Interval: {formatTooltipDuration(tooltipData)}</span>
			<span class="text-power-chart px-1.5 font-semibold">{formatPower(tooltipData.value)}</span>
			{#if tooltipBestData}
				(12-week best
				<span class="text-best-power-chart px-1.5 font-semibold">
					{formatPower(tooltipBestData.value)}
				</span>
				)
			{/if}
		{:else}
			<span class="invisible px-1.5">Interval: –</span>
		{/if}
		{#if hasWeight}
			<div class="join ml-auto" role="group" aria-label="Power curve unit">
				<button
					class="btn join-item btn-xs"
					class:btn-primary={effectiveMode === 'absolute'}
					class:btn-outline={effectiveMode === 'relative'}
					onclick={() => (mode = 'absolute')}>W</button
				>
				<button
					class="btn join-item btn-xs"
					class:btn-primary={effectiveMode === 'relative'}
					class:btn-outline={effectiveMode === 'absolute'}
					onclick={() => (mode = 'relative')}>W/kg</button
				>
			</div>
		{/if}
	</div>
	<svg
		{width}
		{height}
		viewBox="0 0 {width} {height}"
		role="img"
		onmousemove={handleMouseMove}
		onmouseleave={handleMouseLeave}
		style="max-width: 100%; height: auto; display: block;"
	>
		<defs>
			<linearGradient id="power-curve-gradient" x1="0" x2="0" y1="0" y2="1">
				<stop offset="0%" stop-color="var(--color-power-chart)" stop-opacity="0.5" />
				<stop offset="100%" stop-color="var(--color-power-chart)" stop-opacity="0.1" />
			</linearGradient>
			<clipPath id="power-curve-clip">
				<rect
					x={marginLeft}
					y={marginTop}
					width={width - marginLeft - marginRight}
					height={height - marginTop - marginBottom}
				/>
			</clipPath>
		</defs>

		<!-- Horizontal grid lines + Y axis labels -->
		{#each yTicks as tick (tick)}
			<g transform="translate(0, {yScale(tick)})">
				<text
					x={marginLeft - 6}
					text-anchor="end"
					dominant-baseline="middle"
					font-size="10"
					class="fill-current opacity-60">{formatTickValue(tick)}</text
				>
				<line
					x1={marginLeft}
					x2={width - marginRight}
					class="stroke-current"
					stroke-opacity="0.1"
				/>
			</g>
		{/each}

		<!-- Area fill -->
		<path d={areaPath} fill="url(#power-curve-gradient)" clip-path="url(#power-curve-clip)" />

		<!-- Curve line -->
		<path
			d={linePath}
			fill="none"
			stroke="var(--color-power-chart)"
			stroke-width="1.5"
			clip-path="url(#power-curve-clip)"
		/>

		<!-- Best curve line (12-week comparison) -->
		{#if hasBestCurve}
			<path
				d={bestLinePath}
				fill="none"
				stroke="var(--color-best-power-chart)"
				stroke-width="1.5"
				stroke-dasharray="5,3"
				clip-path="url(#power-curve-clip)"
			/>
		{/if}

		<!-- X axis baseline -->
		<line
			x1={marginLeft}
			x2={width - marginRight}
			y1={height - marginBottom}
			y2={height - marginBottom}
			class="stroke-current"
			stroke-opacity="0.2"
		/>

		<!-- X axis ticks and labels -->
		{#each FIXED_DURATIONS as tick (tick)}
			<g transform="translate({xScale(tick)}, {height - marginBottom})">
				<line y2="4" class="stroke-current" stroke-opacity="0.4" />
				<text
					y="16"
					text-anchor="middle"
					font-size="10"
					class="fill-current"
					opacity="0.6"
					transform="rotate(-35) translate(-7, -2)"
				>
					{formatTickDuration(tick)}
				</text>
			</g>
		{/each}

		<!-- Tooltip cursor line and dot -->
		{#if tooltipX !== undefined && tooltipData}
			<line
				x1={tooltipX}
				x2={tooltipX}
				y1={marginTop}
				y2={height - marginBottom}
				class="stroke-current"
				stroke-dasharray="3,2"
				stroke-opacity="0.5"
			/>
			<circle
				cx={xScale(tooltipData.duration)}
				cy={yScale(tooltipData.value)}
				r="4"
				fill="var(--color-power-chart)"
			/>
		{/if}
	</svg>
	{#if hasBestCurve}
		<div class="flex items-center justify-center gap-4 pb-1 text-xs opacity-80">
			<span class="flex items-center gap-1.5">
				<span class="inline-block h-0.5 w-4 rounded" style="background: var(--color-power-chart)"
				></span>
				This activity
			</span>
			<span class="flex items-center gap-1.5">
				<span
					class="inline-block h-0.5 w-4"
					style="
						background-image: linear-gradient(
							to right,
							var(--color-best-power-chart) 50%,
							transparent 50%
						);
						background-size: 6px 100%;
					"
				></span>
				12-week best
			</span>
		</div>
	{/if}
{:else}
	<p class="py-4 text-center text-sm opacity-50">No power data available</p>
{/if}
