<script lang="ts">
	import { formatDurationCompactWithUnits } from '$lib/duration';
	import { paceInSecondToString } from '$lib/speed';
	import * as d3 from 'd3';
	import { dayjs } from '$lib/duration';
	import { isSome, map, none, unwrapOr, type Option } from '$lib/Options';

	export interface TimeseriesChartProps {
		values: Record<string, Record<string, number>>;
		width: number;
		height: number;
		unit: string;
		format: 'number' | 'duration' | 'pace';
		average: Option<number>;
		timeDomain?: Option<{ start: string; end: string | null }>;
	}

	let {
		values,
		height,
		width,
		unit,
		format,
		average,
		timeDomain = none()
	}: TimeseriesChartProps = $props();
	let marginTop = 20;
	let marginRight = 20;
	let marginBottom = 20;
	let marginLeft = 55;

	let gx: SVGGElement;
	let gy: SVGGElement;
	let gyGrid: SVGGElement;
	let gDots: SVGGElement;
	let svgElement: SVGElement;

	let formatedValues = $derived.by(() => {
		const _values: { time: string; group: string; value: number }[] = [];
		for (const [group, granuleValues] of Object.entries(values)) {
			for (const [time, value] of Object.entries(granuleValues as Record<string, number>)) {
				_values.push({ time, group, value });
			}
		}
		return _values;
	});

	let valuesAsTime = $derived(
		formatedValues.map(({ time, value }) => ({ time: dayjs(time).unix(), value: value }))
	);

	let timeAxisTickFormater = $derived.by(() => {
		return (timestamp: d3.NumberValue, _idx: number) =>
			dayjs.unix(timestamp.valueOf()).format('MMM D');
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
		if (valuesAsTime.length === 0) {
			return [];
		}

		return d3.ticks(0, d3.max(valuesAsTime, (v) => v.value) ?? 0, 6);
	};

	let yAxisTickValues = (): number[] => {
		if (valuesAsTime.length === 0) {
			return [];
		}
		if (format === 'duration') {
			const dt = 600;
			const maxDuration = d3.max(valuesAsTime, (v) => v.value) ?? 0;
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
	let minTime = $derived.by(() => {
		const values = [
			dayjs
				.unix(d3.min(valuesAsTime, (v) => v.time) ?? 0)
				.startOf('day')
				.unix()
		];
		if (isSome(timeDomain)) {
			values.push(dayjs(timeDomain.value.start).unix());
		}

		return Math.min(...values);
	});
	let maxTime = $derived.by(() => {
		const values = [
			dayjs
				.unix(d3.max(valuesAsTime, (v) => v.time) ?? 0)
				.endOf('day')
				.unix()
		];

		if (isSome(timeDomain) && timeDomain.value.end !== null) {
			values.push(dayjs(timeDomain.value.end).unix());
		}

		return Math.max(...values);
	});

	let x = $derived(
		d3
			.scaleLinear()
			.domain([minTime, maxTime])
			.range([marginLeft, width - marginRight])
	);

	let y = $derived(
		d3
			.scaleLinear()
			.domain([0, (d3.max(valuesAsTime, (v) => v.value) ?? 0) * 1.1])
			.rangeRound([height - marginBottom, marginTop])
	);

	let averageLineY = $derived(map(average, (avg) => y(avg)));
	let averageLegendY = $derived(
		map(averageLineY, (avg) =>
			Math.max(marginTop + 12, Math.min(height - marginBottom - 4, avg - 6))
		)
	);
	let averageLegend = $derived(map(average, (avg) => `Average = ${formatTooltipValue(avg)}`));

	// Tooltip state
	let tooltip = $state<{
		visible: boolean;
		x: number;
		y: number;
		showBelow: boolean;
		time: number;
		value: number;
	}>({
		visible: false,
		x: 0,
		y: 0,
		showBelow: false,
		time: 0,
		value: 0
	});

	// Format tooltip value based on format type
	const formatTooltipValue = (value: number): string => {
		if (format === 'duration') {
			return formatDurationCompactWithUnits(value);
		}
		if (unit === 'activities') {
			return `${Math.round(value)} ${unit}`;
		}
		if (format === 'pace') {
			return `${paceInSecondToString(value)} /km`;
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
		d3.select(gDots).call((sel) =>
			sel
				.attr('stroke', 'steelblue')
				.attr('stroke-width', 1.5)
				.attr('fill', 'none')
				.selectAll('circle')
				.data(valuesAsTime)
				.join('circle')
				.attr('cx', (d) => x(d.time))
				.attr('cy', (d) => y(d.value))
				.attr('r', 5)

				.on('mouseenter', function (event: MouseEvent, point: { time: number; value: number }) {
					// Show tooltip
					const value = point.value;
					const time = point.time;

					// Use SVG coordinates directly
					const xPos = x(point.time);
					const yPos = y(point.value);

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
						time: time,
						value: value
					};

					// Highlight the circle
					d3.select(event.target as SVGRectElement)
						.attr('stroke-width', 2.5)
						.attr('fill', 'steelblue')
						.attr('fill-opacity', 0.6);
				})
				.on('mouseleave', function (event: MouseEvent) {
					// Hide tooltip
					tooltip = { ...tooltip, visible: false };

					// Remove highlight
					d3.select(event.target as SVGRectElement)
						.attr('stroke-width', 1.5)
						.attr('fill', 'none')
						.attr('fill-opacity', 1);
				})
		);

		let maxTimeTicks = $derived(Math.min(8, Math.floor(width / 70)));
		d3.select(gx).call((sel) => {
			sel.call(d3.axisBottom(x).tickFormat(timeAxisTickFormater).ticks(maxTimeTicks));
		});

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
	>
		<g
			bind:this={gyGrid}
			transform="translate({marginLeft} 0)"
			stroke="currentColor"
			opacity="0.3"
		/>

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
				style="paint-order: stroke; stroke: var(--fallback-b1, #ffffff); stroke-width: 3px;"
			>
				{unwrapOr(averageLegend, '')}
			</text>
		{/if}

		<g bind:this={gDots} />

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
							<div class="font-semibold">{dayjs.unix(tooltip.time.valueOf()).format('MMM D')}</div>
							<div class="font-italic text-xs">{dayjs.unix(tooltip.time).format('H[h]mm')}</div>
							<div class="text-xs opacity-80">
								<span>{formatTooltipValue(tooltip.value)}</span>
							</div>
						</div>
					</div>
				</div>
			</foreignObject>
		{/if}
	</svg>
</div>
