<script lang="ts">
	import '../app.css';
	import { loadTheme, setTheme } from '#lib/contexts/theme.js';
	import { setAuthInfo, setCurrentUser } from '#lib/contexts/auth.js';
	import { fetchAuthInfo, fetchUserInfo, type AuthInfo, type UserInfo } from '#lib/api/index.js';
	import { none, setInnerValue, type Option } from '#lib/Options.js';

	let { children } = $props();

	// Initialize app-wide contexts
	let theme = $state(loadTheme());
	setTheme(theme);
	let authInfo: Option<AuthInfo> = $state(none());
	setAuthInfo(authInfo);
	let currentUser: Option<UserInfo> = $state(none());
	setCurrentUser(currentUser);

	const fetchedAuthInfo = await fetchAuthInfo();
	setInnerValue(authInfo, fetchedAuthInfo);

	// The current user only exists in multi-user mode and requires a session: fetch it without
	// blocking the initial render, so the admin navigation simply stays hidden until it resolves
	// (and on the login page where the request answers `401`).
	if (fetchedAuthInfo.strategy === 'MultiUser') {
		void fetchUserInfo()
			.then((user) => setInnerValue(currentUser, user))
			.catch(() => {});
	}
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
