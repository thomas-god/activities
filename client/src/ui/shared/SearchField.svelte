<script lang="ts">
	import { PUBLIC_APP_URL } from '$env/static/public';
	import { none, some, type Option } from '$lib/Options';
	import { Search } from '@lucide/svelte';
	import { tick } from 'svelte';

	export interface SearchResult {
		kind: 'activity' | 'training_note';
		id: string;
	}
	let { searchResults = $bindable() }: { searchResults: Option<SearchResult[]> } = $props();

	let searchPattern: string | null = $state(null);
	let showInput = $state(false);
	let inputElement: HTMLInputElement;

	const openAndFocus = async () => {
		showInput = true;
		// wait for the label to lose its `hidden` class before focusing,
		// otherwise focus() silently no-ops on a display:none element
		await tick();
		inputElement.focus();
	};

	const searchForPattern = async (pattern: string) => {
		searchPattern = pattern;
		if (searchPattern !== null && searchPattern !== '') {
			const res = await fetch(`${PUBLIC_APP_URL}/api/search?pattern=${searchPattern}`, {
				method: 'GET',
				mode: 'cors',
				credentials: 'include'
			});

			const found = await res.json();
			searchResults = some(found);
		} else {
			searchResults = none();
		}
	};
</script>

<button class={`btn btn-sm ${showInput ? 'hidden' : ''}`} onclick={openAndFocus}>
	<Search class="size-4" />
</button>

<label class={`input input-sm ${showInput ? '' : 'hidden'}`}>
	<Search class="size-4" />
	<input
		type="search"
		required
		placeholder="Search"
		bind:this={inputElement}
		bind:value={() => searchPattern || '', searchForPattern}
	/>
</label>
