<!--
@component
Training metric's data and metadata are considered fixed: they're computed by the server, the client
cannot directly update them. Editing a training metric's definition or changing the date range
always involves a round-trip with the server.

Thus this chart's variables are mostly static: we don't `$derived` from fixed props like `data`,
`format`, `unit`, etc. This avoids some undefined behavior with d3.js where multiple `$derived`
would rerun on some unidentified conditions (up to several hundreds times, obviously tanking
performances).

The only real dynamic props we use `$derived` on are `width` and `height` to handle resizing.

If even with limited `$derived` performances are still bad, you can replace some of them with
plain `$state(/* mutation */)` and `$effect(() => /* mutation */)` to apparently break the
`$derived` reruns explosion. Use the browser performance tool to find which `$derived` to convert.

The `state_referenced_locally` warnings are left ON so that we have to explicitly add
`// svelte-ignore state_referenced_locally` comments to variables we consider fixed, and avoid
forgetting `$derived` on actual dynamic variables.

The same design decision applies to other types of chart in this module.
-->
<script lang="ts">
	import { dayjs } from '$lib/duration';
	import * as d3 from 'd3';
	import { asOption, isSome, map, none, unwrapOr, type Option } from '$lib/Options';
	import {
		buildAbsoluteTimeFormatter,
		buildRelativeTimeFormatter,
		formatTooltipValue,
		parseMetricIntoPoints,
		type DisplayMode
	} from '.';
	import type { TrainingMetricGranularity } from '$lib/trainingMetric';
	import { expectedBinsForDomain, type TimeDomain } from '$ui/training_metrics';

	let {
		data,
		width,
		height,
		target,
		average,
		format,
		granularity,
		unit,
		yMaxValue = none(),
		timeDomain = none(),
		displayMode = 'absolute'
	}: {
		data: Record<string, Record<string, number | null>>;
		width: number;
		height: number;
		unit: string;
		format: 'number';
		average: Option<number>;
		target: Option<number>;
		yMaxValue?: Option<number>;
		timeDomain?: TimeDomain;
		granularity: TrainingMetricGranularity;
		displayMode?: DisplayMode;
	} = $props();

	let marginTop = 20;
	let marginRight = 20;
	let marginBottom = 20;
	let marginLeft = 55;

	let gx: SVGGElement;
	let gy: SVGGElement;
	let gyGrid: SVGGElement;
	let gPaths: SVGGElement;
	let svgElement: SVGElement;

	// A missing/null group value stacks as 0 so all bands share the same time grid.
	// '__ts' carries the timestamp so we can recover the time string from stack data.
	type StackRow = Record<string, number>;

	// svelte-ignore state_referenced_locally
	const { points, times: metricTimes } = parseMetricIntoPoints(data, timeDomain, {
		replaceNullValues: false
	});
	// svelte-ignore state_referenced_locally
	/* eslint-disable svelte/prefer-writable-derived */
	let domainTimes = $state(expectedBinsForDomain(timeDomain, asOption(granularity), dayjs()));
	$effect(() => {
		domainTimes = expectedBinsForDomain(timeDomain, asOption(granularity), dayjs());
	});

	let times = $derived(unwrapOr(domainTimes, metricTimes));

	const groups = [...new Set(points.map((v) => v.group))].sort();

	// svelte-ignore state_referenced_locally
	let timeByTs = $state(new Map(times.map((time) => [dayjs(time).unix(), time])));
	$effect(() => {
		timeByTs = new Map(times.map((time) => [dayjs(time).unix(), time]));
	});

	let rows = $derived.by(() => {
		/* eslint-disable svelte/prefer-svelte-reactivity */
		const valueByGroupAndTime = new Map<string, number>();
		for (const v of points) {
			valueByGroupAndTime.set(`${v.group}\u0000${v.time}`, v.value);
		}
		return times.map((time) => {
			const row: StackRow = { __ts: dayjs(time).unix() };
			for (const group of groups) {
				row[group] = valueByGroupAndTime.get(`${group}\u0000${time}`) ?? 0;
			}
			return row;
		});
	});

	let series = $derived(
		d3
			.stack<StackRow>()
			.keys(groups)
			.value((d, key) => d[key] ?? 0)(rows)
	);

	const yAxisDefaultTickValues = (): number[] => {
		if (points.length === 0) {
			return [];
		}

		return d3.ticks(0, maxValue, 6);
	};

	const yAxisTickValues = (): number[] => {
		if (points.length === 0) {
			return [];
		}

		return yAxisDefaultTickValues();
	};

	// svelte-ignore state_referenced_locally
	const maxValue = isSome(yMaxValue)
		? yMaxValue.value
		: Math.max(
				d3.max(series, (s) => d3.max(s, (d) => d[1]) ?? 0) ?? 0,
				unwrapOr(target, Number.NEGATIVE_INFINITY)
			);

	let maxTimeTicks = $derived(Math.min(8, Math.floor(width / 70)));

	// Point scales ignore axis .ticks(n), so select tick times explicitly.
	let xTickValues = $derived.by(() => {
		if (times.length === 0) {
			return [];
		}
		const step = Math.max(1, Math.ceil(times.length / maxTimeTicks));
		return times.filter((_, i) => i % step === 0 || i === times.length - 1);
	});

	// svelte-ignore state_referenced_locally
	let xAxis = $state(
		d3
			.scalePoint()
			.domain(times)
			.range([marginLeft, width - marginRight])
	);
	$effect(() => {
		xAxis.domain(times).range([marginLeft, width - marginRight]);
	});

	let yAxis = $derived(
		d3
			.scaleLinear()
			.domain([0, maxValue * 1.1])
			.rangeRound([height - marginBottom, marginTop])
	);

	let area = $derived(
		d3
			.area<d3.SeriesPoint<StackRow>>()
			.x((d) => xAxis(timeByTs.get(d.data.__ts)!)!)
			.y0((d) => yAxis(d[0]))
			.y1((d) => yAxis(d[1]))
			.curve(d3.curveMonotoneX)
	);

	const color = d3.scaleOrdinal(d3.schemeCategory10).domain(groups);

	// svelte-ignore state_referenced_locally
	const absoluteTimeFormatter = buildAbsoluteTimeFormatter(granularity);
	let relativeTimeFormatter = $derived(buildRelativeTimeFormatter(times, granularity));

	const yAxisTickFormatter = () => {
		return (value: d3.NumberValue, _idx: number) =>
			`${value.toString()} ${unit === 'activities' ? '' : unit}`;
	};

	let averageLineY = $derived(map(average, (avg) => yAxis(avg)));
	let averageLegendY = $derived(
		map(averageLineY, (avg) =>
			Math.max(marginTop + 12, Math.min(height - marginBottom - 4, avg - 6))
		)
	);
	// svelte-ignore state_referenced_locally
	const averageLegend = map(average, (avg) => `Average = ${formatTooltipValue(avg, format, unit)}`);

	let targetLineY = $derived(map(target, (t) => yAxis(t)));
	let targetLegendY = $derived(
		map(targetLineY, (t) => Math.max(marginTop + 12, Math.min(height - marginBottom - 4, t - 6)))
	);
	// svelte-ignore state_referenced_locally
	const targetLegend = map(target, (t) => `Target = ${formatTooltipValue(t, format, unit)}`);

	// Tooltip state
	let tooltip = $state<{
		visible: boolean;
		x: number;
		y: number;
		showBelow: boolean;
		time: string;
		group: string;
		value: number;
		total: number;
	}>({
		visible: false,
		x: 0,
		y: 0,
		showBelow: false,
		time: '',
		group: '',
		value: 0,
		total: 0
	});

	// Hide tooltip on scroll
	const handleScroll = () => {
		if (tooltip.visible) {
			tooltip = { ...tooltip, visible: false };
		}
	};

	$effect(() => {
		// Add scroll listener to hide tooltip when scrolling
		window.addEventListener('scroll', handleScroll, true); // Use capture phase to catch all scroll events

		return () => {
			window.removeEventListener('scroll', handleScroll, true);
		};
	});

	const showTooltipAt = (
		xPos: number,
		yPos: number,
		time: string,
		group: string,
		value: number,
		total: number
	) => {
		// Check if there's enough space above the bar for tooltip (need ~100px)
		const tooltipHeight = 60;
		const spaceAbove = yPos - marginTop;
		const showBelow = spaceAbove < tooltipHeight;

		// Check horizontal space for tooltip (tooltip width is 200px)
		const tooltipWidth = 100;
		const tooltipHalfWidth = tooltipWidth / 2;
		const spaceLeft = xPos - marginLeft;
		const spaceRight = width - marginRight - xPos;

		// Determine tooltip x position
		let tooltipX = xPos - tooltipHalfWidth; // Center by default
		if (spaceLeft < tooltipHalfWidth) {
			// Not enough space on the left, align to left edge
			tooltipX += tooltipHalfWidth;
		} else if (spaceRight < tooltipHalfWidth) {
			// Not enough space on the right, align to right edge
			tooltipX -= tooltipHalfWidth;
		}

		tooltip = {
			visible: true,
			x: tooltipX,
			y: yPos,
			showBelow,
			time,
			group,
			value,
			total
		};
	};

	const nearestTime = (mouseX: number): string | undefined => {
		let best: string | undefined;
		let bestDist = Number.POSITIVE_INFINITY;
		for (const time of times) {
			const dist = Math.abs((xAxis(time) ?? Number.POSITIVE_INFINITY) - mouseX);
			if (dist < bestDist) {
				bestDist = dist;
				best = time;
			}
		}
		return best;
	};

	$effect(() => {
		d3.select(gPaths).call((sel) =>
			sel
				.selectAll<SVGPathElement, d3.Series<StackRow, string>>('path')
				.data(series, (d) => d.key)
				.join('path')
				.attr('fill', (d) => color(d.key))
				.attr('fill-opacity', 0.9)
				.attr('stroke', 'none')
				.attr('d', area)
				.on('mousemove', function (event: MouseEvent, s: d3.Series<StackRow, string>) {
					const [mouseX] = d3.pointer(event, svgElement);
					const time = nearestTime(mouseX);
					const point =
						time === undefined ? undefined : s.find((d) => d.data.__ts === dayjs(time).unix());
					if (time === undefined || point === undefined) {
						return;
					}

					const xPos = xAxis(time)!;
					const yPos = yAxis((point[0] + point[1]) / 2);

					// Total = sum of every group's value for this time (full stack height)
					const row = rows.find((r) => r.__ts === point.data.__ts);
					const total = row === undefined ? point[1] : d3.sum(groups, (g) => row[g] ?? 0);

					showTooltipAt(xPos, yPos, time, s.key, point[1] - point[0], total);

					// Highlight the band
					d3.select(this).attr('fill-opacity', 1);
				})
				.on('mouseleave', function (_event: MouseEvent) {
					// Hide tooltip
					tooltip = { ...tooltip, visible: false };

					// Remove highlight
					d3.select(this).attr('fill-opacity', 0.7);
				})
		);

		d3.select(gx).call((sel) => {
			sel.call(
				d3
					.axisBottom(xAxis)
					.tickFormat(displayMode === 'absolute' ? absoluteTimeFormatter : relativeTimeFormatter)
					.tickValues(xTickValues)
			);
		});

		d3.select(gy).call((sel) =>
			sel.call(d3.axisLeft(yAxis).tickFormat(yAxisTickFormatter()).tickValues(yAxisTickValues()))
		);

		const yValues = yAxisTickValues() === null ? yAxis.ticks() : yAxisTickValues();

		d3.select(gyGrid).call((sel) =>
			sel
				.selectAll('line')
				.data(yValues)
				.join('line')
				.attr('x1', 0)
				.attr('x2', width - marginRight - marginLeft)
				.attr('y1', (tickValue) => yAxis(tickValue))
				.attr('y2', (tickValue) => yAxis(tickValue))
		);
	});
