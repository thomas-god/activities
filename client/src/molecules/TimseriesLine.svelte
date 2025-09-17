<script lang="ts">
	import * as d3 from 'd3';

	let {
		values,
		xScale,
		range,
		yMargin
	}: {
		values: Array<[number, number]>;
		xScale: d3.ScaleLinear<number, number, never>;
		range: Array<number>;
		yMargin: number;
	} = $props();
	let path: SVGPathElement;
	let gy: SVGGElement;

	let domain = $derived(d3.extent(values, (val) => val[1]) as [number, number]);
	let scale = $derived(d3.scaleLinear(domain, range));
	let axis = $derived(d3.axisLeft(scale));
	let line = $derived((data: [number, number][]) =>
		d3
			.line()
			.x((point) => xScale(point[0]))
			.y((point) => scale(point[1]))(data)
	);
	$effect(() => {
		d3.select(gy).call(axis);
	});
</script>

<g bind:this={gy} transform="translate({yMargin} 0)" />
<path
	bind:this={path}
	clip-path="url(#clip-path)"
	fill="none"
	class="stroke-neutral"
	stroke-width="1.5"
	d={line(values)}
/>
