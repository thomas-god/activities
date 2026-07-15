/**
 *  Convert snake_case or camelCase to Title Case
 */
export const toTitleCase = (metric: string): string => {
	return metric
		.replace(/([A-Z])/g, ' $1')
		.replace(/_/g, ' ')
		.split(' ')
		.map((word) => word.charAt(0).toUpperCase() + word.slice(1).toLowerCase())
		.join(' ')
		.trim();
};
