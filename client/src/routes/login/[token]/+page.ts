import type { PageLoad } from './$types';

import { PUBLIC_APP_URL } from '$env/static/public';
import { goto } from '$app/navigation';
import { error } from '@sveltejs/kit';
import { resolve } from '$app/paths';

export const load: PageLoad = async ({ fetch, params }) => {
	const res = await fetch(`${PUBLIC_APP_URL}/api/login/validate/${params.token}`, {
		method: 'POST',
		credentials: 'include',
		mode: 'cors'
	});
	if (res.status === 200) {
		goto(resolve('/'));
	} else {
		error(res.status, 'login failed');
	}
};

export const prerender = false;
export const ssr = false;
