<script lang="ts">
	import * as d3 from 'd3';

	export type LineOrder = 'first' | 'second' | 'third';
	export interface TimseriesLineProps {
		values: Array<[number, number]>;
		xScale: d3.ScaleLinear<number, number, never>;
		range: Array<number>;
		order: LineOrder;
		yMargin: number;
		width: number;
	}

	let { values, xScale, range, yMargin, order, width }: TimseriesLineProps = $props();
	let path: SVGPathElement;
	let gy: SVGGElement;

	let domain = $derived(d3.extent(values, (val) => val[1]) as [number, number]);
	let scale = $derived(d3.scaleLinear(domain, range));
	let axis = $derived(order === 'first' ? d3.axisLeft(scale) : d3.axisRight(scale));
	let line = $derived((data: [number, number][], xScale: d3.ScaleLinear<number, number, never>) =>
		d3
			.line()
			.x((point) => xScale(point[0]))
			.y((point) => scale(point[1]))(data)
	);
	$effect(() => {
		d3.select(path).attr('d', line(values, xScale));
		d3.select(gy).call(axis);
	});
	let axisTranslate = $derived.by(() => {
		if (order === 'first') {
			return `translate(${yMargin} 0)`;
		}
		if (order === 'second') {
			return `translate(${width - yMargin} 0)`;
		}

		if (order === 'third') {
			return `translate(${yMargin} 0)`;
		}
	});

	export const colors: Record<LineOrder, string> = {
		first: 'stroke-first-chart',
		second: 'stroke-second-chart',
		third: 'stroke-third-chart'
	};
</script>

<g bind:this={gy} transform={axisTranslate} class={colors[order]} fill={null} />
<path
	bind:this={path}
	clip-path="url(#clip-path)"
	fill="none"
	class={colors[order]}
	stroke-width="1.5"
/>
