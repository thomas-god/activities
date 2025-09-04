import * as z from 'zod';

export const ActivityListItem = z.object({
	id: z.string(),
	sport: z.string(),
	duration: z.number(),
	start_time: z.iso.datetime({ offset: true })
});

export const ActivityList = z.array(ActivityListItem);

export type ActivityList = z.infer<typeof ActivityList>;
export type ActivityListItem = z.infer<typeof ActivityListItem>;
