<script lang="ts">
	import { formatDurationCompactWithUnits, formatWeekInterval } from '$lib/duration';
	import * as d3 from 'd3';
	import dayjs from 'dayjs';

	export interface TimeseriesChartProps {
		values: { time: string; group: string; value: number }[];
		width: number;
		height: number;
		unit: string;
		granularity: string;
		format: 'number' | 'duration';
		showGroup?: boolean;
	}

	let {
		values,
		height,
		width,
		unit,
		granularity,
		format,
		showGroup = true
	}: TimeseriesChartProps = $props();
	let marginTop = 20;
	let marginRight = 20;
	let marginBottom = 20;
	let marginLeft = 50;

	let gx: SVGGElement;
	let gy: SVGGElement;
	let gyGrid: SVGGElement;
	let svg: SVGElement;

	let timeAxisTickFormater = $derived.by(() => {
		if (granularity === 'Monthly') {
			return (date: string, _idx: number) => {
				return dayjs(date).format('MMM YYYY');
			};
		}

		if (granularity === 'Weekly') {
			return (date: string) => {
				return formatWeekInterval(date);
			};
		}

		return (date: string, _idx: number) => date;
	});

	let yAxisTickFormater = $derived.by(() => {
		if (format === 'duration') {
			return (value: d3.NumberValue, _idx: number) => {
				return formatDurationCompactWithUnits(value.valueOf());
			};
		}

		return (value: d3.NumberValue, _idx: number) => value.toString();
	});

	let yAxisDefaultTickValues = (): number[] => {
		const maxGroupValue = values
			.reduce<Map<string, number>>((groupValues, value) => {
				if (groupValues.has(value.time)) {
					groupValues.set(value.time, groupValues.get(value.time)! + value.value);
				} else {
					groupValues.set(value.time, value.value);
				}

				return groupValues;
			}, new Map<string, number>())
			.entries()
			.reduce(([_dt, previous], [__, curr]) => [_dt, curr > previous ? curr : previous])[1];
		return d3.ticks(0, maxGroupValue, 6);
	};

	let yAxisTickValues = (): number[] => {
		if (format === 'duration') {
			const dt = 600;
			const maxDuration = values
				.reduce<Map<string, number>>((times, value) => {
					if (times.has(value.time)) {
						times.set(value.time, times.get(value.time)! + value.value);
					} else {
						times.set(value.time, value.value);
					}

					return times;
				}, new Map<string, number>())
				.entries()
				.reduce(([_dt, previous], [__, curr]) => [_dt, curr > previous ? curr : previous])[1];
			const roundedUpMaxDuration = Math.ceil(maxDuration / dt) * dt;
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

	// Create stacked series data structure
	// Each series represents one group (e.g., "Cycling", "Running")
	// d3.stack() transforms the data into layers for stacked bar visualization
	let series = $derived(
		d3
			.stack()
			.keys(d3.union(values.map((v) => v.group)))
			.value(([_time, groupMap]: any, groupKey) => groupMap.get(groupKey).value)(
			d3.index(
				values as any,
				(value: any) => value.time,
				(value: any) => value.group
			) as any
		)
	);

	let x = $derived(
		d3
			.scaleBand()
			.domain(
				d3.groupSort(
					values,
					(a, b) => (a.at(0)!.time < b.at(0)!.time ? -1 : 1),
					(value) => value.time
				)
			)
			.range([marginLeft, width - marginRight])
			.padding(0.6)
	);

	let y = $derived(
		d3
			.scaleLinear()
			.domain([0, d3.max(series, (groupSeries) => d3.max(groupSeries, (point) => point[1]))!])
			.rangeRound([height - marginBottom, marginTop])
	);

	const color = $derived(d3.scaleOrdinal(d3.schemeObservable10));

	// Extract unique group names for the legend
	let groups = $derived(Array.from(d3.union(values.map((v) => v.group))).sort());

	let maxTimeTicks = $derived(Math.min(8, Math.floor(width / 70)));

	// Tooltip state
	let tooltip = $state<{
		visible: boolean;
		x: number;
		y: number;
		time: string;
		group: string;
		value: number;
		total: number;
	}>({
		visible: false,
		x: 0,
		y: 0,
		time: '',
		group: '',
		value: 0,
		total: 0
	});

	// Format tooltip value based on format type
	const formatTooltipValue = (value: number): string => {
		if (format === 'duration') {
			return formatDurationCompactWithUnits(value);
		}
		if (unit === 'activities') {
			return `${Math.round(value)} ${unit}`;
		}
		return `${value.toFixed(1)} ${unit}`;
	};

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
		d3.select(svg).call((sel) =>
			sel
				.selectAll('g')
				.data(series)
				.join('g')
				.attr('fill', (groupSeries) => color(groupSeries.key))
				.selectAll('rect')
				.data((groupSeries) =>
					groupSeries.map((stackedDataPoint: any) => {
						// Attach the group key (e.g., "Cycling", "Running") to each data point
						// so we can identify which group each rectangle belongs to
						stackedDataPoint.key = groupSeries.key;
						return stackedDataPoint;
					})
				)
				.join('rect')
				.attr('x', (stackedDataPoint: any) => x(stackedDataPoint.data[0])!)
				.attr('y', (stackedDataPoint: any) => y(stackedDataPoint[1]))
				.attr('height', (stackedDataPoint: any) => y(stackedDataPoint[0]) - y(stackedDataPoint[1]))
				.attr('width', x.bandwidth())
				.on('mouseenter', function (event: MouseEvent, stackedDataPoint: any) {
					// Show tooltip
					const rect = event.target as SVGRectElement;
					const rectBounds = rect.getBoundingClientRect();
					const value = stackedDataPoint[1] - stackedDataPoint[0]; // Height of this segment
					const time = stackedDataPoint.data[0];

					// Calculate total for this time across all groups
					const total = values.filter((v) => v.time === time).reduce((sum, v) => sum + v.value, 0);

					tooltip = {
						visible: true,
						x: rectBounds.left + rectBounds.width / 2,
						y: rectBounds.top,
						time: time,
						group: stackedDataPoint.key,
						value: value,
						total: total
					};

					// Highlight the bar
					d3.select(rect).attr('opacity', 0.8);
				})
				.on('mouseleave', function (event: MouseEvent) {
					// Hide tooltip
					tooltip = { ...tooltip, visible: false };

					// Remove highlight
					d3.select(event.target as SVGRectElement).attr('opacity', 1);
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
	>
		<g
			bind:this={gyGrid}
			transform="translate({marginLeft} 0)"
			stroke="currentColor"
			opacity="0.3"
		/>

		<g bind:this={svg} />

		<g bind:this={gx} transform="translate(0 {height - marginBottom})" />
		<g bind:this={gy} transform="translate({marginLeft} 0)" />
	</svg>

	<!-- Legend -->
	{#if showGroup}
		<div class="flex flex-wrap items-center justify-center gap-3 px-2 text-sm">
			{#each groups as group}
				<div class="flex items-center gap-1.5">
					<div class="h-3 w-3 rounded-sm" style="background-color: {color(group)}"></div>
					<span>{group}</span>
				</div>
			{/each}
		</div>
	{/if}
</div>

<!-- Tooltip -->
{#if tooltip.visible}
	<div
		class="pointer-events-none fixed z-50 rounded-box bg-base-300 px-3 py-2 text-sm shadow-lg"
		style="left: {tooltip.x}px; top: {tooltip.y - 10}px; transform: translate(-50%, -100%);"
	>
		<div class="flex flex-col gap-1">
			<div class="font-semibold">{timeAxisTickFormater(tooltip.time, 0)}</div>
			<div class="flex items-center gap-2 text-xs opacity-80">
				{#if showGroup}
					<span>{tooltip.group}</span>
					<span>•</span>
				{/if}
				<span>{formatTooltipValue(tooltip.value)}</span>
			</div>
			{#if showGroup && tooltip.total !== tooltip.value}
				<div class="flex items-center gap-2 text-xs opacity-60">
					<span>Total</span>
					<span>•</span>
					<span>{formatTooltipValue(tooltip.total)}</span>
				</div>
			{/if}
		</div>
	</div>
{/if}
