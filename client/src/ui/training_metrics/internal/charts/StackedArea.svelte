<!-- Stacked area chart for Feedback/Weight&Nutrition data -->
<script lang="ts">
	import dayjs from 'dayjs';
	import * as d3 from 'd3';
	import { isSome, map, none, unwrapOr, type Option } from '$lib/Options';
	import { formatTooltipValue } from '.';
	import { SvelteMap } from 'svelte/reactivity';

	let {
		data,
		width,
		height,
		target,
		average,
		format,
		unit,
		yMaxValue = none()
	}: {
		data: Record<string, Record<string, number | null>>;
		width: number;
		height: number;
		unit: string;
		format: 'number';
		average: Option<number>;
		target: Option<number>;
		yMaxValue?: Option<number>;
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

	let values = $derived.by(() => {
		const _values: { time: string; timestamp: number; group: string; value: number }[] = [];
		for (const [group, granuleValues] of Object.entries(data)) {
			for (const [time, value] of Object.entries(granuleValues)) {
				if (value !== null) {
					_values.push({ time, timestamp: dayjs(time).unix(), group, value });
				}
			}
		}
		return _values;
	});

	let groups = $derived([...new Set(values.map((v) => v.group))].sort());

	let times = $derived.by(() => {
		const tsByTime = new SvelteMap<string, number>();
		for (const v of values) {
			const known = tsByTime.get(v.time);
			if (known === undefined || v.timestamp < known) {
				tsByTime.set(v.time, v.timestamp);
			}
		}
		return [...tsByTime.entries()].sort((a, b) => a[1] - b[1]).map(([time]) => time);
	});

	let timeByTs = $derived(new Map(times.map((time) => [dayjs(time).unix(), time])));

	let rows = $derived.by(() => {
		const valueByGroupAndTime = new SvelteMap<string, number>();
		for (const v of values) {
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

	let yAxisDefaultTickValues = (): number[] => {
		if (values.length === 0) {
			return [];
		}

		return d3.ticks(0, maxValue, 6);
	};

	let yAxisTickValues = (): number[] => {
		if (values.length === 0) {
			return [];
		}

		return yAxisDefaultTickValues();
	};

	let maxValue = $derived(
		isSome(yMaxValue)
			? yMaxValue.value
			: Math.max(
					d3.max(series, (s) => d3.max(s, (d) => d[1]) ?? 0) ?? 0,
					unwrapOr(target, Number.NEGATIVE_INFINITY)
				)
	);

	let xAxis = $derived(
		d3
			.scalePoint()
			.domain(times)
			.range([marginLeft, width - marginRight])
	);
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

	let color = $derived(d3.scaleOrdinal(d3.schemeCategory10).domain(groups));

	let timeAxisTickFormater = $derived.by(() => {
		return (date: string, _idx: number) => {
			return dayjs(date).format('MMM D');
		};
	});

	let yAxisTickFormater = $derived.by(() => {
		return (value: d3.NumberValue, _idx: number) =>
			`${value.toString()} ${unit === 'activities' ? '' : unit}`;
	});
	let averageLineY = $derived(map(average, (avg) => yAxis(avg)));
	let averageLegendY = $derived(
		map(averageLineY, (avg) =>
			Math.max(marginTop + 12, Math.min(height - marginBottom - 4, avg - 6))
		)
	);
	let averageLegend = $derived(
		map(average, (avg) => `Average = ${formatTooltipValue(avg, format, unit)}`)
	);

	let targetLineY = $derived(map(target, (t) => yAxis(t)));
	let targetLegendY = $derived(
		map(targetLineY, (t) => Math.max(marginTop + 12, Math.min(height - marginBottom - 4, t - 6)))
	);
	let targetLegend = $derived(
		map(target, (t) => `Target = ${formatTooltipValue(t, format, unit)}`)
	);

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
					d3.select(this).attr('fill-opacity', 0.9);
				})
				.on('mouseleave', function (_event: MouseEvent) {
					// Hide tooltip
					tooltip = { ...tooltip, visible: false };

					// Remove highlight
					d3.select(this).attr('fill-opacity', 0.7);
				})
		);

		let maxTimeTicks = $derived(Math.min(8, Math.floor(width / 70)));
		d3.select(gx).call((sel) => {
			sel.call(d3.axisBottom(xAxis).tickFormat(timeAxisTickFormater).ticks(maxTimeTicks));
		});

		d3.select(gy).call((sel) =>
			sel.call(d3.axisLeft(yAxis).tickFormat(yAxisTickFormater).tickValues(yAxisTickValues()))
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
							<div class="font-semibold">{tooltip.time}</div>
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
