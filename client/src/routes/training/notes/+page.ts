import type { PageLoad } from './$types';
import { fetchTrainingNotes } from '$lib/api/training';

export const load: PageLoad = async ({ fetch, depends }) => {
	depends('app:training-notes');

	return {
		notes: fetchTrainingNotes(fetch, depends).then((notes) =>
			notes.toSorted((a, b) => (a.date > b.date ? -1 : 1))
		)
	};
};
