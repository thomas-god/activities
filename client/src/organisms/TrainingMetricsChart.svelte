<script lang="ts">
	import * as d3 from 'd3';

	export interface TimeseriesChartProps {
		values: { time: string; value: number }[];
		width: number;
		height: number;
		title: string;
	}

	let { values, height, width, title }: TimeseriesChartProps = $props();
	let marginTop = 20;
	let marginRight = 20;
	let marginBottom = 20;
	let marginLeft = 40;

	let gx: SVGGElement;
	let gy: SVGGElement;

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
</script>

<svg {width} {height} viewBox={`0 0 ${width} ${height}`} role="img" class="p-1 select-none">
	<g>
		{#each values as value (value.time)}
			<rect
				x={x(value.time)}
				y={y(value.value)}
				height={y(0) - y(value.value)}
				width={x.bandwidth()}
				rx={7}
				ry={0}
				class="fill-accent"
			/>
		{/each}
	</g>
	<g bind:this={gx} transform="translate(0 {height - marginBottom})" />
	<g bind:this={gy} transform="translate({marginLeft} 0)" />
</svg>
