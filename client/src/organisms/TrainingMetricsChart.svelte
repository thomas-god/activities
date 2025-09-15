<script lang="ts">
	import * as d3 from 'd3';

	export interface TimeseriesChartProps {
		values: { time: string; value: number }[];
		width: number;
		height: number;
		unit: string;
	}

	let { values, height, width, unit }: TimeseriesChartProps = $props();
	let marginTop = 20;
	let marginRight = 20;
	let marginBottom = 20;
	let marginLeft = 40;

	let gx: SVGGElement;
	let gy: SVGGElement;

	let noValues = $derived(values.every((val) => val.value === 0));

	let time = $derived.by(() => {
		let time = [];
		for (let i = 0; i < values.length; i++) {
			time.push(values[i].time);
		}
		return time;
	});

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

	$effect(() => {
		d3.select(gx).call((sel) => sel.call(d3.axisBottom(x)));
		d3.select(gy).call((sel) => sel.call(d3.axisLeft(y)));
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
</script>

{#if noValues}
	<p class="py-6 text-center text-sm italic opacity-60">
		No activities found for this metric over the selected period
	</p>
{:else}
	<svg {width} {height} viewBox={`0 0 ${width} ${height}`} role="img" class="p-1 select-none">
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
						rx={2}
						class="fill-accent"
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
							<text y="-18">{selectedMetric.time}</text>
							<text y="-6">{selectedMetric.value} {unit}</text>
						</g>
					{/if}
				</g>
			{/each}
		</g>

		<g bind:this={gx} transform="translate(0 {height - marginBottom})" />
		<g bind:this={gy} transform="translate({marginLeft} 0)" />
	</svg>
{/if}
