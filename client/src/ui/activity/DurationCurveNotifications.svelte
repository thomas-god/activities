<script lang="ts">
	import {
		computeDurationCurveRecords,
		formatCurveDuration,
		formatCurveValue,
		type DurationCurveReference
	} from '#lib/durationCurve.js';
	import { TrendingUp } from '@lucide/svelte';

	interface Props {
		/** The activity's own duration curve values, indexed like `DURATION_CURVE_DURATIONS`. */
		curveValues: (number | null)[];
		/** Reference curves the activity is compared against (e.g. the 12-week best). */
		references: DurationCurveReference[];
		/** Unit of the curve values, as returned by the backend (e.g. `W`, `km/h`). */
		unit: string;
	}

	let { curveValues, references, unit }: Props = $props();

	let groups = $derived(computeDurationCurveRecords(curveValues, references));
</script>

{#if groups.length > 0}
	<div class="mx-2 mb-2 flex flex-col gap-3 rounded-box border border-base-300 bg-base-200/40 p-3">
		{#each groups as group (group.label)}
			<div>
				<h4 class="mb-1 flex items-center gap-1.5 text-sm font-semibold">
					<TrendingUp class="size-4 text-success" />
					New {group.label} bests
				</h4>
				<ul class="flex flex-col gap-0.5">
					{#each group.records as record (record.duration)}
						<li class="flex flex-wrap items-baseline gap-x-2 text-sm">
							<span class="w-14 shrink-0 font-medium opacity-80">
								{formatCurveDuration(record.duration)}
							</span>
							{#if record.previousValue !== null}
								<span class="opacity-60">
									{formatCurveValue(record.previousValue, unit)}
								</span>
								<span class="opacity-60">→</span>
							{/if}
							<span class="font-semibold text-success">
								{formatCurveValue(record.newValue, unit)}
							</span>
						</li>
					{/each}
				</ul>
			</div>
		{/each}
	</div>
{/if}
