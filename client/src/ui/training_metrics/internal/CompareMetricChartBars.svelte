<script lang="ts">
	import { formatDurationCompactWithUnits, formatWeekInterval } from '$lib/duration';
	import {
		bucketOffset,
		displayGroupName,
		relativeBucketLabel,
		type TrainingMetricGranularity,
		type TrainingMetricGroupByClause
	} from '$lib/trainingMetric';
	import { isSome, map, unwrapOr, type Option } from '$lib/Options';
	import { paceInSecondToString } from '$lib/speed';
	import * as d3 from 'd3';
	import dayjs from 'dayjs';
	import { formatTooltipValue, getGroupColor } from './chart';

	export interface CompareMetricChartBarsProps {
		values: Record<string, Record<string, number>>;
		/** Date the chart's buckets are aligned to (the compared period's anchor). */
		anchor: string;
		/** Shared x-axis domain of the comparison (union of both periods' bucket offsets). */
		bucketDomain: number[];
		yDomain: number[];
		width: number;
		height: number;
		unit: string;
		granularity: TrainingMetricGranularity;
		format: 'number' | 'duration' | 'pace';
		showGroup?: boolean;
		groupBy: TrainingMetricGroupByClause | null;
		stacked?: boolean;
		average: Option<number>;
		target: Option<number>;
		/** Bucket offset currently hovered, shared between the compared charts. */
		hovered?: number | null;
	}

	let {
		values,
		anchor,
		bucketDomain,
		yDomain,
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
		hovered = $bindable(null)
	}: CompareMetricChartBarsProps = $props();

	let marginTop = 20;
	let marginRight = 20;
	let marginBottom = 20;
	let marginLeft = 55;

	let gx: SVGGElement;
	let gy: SVGGElement;
	let gyGrid: SVGGElement;
	let gBars: SVGGElement;
	let svgElement: SVGElement;

	type FormattedValue = { time: string; offset: number; group: string; value: number };
	type SeriesDatum = [number, d3.InternMap<string, FormattedValue>];
	type StackedDataPoint = d3.SeriesPoint<SeriesDatum> & { key: string };

	let formatedValues = $derived.by(() => {
		const _values: FormattedValue[] = [];
		for (const [group, granuleValues] of Object.entries(values)) {
			for (const [time, value] of Object.entries(granuleValues as Record<string, number>)) {
				_values.push({ time, offset: bucketOffset(time, anchor, granularity), group, value });
			}
		}
		return _values;
	});

	// Calendar date of each aligned bucket, used for the axis tick labels: the
	// aligned axis is relative to the anchor but each period keeps its own dates.
	let offsetDates = $derived.by(() => {
		const dates: Record<number, string> = {};
		for (const value of [...formatedValues].toSorted((a, b) => (a.time < b.time ? -1 : 1))) {
			if (!(value.offset in dates)) {
				dates[value.offset] = value.time;
			}
		}
		return dates;
	});

	// Axis ticks use the shared domain's ordinal positions so both compared
	// charts display the same labels at the same positions: "Week 3" is the
	// third timeslot of the comparison for each period.
	let timeAxisTickFormater = $derived.by(() => {
		return (offset: number, _idx: number) =>
			relativeBucketLabel(offset, bucketDomain[0] ?? 0, granularity);
	});

	// Calendar date of a bucket, shown as context next to the relative label.
	let calendarTickFormater = $derived.by(() => {
		return (offset: number, _idx: number) => {
			const date = offsetDates[offset];
			if (date === undefined) {
				return '';
			}
			if (granularity === 'Monthly') {
				return dayjs(date).format('MMM YYYY');
			}
			if (granularity === 'Weekly') {
				return formatWeekInterval(date);
			}
			return dayjs(date).format('MMM D');
		};
	});

	let yAxisTickFormater = $derived.by(() => {
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
	});

	let yAxisDefaultTickValues = (): number[] => {
		if (formatedValues.length === 0) {
			return [];
		}

		return d3.ticks(0, yDomain[1], 6);
	};

	let yAxisTickValues = (): number[] => {
		if (formatedValues.length === 0) {
			return [];
		}
		if (format === 'duration') {
			const dt = 600;
			const maxDuration = yDomain[1];
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
			return yAxisDefaultTickValues();
		}
		return yAxisDefaultTickValues();
	};

	// Order of the groups inside the stacked series, sorted alphabetically ascending by
	// display name. First entry in the array is stacked at the bottom.
	let groupOrdering = $derived(
		Array.from(d3.union(formatedValues.map((v) => displayGroupName(v.group, groupBy)))).sort()
	);

	// Create stacked series data structure
	// Each series represents one group (e.g., "Cycling", "Running")
	// d3.stack() transforms the data into layers for stacked bar visualization
	let series = $derived(
		d3
			.stack<SeriesDatum, string>()
			.keys(groupOrdering)
			.value(([, groupMap], groupKey) => groupMap.get(groupKey)!.value)(
			d3.index(
				formatedValues,
				(value) => value.offset,
				(value) => displayGroupName(value.group, groupBy)
			)
		)
	);

	let x = $derived(
		d3
			.scaleBand<number>()
			.domain(bucketDomain)
			.range([marginLeft, width - marginRight])
			.padding(0.6)
	);

	let y = $derived(
		d3
			.scaleLinear()
			.domain(yDomain)
			.rangeRound([height - marginBottom, marginTop])
	);

	let averageLineY = $derived(map(average, (avg) => y(avg)));
	let averageLegendY = $derived(
		map(averageLineY, (avg) =>
			Math.max(marginTop + 12, Math.min(height - marginBottom - 4, avg - 6))
		)
	);
	let averageLegend = $derived(
		map(average, (avg) => `Average = ${formatTooltipValue(avg, format, unit)}`)
	);

	let targetLineY = $derived(map(target, (t) => y(t)));
	let targetLegendY = $derived(
		map(targetLineY, (t) => Math.max(marginTop + 12, Math.min(height - marginBottom - 4, t - 6)))
	);
	let targetLegend = $derived(
		map(target, (t) => `Target = ${formatTooltipValue(t, format, unit)}`)
	);

	const colors = $derived.by(() => {
		const scale = d3.scaleOrdinal(d3.schemeObservable10);
		const customScale = (groupName: string) => {
			const customColor = getGroupColor(groupName, groupBy);
			return customColor || scale(groupName);
		};
		return customScale;
	});

	// Extract unique group names for the legend (alphabetical ascending).
	let groups = $derived(groupOrdering);

	let xGroup = $derived(d3.scaleBand().domain(groups).range([0, x.bandwidth()]).padding(0.1));

	let maxTimeTicks = $derived(Math.min(8, Math.floor(width / 70)));

	// Tooltip state; `segment` distinguishes the rich per-bar tooltip (local
	// pointer over a rect) from the total-only tooltip shown when the compared
	// sibling chart hovers the same bucket.
	let tooltip = $state<{
		visible: boolean;
		x: number;
		y: number;
		showBelow: boolean;
		offset: number;
		group: string;
		value: number;
		total: number;
		segment: boolean;
	}>({
		visible: false,
		x: 0,
		y: 0,
		showBelow: false,
		offset: 0,
		group: '',
		value: 0,
		total: 0,
		segment: false
	});

	let pointerOver = $state(false);
	let segmentTooltip = $state(false);

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

	const TOOLTIP_HEIGHT = 100;
	const TOOLTIP_WIDTH = 100;

	const tooltipXFor = (xPos: number): number => {
		const tooltipHalfWidth = TOOLTIP_WIDTH / 2;
		const spaceLeft = xPos - marginLeft;
		const spaceRight = width - marginRight - xPos;

		// Center by default, clamped to the plot area
		let tooltipX = xPos - tooltipHalfWidth;
		if (spaceLeft < tooltipHalfWidth) {
			// Not enough space on the left, align to left edge
			tooltipX += tooltipHalfWidth;
		} else if (spaceRight < tooltipHalfWidth) {
			// Not enough space on the right, align to right edge
			tooltipX -= tooltipHalfWidth;
		}
		return tooltipX;
	};

	const showBelowFor = (yPos: number): boolean => yPos - marginTop < TOOLTIP_HEIGHT;

	const showTotalTooltip = (offset: number) => {
		const bucketValues = formatedValues.filter((value) => value.offset === offset);
		const total = bucketValues.reduce((sum, value) => sum + value.value, 0);
		const topValue =
			bucketValues.length === 0
				? 0
				: stacked
					? d3.sum(bucketValues, (value) => value.value)
					: (d3.max(bucketValues, (value) => value.value) ?? 0);
		const xPos = x(offset)! + x.bandwidth() / 2;
		const yPos = y(topValue);

		tooltip = {
			visible: true,
			x: tooltipXFor(xPos),
			y: yPos,
			showBelow: showBelowFor(yPos),
			offset,
			group: '',
			value: total,
			total,
			segment: false
		};
	};

	// Synced tooltip: when the hovered bucket is set by the compared sibling
	// chart, show this chart's total for the same bucket.
	$effect(() => {
		if (pointerOver) {
			return;
		}
		if (hovered !== null && x.domain().includes(hovered)) {
			if (!tooltip.visible || tooltip.segment || tooltip.offset !== hovered) {
				showTotalTooltip(hovered);
			}
		} else if (tooltip.visible) {
			tooltip = { ...tooltip, visible: false };
		}
	});

	const pointerPosition = (event: PointerEvent): { x: number; y: number } | null => {
		const rect = svgElement.getBoundingClientRect();
		if (rect.width === 0 || rect.height === 0) {
			return null;
		}
		return {
			x: ((event.clientX - rect.left) / rect.width) * width,
			y: ((event.clientY - rect.top) / rect.height) * height
		};
	};

	const offsetAt = (px: number): number | null => {
		const step = x.step();
		if (step === 0) {
			return null;
		}
		const idx = Math.floor((px - marginLeft) / step);
		return x.domain()[idx] ?? null;
	};

	const handlePointerMove = (event: PointerEvent) => {
		const position = pointerPosition(event);
		if (position === null) {
			return;
		}
		pointerOver = true;
		const offset = offsetAt(position.x);
		hovered = offset;
		if (offset === null) {
			if (!segmentTooltip && tooltip.visible) {
				tooltip = { ...tooltip, visible: false };
			}
		} else if (!segmentTooltip) {
			showTotalTooltip(offset);
		}
	};

	const handlePointerLeave = () => {
		pointerOver = false;
		segmentTooltip = false;
		hovered = null;
		if (tooltip.visible) {
			tooltip = { ...tooltip, visible: false };
		}
	};

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
					const offset = stackedDataPoint.data[0];

					// Calculate total for this offset across all groups
					const total = formatedValues
						.filter((v) => v.offset === offset)
						.reduce((sum, v) => sum + v.value, 0);

					// Use SVG coordinates directly
					const xPos = stacked
						? x(stackedDataPoint.data[0])! + x.bandwidth() / 2
						: x(stackedDataPoint.data[0])! + xGroup(stackedDataPoint.key)! + xGroup.bandwidth() / 2;
					const yPos = stacked
						? y(stackedDataPoint[1])
						: y(stackedDataPoint[1] - stackedDataPoint[0]);

					tooltip = {
						visible: true,
						x: tooltipXFor(xPos),
						y: yPos,
						showBelow: showBelowFor(yPos),
						offset: offset,
						group: stackedDataPoint.key,
						value: value,
						total: total,
						segment: true
					};
					segmentTooltip = true;

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
					segmentTooltip = false;

					// Remove highlight
					d3.select(event.target as SVGRectElement).attr('stroke', 'none');
				})
		);

		d3.select(gx).call((sel) =>
			sel.call(
				d3
					.axisBottom(x)
					.tickFormat(timeAxisTickFormater)
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
			sel.call(d3.axisLeft(y).tickFormat(yAxisTickFormater).tickValues(yAxisTickValues()))
		);

		const yValues = yAxisTickValues() === null ? y.ticks() : yAxisTickValues();

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
		onpointermove={handlePointerMove}
		onpointerleave={handlePointerLeave}
	>
		<g
			bind:this={gyGrid}
			transform="translate({marginLeft} 0)"
			stroke="currentColor"
			opacity="0.3"
		/>

		<g bind:this={gBars} />

		{#if hovered !== null && x.domain().includes(hovered)}
			<!-- Synced crosshair: `hovered` is also set by the compared sibling chart -->
			<line
				x1={x(hovered)! + x.bandwidth() / 2}
				x2={x(hovered)! + x.bandwidth() / 2}
				y1={marginTop}
				y2={height - marginBottom}
				stroke="currentColor"
				stroke-width="1"
				stroke-dasharray="4 4"
				opacity="0.5"
				pointer-events="none"
			/>
		{/if}

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
								{relativeBucketLabel(tooltip.offset, bucketDomain[0] ?? 0, granularity)}
								<span class="font-normal opacity-70">
									· {calendarTickFormater(tooltip.offset, 0)}
								</span>
							</div>
							<div class="text-xs opacity-80">
								{#if showGroup && tooltip.segment}
									<span>{tooltip.group}</span>
									<span>•</span>
								{/if}
								{#if showGroup && !tooltip.segment && stacked}
									<span class="opacity-60">Total</span>
									<span>•</span>
								{/if}
								<span>{formatTooltipValue(tooltip.value, format, unit)}</span>
							</div>
							{#if tooltip.segment && showGroup && tooltip.total !== tooltip.value && stacked}
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