</script>

<div class="flex flex-col gap-2">
	<svg
		{width}
		{height}
		viewBox={`0 0 ${width} ${height}`}
		role="img"
		class="h-full w-full p-1 select-none"
		bind:this={svgElement}
	>
		<g
			bind:this={gyGrid}
			transform="translate({marginLeft} 0)"
			stroke="currentColor"
			opacity="0.3"
		/>

		<g bind:this={gPaths} />

		{#if isSome(average)}
			<line
				x1={marginLeft}
				x2={width - marginRight}
				y1={unwrapOr(averageLineY, 0)}
				y2={unwrapOr(averageLineY, 0)}
				stroke="currentColor"
				stroke-width="1.5"
				opacity="0.8"
			/>
			<text
				x={marginLeft + 4}
				y={unwrapOr(averageLegendY, 0)}
				class="fill-current text-xs"
				style="paint-order: stroke; stroke: var(--color-base-100); stroke-width: 3px;"
			>
				{unwrapOr(averageLegend, '')}
			</text>
		{/if}

		{#if isSome(target)}
			<line
				x1={marginLeft}
				x2={width - marginRight}
				y1={unwrapOr(targetLineY, 0)}
				y2={unwrapOr(targetLineY, 0)}
				stroke="currentColor"
				stroke-width="1.5"
				stroke-dasharray="4 4"
				opacity="0.8"
			/>
			<text
				x={width - marginRight - 4}
				y={unwrapOr(targetLegendY, 0)}
				text-anchor="end"
				class="fill-current text-xs"
				style="paint-order: stroke; stroke: var(--color-base-100); stroke-width: 3px;"
			>
				{unwrapOr(targetLegend, '')}
			</text>
		{/if}

		<g bind:this={gx} transform="translate(0 {height - marginBottom})" />
		<g bind:this={gy} transform="translate({marginLeft} 0)" />

		<!-- Tooltip inside SVG -->
		{#if tooltip.visible}
			<foreignObject
				x={Math.round(tooltip.x)}
				y={tooltip.showBelow ? Math.round(tooltip.y) + 10 : Math.round(tooltip.y) - 90}
				width="200"
				height="100"
				class="pointer-events-none overflow-visible"
			>
				<div xmlns="http://www.w3.org/1999/xhtml" class="fixed">
					<div class="rounded-box bg-base-300 px-3 py-2 text-sm shadow-lg">
						<div class="flex flex-col gap-1">
							<div class="font-semibold">
								{displayMode === 'absolute'
									? absoluteTimeFormatter(tooltip.time, 0)
									: relativeTimeFormatter(tooltip.time, 0)}
								{#if displayMode === 'relative'}
									<span class="text-xs font-light italic">
										•
										{absoluteTimeFormatter(tooltip.time, 0)}
									</span>
								{/if}
							</div>
							<div class="text-xs">
								<span class="font-semibold">{tooltip.group}</span>:
								{formatTooltipValue(tooltip.value, format, unit)}
							</div>
							<div class="text-xs">
								<span class="font-semibold">Total</span>:
								{formatTooltipValue(tooltip.total, format, unit)}
							</div>
						</div>
					</div>
				</div>
			</foreignObject>
		{/if}
	</svg>
</div>
