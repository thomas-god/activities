<script lang="ts">
	import '../app.css';
	import { loadTheme, setTheme } from '#lib/contexts/theme.js';
	import { setAuthInfo } from '#lib/contexts/auth.js';
	import { fetchAuthInfo, type AuthInfo } from '#lib/api/index.js';
	import { none, setInnerValue, type Option } from '#lib/Options.js';

	let { children } = $props();

	// Initialize app-wide contexts
	let theme = $state(loadTheme());
	setTheme(theme);
	let authInfo: Option<AuthInfo> = $state(none());
	setAuthInfo(authInfo);
	setInnerValue(authInfo, await fetchAuthInfo());
</script>

<div class="page-container">
	{@render children?.()}
</div>

<style>
	:global(html),
	:global(body) {
		height: 100%;
		margin: 0;
	}
	.page-container {
		height: 100%;
		max-width: 1350px;
		margin: 0 auto;
		padding: 12px 8px;

		@media (min-width: 640px) {
			padding: 20px;
		}
	}
</style>
