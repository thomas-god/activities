import { PUBLIC_APP_URL } from '$env/static/public';

export interface MetricsOrderingScope {
	type: 'global' | 'trainingPeriod';
	trainingPeriodId?: string;
}

export interface MetricsOrderingResponse {
	metric_ids: string[];
}

export async function getMetricsOrdering(
	scope: MetricsOrderingScope
): Promise<string[] | null> {
	try {
		const params = new URLSearchParams({ type: scope.type });
		if (scope.type === 'trainingPeriod' && scope.trainingPeriodId) {
			params.append('trainingPeriodId', scope.trainingPeriodId);
		}

		const response = await fetch(
			`${PUBLIC_APP_URL}/api/training/metrics/ordering?${params.toString()}`,
			{
				method: 'GET',
				credentials: 'include',
				mode: 'cors'
			}
		);

		if (response.ok) {
			const result: MetricsOrderingResponse = await response.json();
			return result.metric_ids || [];
		}

		console.error('Failed to fetch metrics ordering');
		return null;
	} catch (error) {
		console.error('Error fetching metrics ordering:', error);
		return null;
	}
}

export async function setMetricsOrdering(
	scope: MetricsOrderingScope,
	metricIds: string[]
): Promise<boolean> {
	try {
		const body: any = {
			type: scope.type,
			metric_ids: metricIds
		};

		if (scope.type === 'trainingPeriod' && scope.trainingPeriodId) {
			body.trainingPeriodId = scope.trainingPeriodId;
		}

		const response = await fetch(`${PUBLIC_APP_URL}/api/training/metrics/ordering`, {
			method: 'POST',
			credentials: 'include',
			mode: 'cors',
			headers: {
				'Content-Type': 'application/json'
			},
			body: JSON.stringify(body)
		});

		if (response.ok) {
			return true;
		}

		const error = await response.json();
		alert(error.error || 'Failed to save metrics ordering');
		return false;
	} catch (error) {
		alert('Error saving metrics ordering');
		console.error(error);
		return false;
	}
}
