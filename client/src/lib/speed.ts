export const speedToPace = (speed: number): number => {
	return (1 / speed) * 60;
};

export const paceToString = (pace: number, pad = false): string => {
	const minutes = Math.floor(pace);
	const seconds = Math.round((pace - minutes) * 60);

	return `${pad ? minutes.toString().padStart(2, '0') : minutes}:${seconds.toString().padStart(2, '0')}`;
};
