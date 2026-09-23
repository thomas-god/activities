import type { TrainingMetric, TrainingPeriodDetails } from '$lib/api';
import { asOption, isNone, map, none, some, unwrapOr, type Option } from '$lib/Options';
import type { CompareAlignment, TrainingMetricGranularity } from '$lib/trainingMetric';
import { dayjs, granularityUnits } from '$lib/duration';
import { expectedBinsForDomain, type TimeDomain } from '$ui/training_metrics';

export const numberOfDistinctBins = (
	start: string,
	end: string | null,
	granularity: TrainingMetricGranularity | null
): number => {
	return unwrapOr(
		map(
			expectedBinsForDomain(some({ start: start, end: end }), asOption(granularity), dayjs()),
			(b) => b.length
		),
		0
	);
};

export const computeMissingNumberOfBins = (
	first: { metric: TrainingMetric; period: TrainingPeriodDetails },
	second: { metric: TrainingMetric; period: TrainingPeriodDetails }
): { first: Option<number>; second: Option<number> } => {
	const binsFirst = numberOfDistinctBins(
		first.period.start,
		first.period.end,
		first.metric.granularity
	);
	const binsSecond = numberOfDistinctBins(
		second.period.start,
		second.period.end,
		second.metric.granularity
	);
	return {
		first: binsFirst < binsSecond ? some(binsSecond - binsFirst) : none(),
		second: binsSecond < binsFirst ? some(binsFirst - binsSecond) : none()
	};
};

export const computeExtendedTimeDomain = (
	period: TrainingPeriodDetails,
	binsDelta: Option<number>,
	granularity: Option<TrainingMetricGranularity>,
	alignBy: CompareAlignment,
	now: dayjs.Dayjs
): TimeDomain => {
	const start = period.start;
	const end = period.end === null ? now.format('YYYY-MM-DD') : period.end;

	if (isNone(binsDelta)) {
		return some({ start: period.start, end: end });
	}

	const deltaGranularity = granularityUnits(granularity);

	// to align the starts we have to extend the end, and vice-versa
	return some({
		start:
			alignBy === 'end'
				? dayjs(start).subtract(binsDelta.value, deltaGranularity.add).format('YYYY-MM-DD')
				: start,
		end:
			alignBy === 'start'
				? dayjs(end).add(binsDelta.value, deltaGranularity.add).format('YYYY-MM-DD')
				: end
	});
};
