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
	import { asOption, isNone, isSome, map, none, some, unwrapOr, type Option } from '$lib/Options';
	import { untrack } from 'svelte';
	import type { TrainingMetricGranularity } from '$lib/trainingMetric';
	import { expectedBinsForDomain, type TimeDomain } from '$ui/training_metrics';
	import { dayjs } from '$lib/duration';

	import {
		buildAbsoluteTimeFormatter,
		buildRelativeTimeFormatter,
		formatTooltipValue,
		mapDomainToGranularity,
		parseMetricIntoPoints,
		type DisplayMode,
		type Point
	} from '.';
	import type { ChartHandle, HoveredBin } from '$ui/training_metrics/TrainingMetricChart.svelte';
	import Tooltip, { type TooltipData } from './Tooltip.svelte';

	let {
		data,
		width,
		height,
		target,
		average,
		format,
		unit,
		granularity,
		yInterceptZero = true,
		yMaxValue = none(),
		timeDomain = none(),
		displayMode = 'absolute',
		syncHoveredBin = $bindable()
	}: {
		data: Record<string, Record<string, number | null>>;
		width: number;
		height: number;
		unit: string;
		format: 'number';
		average: Option<number>;
		target: Option<number>;
		yInterceptZero?: boolean;
		yMaxValue?: Option<number>;
		timeDomain?: TimeDomain;
		granularity: TrainingMetricGranularity;
		displayMode?: DisplayMode;
		syncHoveredBin: Option<HoveredBin>;
	} = $props();

	let marginTop = 20;
	let marginRight = 20;
	let marginBottom = 20;
	let marginLeft = 55;

	let gx: SVGGElement;
	let gy: SVGGElement;
	let gyGrid: SVGGElement;
	let gDots: SVGGElement;
	let gPaths: SVGGElement;
	let svgElement: SVGElement;

	// svelte-ignore state_referenced_locally
	const snappedDomain = mapDomainToGranularity(timeDomain, granularity);

	// svelte-ignore state_referenced_locally
	const { points, times: metricTimes } = parseMetricIntoPoints(data, snappedDomain, {
		replaceNullValues: false
	});

	// svelte-ignore state_referenced_locally
	/* eslint-disable svelte/prefer-writable-derived */
	let domainTimes = $state(expectedBinsForDomain(timeDomain, asOption(granularity), dayjs()));
	$effect(() => {
		domainTimes = expectedBinsForDomain(timeDomain, asOption(granularity), dayjs());
	});

	let times = $derived(unwrapOr(domainTimes, metricTimes));
	let binByTime = $derived(new Map(times.entries().map(([idx, bin]) => [bin, idx])));

	// svelte-ignore state_referenced_locally
	const internalMaxValue = Math.max(
		d3.max(points, (v) => v.value) ?? 0,
		unwrapOr(target, Number.NEGATIVE_INFINITY)
	);

	export function getYMax() {
		return internalMaxValue;
	}
	({ getYMax, getYMin }) satisfies ChartHandle;
	let maxValue = $derived(unwrapOr(yMaxValue, internalMaxValue));

	// svelte-ignore state_referenced_locally
	const minValue = Math.min(
		d3.min(points, (v) => v.value) ?? 0,
		unwrapOr(target, Number.POSITIVE_INFINITY)
	);

	const delta = $derived((maxValue - minValue) * 0.1);

	export function getYMin() {
		return yInterceptZero ? 0 : minValue - delta;
	}

	const range = $derived([
		yInterceptZero ? 0 : minValue - delta,
		yInterceptZero ? maxValue * 1.1 : maxValue + delta
	]);
	const yAxisDefaultTickValues = (): number[] => {
		if (points.length === 0) {
			return [];
		}

		return d3.ticks(range[0], range[1], 6);
	};

	const yAxisTickValues = (): number[] => {
		if (points.length === 0) {
			return [];
		}

		return yAxisDefaultTickValues();
	};

	let maxTimeTicks = $derived(Math.min(8, Math.floor(width / 70)));
	let xTickValues = $derived.by(() => {
		if (times.length === 0) {
			return [];
		}
		const step = Math.max(1, Math.ceil(times.length / maxTimeTicks));
		return times.filter((_, i) => i % step === 0 || i === times.length - 1);
	});

	let xAxis = $derived(
		d3
			.scalePoint()
			.domain(times)
			.range([marginLeft, width - marginRight])
	);
	let yAxis = $derived(
		d3
			.scaleLinear()
			.domain(range)
			.rangeRound([height - marginBottom, marginTop])
	);

	let line = $derived(
		d3
			.line<Point>()
			.x((d) => xAxis(d.time)!)
			.y((d) => yAxis(d.value))
			.curve(d3.curveCatmullRom.alpha(0.5))
	);

	const groupedValues = d3.groups(
		[...points].sort((a, b) => a.timestamp - b.timestamp),
		(v) => v.group
	);

	const color = d3
		.scaleOrdinal(d3.schemeCategory10)
		.domain(groupedValues.map(([group]) => group).sort());

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
	let tooltip = $state<TooltipData>({
		visible: false,
		source: 'local',
		x: 0,
		y: 0,
		showBelow: false,
		time: '',
		group: '',
		value: 0,
		total: none()
	});
	$effect(() => {
		const _tooltip = untrack(() => tooltip);
		if (isNone(syncHoveredBin)) {
			if (_tooltip.source === 'external') {
				tooltip = { ..._tooltip, visible: false };
			}
			return;
		}
		if (_tooltip.visible && _tooltip.source === 'local') {
			// Already visible from local, don't try to override it
			return;
		}

		const time = times.at(syncHoveredBin.value.bin);
		if (time === undefined) {
			return;
		}
		const group = syncHoveredBin.value.group ?? 'Total';

		const xPos = xAxis(time)!;
		const total = points.filter((v) => v.time === time).reduce((sum, v) => sum + v.value, 0);
		const yPos = Math.min(Math.max(yAxis(total), yAxis.range()[0]), yAxis.range()[1]);

		// Check if there's enough space above the point for tooltip (need ~100px)
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
			source: 'external',
			x: tooltipX,
			y: yPos,
			showBelow,
			time,
			group,
			value: total,
			total: none()
		};
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
		d3.select(gPaths).call((sel) =>
			sel
				.attr('fill', 'none')
				.selectAll<SVGPathElement, [string, Point[]]>('path')
				.data(groupedValues, (d) => d[0])
				.join('path')
				.attr('stroke', (d) => color(d[0]))
				.attr('d', (d) => line(d[1]))
		);

		d3.select(gDots).call((sel) =>
			sel
				.attr('stroke-width', 1)
				.attr('fill-opacity', 0.6)
				.selectAll('circle')
				.data(points)
				.join('circle')
				.attr('stroke', (d) => color(d.group))
				.attr('fill', (d) => color(d.group))
				.attr('cx', (d) => xAxis(d.time)!)
				.attr('cy', (d) => yAxis(d.value))
				.attr('r', 4)

				.on(
					'mouseenter',
					function (
						event: MouseEvent,
						point: { time: string; timestamp: number; value: number; group: string }
					) {
						// Use SVG coordinates directly
						const xPos = xAxis(point.time)!;
						const yPos = yAxis(point.value);

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

						const bin = binByTime.get(point.time);
						if (bin !== undefined) {
							syncHoveredBin = some({ bin, group: point.group });
						}

						tooltip = {
							visible: true,
							source: 'local',
							x: tooltipX,
							y: yPos,
							showBelow: showBelow,
							time: point.time,
							value: point.value,
							group: point.group,
							total: none()
						};

						// Highlight the circle
						d3.select(event.target as SVGRectElement)
							.attr('stroke-width', 2.5)
							.attr('fill', color(point.group))
							.attr('fill-opacity', 0.8);
					}
				)
				.on('mouseleave', function (event: MouseEvent) {
					// Hide tooltip
					tooltip = { ...tooltip, visible: false, source: 'local' };
					syncHoveredBin = none();

					// Remove highlight
					d3.select(event.target as SVGCircleElement)
						.attr('stroke-width', 1)
						.attr(
							'fill',
							color(
								(d3.select(event.target as SVGCircleElement).datum() as { group: string }).group
							)
						)
						.attr('fill-opacity', 0.6);
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

		<g bind:this={gDots} />

		<g bind:this={gPaths} />

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
		<Tooltip
			data={tooltip}
			{format}
			{unit}
			primaryTimeFormatter={displayMode === 'absolute'
				? (time: string) => absoluteTimeFormatter(time, 0)
				: (time: string) => relativeTimeFormatter(time, 0)}
			secondaryTimeFormatter={displayMode === 'relative'
				? some((time: string) => absoluteTimeFormatter(time, 0))
				: none()}
		/>
	</svg>
</div>
