export const BONK_STATUS_VALUES = ['none', 'bonked'] as const;
export type BonkStatus = (typeof BONK_STATUS_VALUES)[number];

export type Nutrition = {
	bonk_status: BonkStatus;
	details: string | null;
};

export const getBonkStatusLabel = (status: BonkStatus | null): string => {
	if (status === null) return 'Not set';
	switch (status) {
		case 'none':
			return 'No bonk';
		case 'bonked':
			return 'Bonked';
		default:
			return 'Unknown';
	}
};

export const getBonkStatusColor = (status: BonkStatus | null): string => {
	if (status === null) return '';
	switch (status) {
		case 'none':
			return 'badge-success';
		case 'bonked':
			return 'badge-warning';
		default:
			return '';
	}
};

export const getBonkStatusIcon = (status: BonkStatus | null): string => {
	if (status === null) return '❓';
	switch (status) {
		case 'none':
			return '✅';
		case 'bonked':
			return '⚠️';
		default:
			return '❓';
	}
};
