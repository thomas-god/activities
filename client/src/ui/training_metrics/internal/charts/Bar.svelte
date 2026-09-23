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
	import * as d3 from 'd3';
	import { dayjs, formatDurationCompactWithUnits } from '$lib/duration';
	import {
		displayGroupName,
		type TrainingMetricGranularity,
		type TrainingMetricGroupByClause
	} from '$lib/trainingMetric';
	import { asOption, isSome, map, none, unwrapOr, type Option } from '$lib/Options';
	import { paceInSecondToString } from '$lib/speed';
	import { expectedBinsForDomain, type TimeDomain } from '$ui/training_metrics';
	import {
		buildAbsoluteTimeFormatter,
		buildRelativeTimeFormatter,
		formatTooltipValue,
		getGroupColor,
		mapDomainToGranularity,
		parseMetricIntoPoints,
		type DisplayMode
	} from '.';
	import type { ChartHandle } from '$ui/training_metrics/TrainingMetricChart.svelte';

	export interface TimeseriesChartProps {
		values: Record<string, Record<string, number | null>>;
		width: number;
		height: number;
		unit: string;
		granularity: TrainingMetricGranularity;
		format: 'number' | 'duration' | 'pace';
		showGroup?: boolean;
		groupBy: Option<TrainingMetricGroupByClause>;
		stacked?: boolean;
		average: Option<number>;
		target: Option<number>;
		timeDomain?: TimeDomain;
		displayMode?: DisplayMode;
		yMaxValue?: Option<number>;
	}

	let {
		values,
		height,
		width,
		unit,
		granularity,
		format,
		groupBy,
		average,
		target,
		showGroup = true,
		stacked = true,
		timeDomain = none(),
		yMaxValue = none(),
		displayMode = 'absolute'
	}: TimeseriesChartProps = $props();
	let marginTop = 20;
	let marginRight = 20;
	let marginBottom = 20;
	let marginLeft = 55;

	let gx: SVGGElement;
	let gy: SVGGElement;
	let gyGrid: SVGGElement;
	let gBars: SVGGElement;
	let svgElement: SVGElement;

	type FormattedValue = { time: string; group: string; value: number };
	type SeriesDatum = [string, d3.InternMap<string, FormattedValue>];
	type StackedDataPoint = d3.SeriesPoint<SeriesDatum> & { key: string };

	// svelte-ignore state_referenced_locally
	const snappedDomain = mapDomainToGranularity(timeDomain, granularity);

	// svelte-ignore state_referenced_locally
	const { points, times: metricTimes } = parseMetricIntoPoints(values, snappedDomain);

	// svelte-ignore state_referenced_locally
	/* eslint-disable svelte/prefer-writable-derived */
	let domainTimes = $state(expectedBinsForDomain(timeDomain, asOption(granularity), dayjs()));
	$effect(() => {
		domainTimes = expectedBinsForDomain(timeDomain, asOption(granularity), dayjs());
	});

	let times = $derived(unwrapOr(domainTimes, metricTimes));

	// svelte-ignore state_referenced_locally
	const absoluteTimeFormatter = buildAbsoluteTimeFormatter(granularity);
	let relativeTimeFormatter = $derived(buildRelativeTimeFormatter(times, granularity));

	const yAxisTickFormatter = (() => {
		if (format === 'duration') {
			return (value: d3.NumberValue, _idx: number) => {
				return formatDurationCompactWithUnits(value.valueOf());
			};
		}
		if (format === 'pace') {
			return (value: d3.NumberValue, _idx: number) => {
				return paceInSecondToString(value.valueOf());
			};
		}

		return (value: d3.NumberValue, _idx: number) =>
			`${value.toString()} ${unit === 'activities' ? '' : unit}`;
	})();

	// Order of the groups inside the stacked series, sorted alphabetically ascending by
	// display name. First entry in the array is stacked at the bottom.
	const groups = Array.from(
		d3.union(points.map((v) => displayGroupName(v.group, groupBy)))
	).toSorted();

	// Create stacked series data structure
	// Each series represents one group (e.g., "Cycling", "Running")
	// d3.stack() transforms the data into layers for stacked bar visualization
	const series = d3
		.stack<SeriesDatum, string>()
		.keys(groups)
		.value(([, groupMap], groupKey) => groupMap.get(groupKey)!.value)(
		d3.index(
			points,
			(value) => value.time,
			(value) => displayGroupName(value.group, groupBy)
		)
	);
	// svelte-ignore state_referenced_locally
	const internalYMaxValue = Math.max(
		stacked
			? d3.max(series, (groupSeries) => d3.max(groupSeries, (point) => point[1]))!
			: d3.max(series, (groupSeries) => d3.max(groupSeries, (point) => point[1] - point[0]))!,
		unwrapOr(target, 0)
	);

	export function getYMax() {
		return internalYMaxValue;
	}
	export function getYMin() {
		return 0;
	}
	({ getYMax, getYMin }) satisfies ChartHandle;

	let maxValue = $derived(unwrapOr(yMaxValue, internalYMaxValue));

	const yAxisDefaultTickValues = $derived(d3.ticks(0, maxValue, 6));

	const yAxisTickValues = $derived.by(() => {
		if (points.length === 0) {
			return [];
		}
		if (format === 'duration') {
			const dt = 600;
			const maxDuration = maxValue;
			const maxDurationWithTarget = Math.max(maxDuration, unwrapOr(target, 0));
			const roundedUpMaxDuration = Math.ceil(maxDurationWithTarget / dt) * dt;
			const numberOfIntervals = Math.min(6, Math.floor(roundedUpMaxDuration / dt));
			const intervalDuration = Math.floor(roundedUpMaxDuration / numberOfIntervals / dt) * dt;

			if (intervalDuration !== 0) {
				const ticks = [];
				for (let i = 0; i < roundedUpMaxDuration; i += intervalDuration) {
					ticks.push(i);
				}
				return ticks;
			}
			return yAxisDefaultTickValues;
		}
		return yAxisDefaultTickValues;
	});

	// svelte-ignore state_referenced_locally
	let x = $state(
		d3
			.scaleBand()
			.domain(times)
			.padding(0.6)
			.range([marginLeft, width - marginRight])
	);

	$effect(() => {
		x.domain(times).range([marginLeft, width - marginRight]);
	});

	let y = $derived(
		d3
			.scaleLinear()
			.domain([0, maxValue])
			.rangeRound([height - marginBottom, marginTop])
	);

	let averageLineY = $derived(map(average, (avg) => y(avg)));
	let averageLegendY = $derived(
		map(averageLineY, (avg) =>
			Math.max(marginTop + 12, Math.min(height - marginBottom - 4, avg - 6))
		)
	);

	// svelte-ignore state_referenced_locally
	const averageLegend = map(average, (avg) => `Average = ${formatTooltipValue(avg, format, unit)}`);

	let targetLineY = $derived(map(target, (t) => y(t)));
	let targetLegendY = $derived(
		map(targetLineY, (t) => Math.max(marginTop + 12, Math.min(height - marginBottom - 4, t - 6)))
	);
	// svelte-ignore state_referenced_locally
	const targetLegend = map(target, (t) => `Target = ${formatTooltipValue(t, format, unit)}`);

	const colors = (() => {
		const scale = d3.scaleOrdinal(d3.schemeObservable10);
		const customScale = (groupName: string) => {
			const customColor = getGroupColor(groupName, groupBy);
			return customColor || scale(groupName);
		};
		return customScale;
	})();

	let xGroup = $derived(d3.scaleBand().domain(groups).range([0, x.bandwidth()]).padding(0.1));

	let maxTimeTicks = $derived(Math.min(8, Math.floor(width / 70)));

	let yValues = $derived(yAxisTickValues.length === 0 ? y.ticks() : yAxisTickValues);

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

	$effect(() => {
		d3.select(gBars).call((sel) =>
			sel
				.selectAll('g')
				.data(series)
				.join('g')
				.attr('fill', (groupSeries) => colors(groupSeries.key))
				.selectAll('rect')
				.data((groupSeries) =>
					groupSeries.map(
						// Attach the group key (e.g., "Cycling", "Running") to each data point
						// so we can identify which group each rectangle belongs to
						(stackedDataPoint): StackedDataPoint =>
							Object.assign(stackedDataPoint, { key: groupSeries.key })
					)
				)
				.join('rect')
				.attr('x', (stackedDataPoint) =>
					stacked
						? x(stackedDataPoint.data[0])!
						: x(stackedDataPoint.data[0])! + xGroup(stackedDataPoint.key)!
				)
				.attr('y', (stackedDataPoint) =>
					stacked ? y(stackedDataPoint[1]) : y(stackedDataPoint[1] - stackedDataPoint[0])
				)
				.attr('height', (stackedDataPoint) =>
					stacked
						? y(stackedDataPoint[0]) - y(stackedDataPoint[1])
						: y(0) - y(stackedDataPoint[1] - stackedDataPoint[0])
				)
				.attr('width', stacked ? x.bandwidth() : xGroup.bandwidth())
				.attr('stroke', 'none')
				.on('mouseenter', function (event: MouseEvent, stackedDataPoint) {
					// Show tooltip
					const rect = event.target as SVGRectElement;
					const value = stackedDataPoint[1] - stackedDataPoint[0]; // Height of this segment
					const time = stackedDataPoint.data[0];

					// Calculate total for this time across all groups
					const total = points.filter((v) => v.time === time).reduce((sum, v) => sum + v.value, 0);

					// Use SVG coordinates directly
					const xPos = stacked
						? x(stackedDataPoint.data[0])! + x.bandwidth() / 2
						: x(stackedDataPoint.data[0])! + xGroup(stackedDataPoint.key)! + xGroup.bandwidth() / 2;
					const yPos = stacked
						? y(stackedDataPoint[1])
						: y(stackedDataPoint[1] - stackedDataPoint[0]);

					// Check if there's enough space above the bar for tooltip (need ~100px)
					const tooltipHeight = 100;
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
						showBelow: showBelow,
						time: time,
						group: stackedDataPoint.key,
						value: value,
						total: total
					};

					// Highlight the bar with a border using the group's color
					d3.select(rect)
						.attr('stroke', colors(stackedDataPoint.key))
						.attr('stroke-width', 3)
						// Draw only left and right borders
						.style(
							'stroke-dasharray',
							`0 ${rect.width.baseVal.value} ${rect.height.baseVal.value} ${rect.width.baseVal.value} ${rect.height.baseVal.value}`
						);
				})
				.on('mouseleave', function (event: MouseEvent) {
					// Hide tooltip
					tooltip = { ...tooltip, visible: false };

					// Remove highlight
					d3.select(event.target as SVGRectElement).attr('stroke', 'none');
				})
		);

		d3.select(gx).call((sel) =>
			sel.call(
				d3
					.axisBottom(x)
					.tickFormat(displayMode === 'absolute' ? absoluteTimeFormatter : relativeTimeFormatter)
					.tickValues(
						x.domain().filter((val, idx, arr) => {
							return idx %
								(arr.length > maxTimeTicks ? Math.ceil(arr.length / maxTimeTicks) : 1) ===
								0
								? val
								: false;
						})
					)
			)
		);

		d3.select(gy).call((sel) =>
			sel.call(d3.axisLeft(y).tickFormat(yAxisTickFormatter).tickValues(yAxisTickValues))
		);

		d3.select(gyGrid).call((sel) =>
			sel
				.selectAll('line')
				.data(yValues)
				.join('line')
				.attr('x1', 0)
				.attr('x2', width - marginRight - marginLeft)
				.attr('y1', (tickValue) => y(tickValue))
				.attr('y2', (tickValue) => y(tickValue))
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

		<g bind:this={gBars} />

		{#if isSome(average)}
			<line
				x1={marginLeft}
				x2={width - marginRight}
				y1={unwrapOr(averageLineY, 0)}
				y2={unwrapOr(averageLineY, 0)}
				stroke="currentColor"
				stroke-width="1.5"
				opacity="0.6"
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
				opacity="0.6"
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
							<div class="text-xs opacity-80">
								{#if showGroup}
									<span>{tooltip.group}</span>
									<span>•</span>
								{/if}
								<span>{formatTooltipValue(tooltip.value, format, unit)}</span>
							</div>
							{#if showGroup && tooltip.total !== tooltip.value && stacked}
								<div class="text-xs opacity-60">
									<span>Total</span>
									<span>•</span>
									<span>{formatTooltipValue(tooltip.total, format, unit)}</span>
								</div>
							{/if}
						</div>
					</div>
				</div>
			</foreignObject>
		{/if}
	</svg>

	<!-- Legend -->
	{#if showGroup}
		<div class="flex flex-wrap items-center justify-center gap-3 px-2 text-sm">
			{#each groups as group (group)}
				<div class="flex items-center gap-1.5">
					<div class="h-3 w-3 rounded-sm" style="background-color: {colors(group)}"></div>
					<span>{group}</span>
				</div>
			{/each}
		</div>
	{/if}
</div>
