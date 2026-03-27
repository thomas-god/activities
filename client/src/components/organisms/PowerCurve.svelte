<script lang="ts">
	import * as d3 from 'd3';

	interface Props {
		powerValues: (number | null)[];
		width: number;
		height: number;
	}

	let { powerValues, width, height }: Props = $props();

	const marginTop = 20;
	const marginRight = 20;
	const marginBottom = 28;
	const marginLeft = 48;

	// Fixed duration set in seconds — ensures curves are comparable across activities
	const FIXED_DURATIONS = [5, 10, 30, 60, 120, 300, 600, 1200, 1800, 3600, 7200, 3600 * 5];

	function computePowerCurve(values: (number | null)[]): [number, number][] {
		// Concatenate active (non-null) seconds into a single continuous stream.
		// Pauses are skipped so that MMP is computed over active riding time only,
		// and the full activity duration is always represented on the curve.
		const active = values.filter((v): v is number => v !== null);
		if (active.length === 0) return [];

		const result: [number, number][] = [];
		for (const d of FIXED_DURATIONS) {
			if (active.length < d) break;

			let sum = 0;
			for (let i = 0; i < d; i++) sum += active[i];
			let maxAvg = sum / d;

			for (let i = d; i < active.length; i++) {
				sum += active[i] - active[i - d];
				const avg = sum / d;
				if (avg > maxAvg) maxAvg = avg;
			}

			result.push([d, maxAvg]);
		}

		// Always add an explicit point for the full active duration so that a
		// 59-min ride doesn't stop at the 30-min marker.
		const lastFixed = result.length > 0 ? result[result.length - 1][0] : 0;
		if (active.length > lastFixed) {
			const totalSum = active.reduce((a, b) => a + b, 0);
			result.push([active.length, totalSum / active.length]);
		}

		return result;
	}

	let curveData = $derived(computePowerCurve(powerValues));

	let xScale = $derived(
		d3.scaleLog(
			[FIXED_DURATIONS.at(0)!, FIXED_DURATIONS.at(-1)!],
			[marginLeft, width - marginRight]
		)
	);

	let yScale = $derived.by(() => {
		const maxPower = d3.max(curveData, (d) => d[1]) ?? 100;
		return d3.scaleLinear([0, maxPower * 1.05], [height - marginBottom, marginTop]);
	});

	let areaPath = $derived.by(() => {
		if (curveData.length === 0) return '';
		const gen = d3
			.area<[number, number]>()
			.x((d) => xScale(d[0]))
			.y0(yScale(0))
			.y1((d) => yScale(d[1]))
			.curve(d3.curveCatmullRom.alpha(0.5));
		return gen(curveData) ?? '';
	});

	let linePath = $derived.by(() => {
		if (curveData.length === 0) return '';
		const gen = d3
			.line<[number, number]>()
			.x((d) => xScale(d[0]))
			.y((d) => yScale(d[1]))
			.curve(d3.curveCatmullRom.alpha(0.5));
		return gen(curveData) ?? '';
	});

	const formatTickDuration = (s: number): string => {
		if (s < 60) return `${s}s`;
		if (s < 3600) return `${s / 60}min`;
		return `${s / 3600}hr`;
	};

	const formatTooltipDuration = (s: number): string => {
		const h = Math.floor(s / 3600);
		const m = Math.floor((s % 3600) / 60);
		const sec = s % 60;
		if (h > 0) return `${h}h ${m.toString().padStart(2, '0')}m ${sec.toString().padStart(2, '0')}s`;
		if (m > 0) return `${m}m ${sec.toString().padStart(2, '0')}s`;
		return `${sec}s`;
	};

	let yTicks = $derived(yScale.ticks(5));

	// Tooltip
	let tooltipX = $state<number | undefined>(undefined);
	const bisector = d3.bisector<[number, number], number>((d) => d[0]);
	let tooltipData = $derived.by(() => {
		if (tooltipX === undefined || curveData.length === 0) return null;
		const duration = xScale.invert(tooltipX);
		const idx = Math.max(0, Math.min(bisector.center(curveData, duration), curveData.length - 1));
		return curveData[idx] ?? null;
	});

	const handleMouseMove = (e: MouseEvent) => {
		tooltipX = Math.min(Math.max(e.offsetX, marginLeft), width - marginRight);
	};

	const handleMouseLeave = () => {
		tooltipX = undefined;
	};
</script>

{#if curveData.length > 0}
	<div class="flex flex-wrap justify-center pt-2 text-xs sm:text-base">
		{#if tooltipData}
			<span class="px-1.5">Interval: {formatTooltipDuration(tooltipData[0])}</span>
			<span class="px-1.5 font-semibold text-power-chart">{Math.round(tooltipData[1])} W</span>
		{:else}
			<span class="invisible px-1.5">Interval: –</span>
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
		{#each yTicks as tick}
			<g transform="translate(0, {yScale(tick)})">
				<text
					x={marginLeft - 6}
					text-anchor="end"
					dominant-baseline="middle"
					font-size="10"
					class="fill-current opacity-60">{tick}W</text
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
		{#each FIXED_DURATIONS as tick}
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
				cx={xScale(tooltipData[0])}
				cy={yScale(tooltipData[1])}
				r="4"
				fill="var(--color-power-chart)"
			/>
		{/if}
	</svg>
{:else}
	<p class="py-4 text-center text-sm opacity-50">No power data available</p>
{/if}
