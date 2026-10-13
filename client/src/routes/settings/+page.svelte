<script lang="ts">
	import Navbar from '#ui/navigation/Navbar.svelte';
	import AdminPage from './AdminPage.svelte';

	import UserPage from './UserPage.svelte';

	let { data } = $props();
</script>

<Navbar />

<UserPage userInfo={data.userInfo} />

<AdminPage adminInfo={data.adminInfo} />
