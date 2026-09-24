<script lang="ts">
	import { isSome, type Option } from '$lib/Options';
	import { formatTooltipValue } from '.';

	export interface TooltipData {
		visible: boolean;
		source: 'local' | 'external';
		x: number;
		y: number;
		showBelow: boolean;
		time: string;
		group: string;
		value: number;
		total: Option<number>;
	}

	/* eslint-disable svelte/no-unused-props */
	let {
		data,
		format,
		unit,
		primaryTimeFormatter,
		secondaryTimeFormatter
	}: {
		data: TooltipData;
		format: 'number' | 'duration' | 'pace';
		unit: string;
		primaryTimeFormatter: (time: string) => string;
		secondaryTimeFormatter: Option<(time: string) => string>;
	} = $props();
</script>

{#if data.visible}
	<foreignObject
		x={Math.round(data.x)}
		y={data.showBelow ? Math.round(data.y) + 10 : Math.round(data.y) - 90}
		width="200"
		height="100"
		class="pointer-events-none overflow-visible"
	>
		<div xmlns="http://www.w3.org/1999/xhtml" class="fixed">
			<div class="rounded-box bg-base-300 px-3 py-2 text-sm shadow-lg">
				<div class="flex flex-col gap-1">
					<div class="font-semibold">
						{primaryTimeFormatter(data.time)}
						{#if isSome(secondaryTimeFormatter)}
							<span class="text-xs font-light italic">
								•
								{secondaryTimeFormatter.value(data.time)}
							</span>
						{/if}
					</div>
					<div class="text-xs opacity-80">
						<span>{data.group}</span>
						<span>•</span>
						<span>{formatTooltipValue(data.value, format, unit)}</span>
					</div>
					{#if isSome(data.total)}
						<div class="text-xs opacity-60">
							<span>Total</span>
							<span>•</span>
							<span>{formatTooltipValue(data.total.value, format, unit)}</span>
						</div>
					{/if}
				</div>
			</div>
		</div>
	</foreignObject>
{/if}
