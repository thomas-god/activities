<script lang="ts">
	import * as d3 from 'd3';
	import { formatDuration } from '$lib/duration';

	export interface TimeseriesChartProps {
		time: number[];
		metric: Array<number | null>;
		width: number;
		height: number;
	}

	let { time, metric, height, width }: TimeseriesChartProps = $props();
	let marginTop = 20;
	let marginRight = 20;
	let marginBottom = 20;
	let marginLeft = 30;

	let values = $derived.by(() => {
		let v = [];
		for (let i = 0; i < time.length; i++) {
			let val = metric[i];
			if (val !== null) {
				v.push([time[i], val]);
			}
		}
		return v as [number, number][];
	});

	let svgElement: SVGElement;
	let path: SVGPathElement;

	// Define x scale and axis
	let x_min = $derived(values.at(0)![0]);
	let x_max = $derived(values.at(-1)![0]);
	let max_zoom = $derived((x_max - x_min) / 60);
	let x = $derived(d3.scaleLinear([x_min, x_max], [marginLeft, width - marginRight]));
	let xAxis = $derived(
		(
			g: d3.Selection<SVGGElement, unknown, null, undefined>,
			x: d3.ScaleLinear<number, number, never>
		) => g.call(d3.axisBottom(x).tickFormat((time, _) => formatDuration(time.valueOf())))
	);
	let gx: SVGGElement;
	$effect(() => {
		d3.select(gx).call(xAxis, x);
	});

	// Define y scale and axis
	let gy: SVGGElement;
	let y_range = $derived(d3.extent(values, (p) => p[1])) as [number, number];
	let yScale = $derived(d3.scaleLinear(y_range, [height - marginBottom, marginTop]));
	let yAxis = $derived(d3.axisLeft(yScale));
	$effect(() => {
		d3.select(gy).call(yAxis);
	});

	// Define line generator
	let line = $derived((data: [number, number][], xScale: d3.ScaleLinear<number, number, never>) =>
		d3
			.line()
			.x((point) => xScale(point[0]))
			.y((point) => yScale(point[1]))(data)
	);

	// Create the zoom behavior
	let zoom = $derived(
		d3
			.zoom<SVGElement, unknown>()
			.scaleExtent([1, max_zoom])
			.extent([
				[marginLeft, 0],
				[width - marginRight, height]
			])
			.translateExtent([
				[marginLeft, -Infinity],
				[width - marginRight, Infinity]
			])
			.on('zoom', zoomed)
	);

	let zoomedXScale = $state(x);
	function zoomed(event: d3.D3ZoomEvent<SVGElement, any>) {
		zoomedXScale = event.transform.rescaleX(x);
		d3.select(path).attr('d', line(values, zoomedXScale));
		d3.select(gx).call(xAxis, zoomedXScale);
	}

	const resetZoom = () => {
		d3.select(svgElement).call(zoom.scaleTo, 1);
	};

	$effect(() => {
		d3.select(svgElement).call(zoom);
	});

	let tooltipXOffset: number | undefined = $state(undefined);
	let tooltip = $derived.by(() => {
		if (tooltipXOffset === undefined) {
			return undefined;
		}

		const bisector = d3.bisector<[number, number], number>((point) => point[0]);

		let nearestValues = values[bisector.center(values, zoomedXScale.invert(tooltipXOffset))];
		return {
			xOffset: zoomedXScale(nearestValues[0]),
			yOffset: yScale(nearestValues[1]),
			timestamp: formatDuration(nearestValues[0]),
			value: nearestValues[1]
		};
	});
	const tooltipCallback = (event: MouseEvent) => {
		tooltipXOffset = event.offsetX;
	};
</script>

<button class="btn" onclick={resetZoom}>Reset zoom</button>
<svg
	bind:this={svgElement}
	{width}
	{height}
	viewBox={`0 0 ${width} ${height}`}
	onmousemove={tooltipCallback}
	role="img"
	class="select-none"
>
	<clipPath id="clip-path">
		<rect
			x={marginLeft}
			y={marginRight}
			width={width - marginLeft - marginRight}
			height={height - marginTop - marginBottom}
		/>
	</clipPath>
	<path
		bind:this={path}
		clip-path="url(#clip-path)"
		fill="none"
		class="stroke-neutral"
		stroke-width="1.5"
		d={line(values, x)}
	/>
	<g bind:this={gx} transform="translate(0 {height - marginBottom})" />
	<g bind:this={gy} transform="translate({marginLeft} 0)" />
	{#if tooltip}
		<g
			transform={`translate(${tooltip.xOffset},${tooltip.yOffset})`}
			pointer-events="none"
			font-family="sans-serif"
			font-size="10"
			text-anchor="middle"
		>
			<rect x="-27" width="54" y="-30" height="24" class="fill-base-100" />
			<text y="-22">{tooltip.timestamp}</text>
			<text y="-12">{tooltip.value.toFixed(2)}</text>
			<circle r="3.5" class="fill-accent" />
		</g>
	{/if}
</svg>
