<script lang="ts">
	import { localiseDate } from '$lib/duration';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();
</script>

<div class="mx-auto flex flex-col gap-4">
	<div class="rounded-box rounded-t-none bg-base-100 shadow-md">
		<div>Training periods</div>
		<div>
			{#each data.periods as period}
				<div>
					{period.name} ({localiseDate(period.start)} - {period.end === null
						? 'Ongoing'
						: localiseDate(period.end)})
				</div>
			{:else}
				<div class="italic text-sm text-center tracking-wide opacity-60">No training periods</div>
			{/each}
		</div>
	</div>
</div>
