<script lang="ts">
	import { strokeColors, textColors, type Metric } from '$lib/colors';
	import * as d3 from 'd3';

	export type LineOrder = 'first' | 'second' | 'third';
	export interface TimseriesLineProps {
		values: Array<[number, number]>;
		xScale: d3.ScaleLinear<number, number, never>;
		range: Array<number>;
		metric: Metric;
		order: LineOrder;
		yMargin: number;
		width: number;
		smoothing?: 'none' | number;
	}

	let {
		values: input_values,
		xScale,
		range,
		yMargin,
		order,
		width,
		metric,
		smoothing = 'none'
	}: TimseriesLineProps = $props();
	let path: SVGPathElement;
	let gy: SVGGElement;

	let values: [number, number][] = $derived.by(() => {
		if (smoothing === 'none' || smoothing < 2) {
			return input_values;
		}
		let smooth_factor = Math.floor(smoothing);
		return input_values.map((value, idx, arr) => {
			if (idx < smooth_factor) {
				return value;
			}
			let sum = 0;
			for (let i = idx; i > idx - smooth_factor; i--) {
				sum += arr[i][1];
			}
			return [value[0], sum / smooth_factor];
		});
	});

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
</script>

<g bind:this={gy} transform={axisTranslate} class={textColors[metric]} fill={null} />
<path
	bind:this={path}
	clip-path="url(#clip-path)"
	fill="none"
	class={strokeColors[metric]}
	stroke-width="1.5"
/>
