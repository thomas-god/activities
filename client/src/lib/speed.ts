export const speedToPace = (speed: number): number => {
	return (1 / speed) * 60;
};

export const paceToString = (pace: number): string => {
	const minutes = Math.floor(pace);
	const seconds = Math.round((pace - minutes) * 60);
	return `${minutes}:${seconds.toString().padStart(2, '0')}/km`;
};
