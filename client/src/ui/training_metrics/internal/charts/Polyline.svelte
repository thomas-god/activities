<!-- Polyline chart for Feedback/Weight&Nutrition data -->
<script lang="ts">
	import dayjs from 'dayjs';
	import * as d3 from 'd3';
	import { isSome, map, none, unwrapOr, type Option } from '$lib/Options';
	import { formatTooltipValue } from '.';

	let {
		data,
		width,
		height,
		target,
		average,
		format,
		unit,
		yInterceptZero = true,
		yMaxValue = none()
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

	type Point = { time: string; timestamp: number; group: string; value: number };

	let values = $derived.by(() => {
		const _values: Point[] = [];
		for (const [group, granuleValues] of Object.entries(data)) {
			for (const [time, value] of Object.entries(granuleValues)) {
				if (value !== null) {
					_values.push({ time, timestamp: dayjs(time).unix(), group, value });
				}
			}
		}
		return _values;
	});

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
			: Math.max(d3.max(values, (v) => v.value) ?? 0, unwrapOr(target, Number.NEGATIVE_INFINITY))
	);
	let minValue = $derived(
		Math.min(d3.min(values, (v) => v.value) ?? 0, unwrapOr(target, Number.POSITIVE_INFINITY))
	);

	let xAxis = $derived(
		d3
			.scalePoint()
			.domain(values.map((v) => v.time).sort((a, b) => (a > b ? 1 : -1)))
			.range([marginLeft, width - marginRight])
	);
	let yAxis = $derived(
		d3
			.scaleLinear()
			.domain([yInterceptZero ? 0 : minValue, maxValue * 1.1])
			.rangeRound([height - marginBottom, marginTop])
	);

	let line = $derived(
		d3
			.line<Point>()
			.x((d) => xAxis(d.time)!)
			.y((d) => yAxis(d.value))
			.curve(d3.curveCatmullRom.alpha(0.5))
	);

	let groupedValues = $derived(
		d3.groups(
			[...values].sort((a, b) => a.timestamp - b.timestamp),
			(v) => v.group
		)
	);

	let color = $derived(
		d3.scaleOrdinal(d3.schemeCategory10).domain(groupedValues.map(([group]) => group).sort())
	);

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
	}>({
		visible: false,
		x: 0,
		y: 0,
		showBelow: false,
		time: '',
		group: '',
		value: 0
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
				.data(values)
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

						tooltip = {
							visible: true,
							x: tooltipX,
							y: yPos,
							showBelow: showBelow,
							time: point.time,
							value: point.value,
							group: point.group
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
					tooltip = { ...tooltip, visible: false };

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
							<div class="font-italic text-xs">{tooltip.group}</div>
							<div class="text-xs opacity-80">
								<span>{formatTooltipValue(tooltip.value, format, unit)}</span>
							</div>
						</div>
					</div>
				</div>
			</foreignObject>
		{/if}
	</svg>
</div>
