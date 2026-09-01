<script lang="ts">
	import type { SportCategory } from '$lib/sport';
	import {
		Bike,
		Dumbbell,
		Flame,
		Footprints,
		Icon,
		Kayak,
		MountainSnow,
		SportShoe,
		WavesLadder,
		type LucideProps
	} from '@lucide/svelte';
	import { skis, soccerPitch, tennisRacket, wavesSharkFin } from '@lucide/lab';

	interface SportIconProps extends LucideProps {
		sport: SportCategory | 'Other' | null;
	}

	let { sport, ...props }: SportIconProps = $props();
</script>

{#if sport === 'Climbing'}
	<MountainSnow {...props} />
{:else if sport === 'Running'}
	<SportShoe {...props} />
{:else if sport === 'Cycling'}
	<Bike {...props} />
{:else if sport === 'Cardio'}
	<Dumbbell {...props} />
{:else if sport === 'Racket'}
	<Icon iconNode={tennisRacket} {...props} />
{:else if sport === 'Rowing'}
	<Kayak {...props} />
{:else if sport === 'Ski'}
	<Icon iconNode={skis} {...props} />
{:else if sport === 'Swimming'}
	<WavesLadder {...props} />
{:else if sport === 'TeamSports'}
	<Icon iconNode={soccerPitch} {...props} />
{:else if sport === 'Walking'}
	<Footprints {...props} />
{:else if sport === 'WaterSports'}
	<Icon iconNode={wavesSharkFin} {...props} />
{:else if sport === null || sport === 'Other'}
	<Flame {...props} />
{/if}
