import type { PageLoad } from './$types';
import { fetchTrainingNotes } from '$lib/api/training';

export const load: PageLoad = async ({ fetch, depends }) => {
	depends('app:training-notes');

	const notes = await fetchTrainingNotes(fetch, depends);

	return { notes };
};
