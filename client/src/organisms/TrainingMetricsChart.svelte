<script lang="ts">
	import {
		formatDuration,
		formatDurationCompactWithUnits,
		formatWeekInterval
	} from '$lib/duration';
	import * as d3 from 'd3';
	import dayjs from 'dayjs';

	export interface TimeseriesChartProps {
		values: { time: string; value: number }[];
		width: number;
		height: number;
		unit: string;
		granularity: string;
		format: 'number' | 'duration';
	}

	let { values, height, width, unit, granularity, format }: TimeseriesChartProps = $props();
	let marginTop = 20;
	let marginRight = 20;
	let marginBottom = 20;
	let marginLeft = 40;

	let gx: SVGGElement;
	let gy: SVGGElement;
	let gyGrid: SVGGElement;

	let noValues = $derived(values.every((val) => val.value === 0));

	let time = $derived.by(() => {
		let time = [];
		for (let i = 0; i < values.length; i++) {
			time.push(values[i].time);
		}
		return time;
	});

	let tooltipTimeFormater = $derived.by(() => {
		if (granularity === 'Monthly') {
			return (date: string) => {
				return dayjs(date).format('MMM YYYY');
			};
		}

		if (granularity === 'Weekly') {
			return (date: string) => {
				return formatWeekInterval(date);
			};
		}

		return (date: string) => date;
	});

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
		return d3.ticks(0, d3.max(values, (d) => d.value)!, 6);
	};

	let yAxisTickValues = (): number[] => {
		if (format === 'duration') {
			const dt = 600;
			const roundedUpMaxDuration = Math.ceil(d3.max(values, (d) => d.value)! / dt) * dt;
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

	let x = $derived(
		d3
			.scaleBand()
			.domain(time)
			.range([marginLeft, width - marginRight])
			.padding(0.2)
	);
	let y = $derived(
		d3
			.scaleLinear()
			.domain([0, d3.max(values, (d) => d.value)] as [number, number])
			.range([height - marginBottom, marginTop])
	);

	let maxTicks = $derived(Math.min(8, Math.floor(width / 70)));
	$effect(() => {
		d3.select(gx).call((sel) =>
			sel.call(
				d3
					.axisBottom(x)
					.tickFormat(timeAxisTickFormater)
					.tickValues(
						x.domain().filter((val, idx, arr) => {
							return idx % (arr.length > maxTicks ? Math.ceil(arr.length / maxTicks) : 1) === 0
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
				.attr('y1', (d) => y(d))
				.attr('y2', (d) => y(d))
		);
	});

	let selectedMetric: { time: string; value: number; x: number; y: number } | undefined =
		$state(undefined);
	const mouseEnter = (event: MouseEvent, metric: { time: string; value: number }) => {
		selectedMetric = { ...metric, x: event.offsetX, y: event.offsetY };
	};
	const mouseLeave = () => {
		selectedMetric = undefined;
	};
	const mouseMove = (event: MouseEvent) => {
		if (selectedMetric !== undefined) {
			selectedMetric = { ...selectedMetric, x: event.offsetX, y: event.offsetY };
		}
	};

	const formatTooltipValue = (value: number): string => {
		if (format === 'duration') {
			return formatDuration(value);
		}
		return `${value.toFixed(2)} ${unit}`;
	};
</script>

{#if noValues}
	<p class="py-6 text-center text-sm italic opacity-60">
		No activities found for this metric over the selected period
	</p>
{:else}
	<svg {width} {height} viewBox={`0 0 ${width} ${height}`} role="img" class="p-1 select-none">
		<g
			bind:this={gyGrid}
			transform="translate({marginLeft} 0)"
			stroke="currentColor"
			opacity="0.3"
		/>
		<g>
			{#each values as value (value.time)}
				<g
					onmouseenter={(event) => mouseEnter(event, value)}
					onmouseleave={mouseLeave}
					onmousemove={mouseMove}
					role="img"
				>
					<rect
						x={x(value.time)}
						y={y(value.value)}
						height={y(0) - y(value.value)}
						width={x.bandwidth()}
						class="fill-primary"
					/>
					{#if selectedMetric !== undefined}
						<g
							class="rounded-8 bg-base-300"
							transform={`translate(${selectedMetric.x},${selectedMetric.y})`}
							pointer-events="none"
							font-family="sans-serif"
							font-size="10"
							text-anchor="middle"
						>
							<rect x="-30" width="60" y="-30" height="30" class="fill-base-100" rx="3" />
							<text y="-18">{tooltipTimeFormater(selectedMetric.time)}</text>
							<text y="-6">{formatTooltipValue(selectedMetric.value)} </text>
						</g>
					{/if}
				</g>
			{/each}
		</g>

		<g bind:this={gx} transform="translate(0 {height - marginBottom})" />
		<g bind:this={gy} transform="translate({marginLeft} 0)" />
	</svg>
{/if}
