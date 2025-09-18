<script lang="ts">
	import * as d3 from 'd3';

	export type AxisPosition = 'left' | 'right' | 'leftOffset';
	export interface TimseriesLineProps {
		values: Array<[number, number]>;
		xScale: d3.ScaleLinear<number, number, never>;
		range: Array<number>;
		position: AxisPosition;
		yMargin: number;
		width: number;
	}

	let { values, xScale, range, yMargin, position, width }: TimseriesLineProps = $props();
	let path: SVGPathElement;
	let gy: SVGGElement;

	let domain = $derived(d3.extent(values, (val) => val[1]) as [number, number]);
	let scale = $derived(d3.scaleLinear(domain, range));
	let axis = $derived(position === 'left' ? d3.axisLeft(scale) : d3.axisRight(scale));
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
		if (position === 'left') {
			return `translate(${yMargin} 0)`;
		}
		if (position === 'right') {
			return `translate(${width - yMargin} 0)`;
		}

		if (position === 'leftOffset') {
			return `translate(${yMargin} 0)`;
		}
	});

	const colors: Record<AxisPosition, string> = {
		left: 'stroke-chart-one',
		right: 'stroke-chart-two',
		leftOffset: 'stroke-chart-three'
	};
</script>

<g bind:this={gy} transform={axisTranslate} class={colors[position]} fill={null} />
<path
	bind:this={path}
	clip-path="url(#clip-path)"
	fill="none"
	class={colors[position]}
	stroke-width="1.5"
/>
