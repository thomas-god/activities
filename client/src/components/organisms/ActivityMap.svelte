<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import type { Timeseries } from '$lib/api/activities';

	interface Props {
		timeseries: Timeseries;
	}

	let { timeseries }: Props = $props();

	let mapContainer: HTMLDivElement;
	let map: import('leaflet').Map | null = null;

	const gpsPoints = $derived.by(() => {
		const lats = timeseries.metrics['Latitude']?.values;
		const lons = timeseries.metrics['Longitude']?.values;

		if (!lats || !lons) return [];

		const points: [number, number][] = [];
		for (let i = 0; i < lats.length; i++) {
			const lat = lats[i];
			const lon = lons[i];
			if (lat !== null && lon !== null) {
				points.push([lat, lon]);
			}
		}
		return points;
	});

	onMount(async () => {
		if (gpsPoints.length === 0) return;

		const L = (await import('leaflet')).default;

		// Import leaflet CSS
		await import('leaflet/dist/leaflet.css');

		map = L.map(mapContainer, { zoomControl: true });

		L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png', {
			attribution:
				'© <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors',
			maxZoom: 19
		}).addTo(map);

		const polyline = L.polyline(gpsPoints, {
			color: '#3b82f6',
			weight: 3,
			opacity: 0.85
		}).addTo(map);

		// Recenter control
		const RecenterControl = L.Control.extend({
			options: { position: 'topleft' },
			onAdd() {
				const btn = L.DomUtil.create('button', 'leaflet-bar leaflet-control');
				btn.title = 'Fit to track';
				btn.innerHTML =
					'<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 3 3 10.53M21 3l-6.5 18-4-9-9-4 18-5.47z"/></svg>';
				btn.style.cssText =
					'width:30px;height:30px;display:flex;align-items:center;justify-content:center;background:#fff;cursor:pointer;color:#333;';
				L.DomEvent.on(btn, 'click', (e) => {
					L.DomEvent.stopPropagation(e);
					map!.fitBounds(polyline.getBounds(), { padding: [20, 20] });
				});
				return btn;
			}
		});
		new RecenterControl().addTo(map);

		// Start marker
		L.circleMarker(gpsPoints[0], {
			radius: 7,
			fillColor: '#22c55e',
			color: '#fff',
			weight: 2,
			opacity: 1,
			fillOpacity: 1
		})
			.bindTooltip('Start')
			.addTo(map);

		// End marker
		L.circleMarker(gpsPoints[gpsPoints.length - 1], {
			radius: 7,
			fillColor: '#ef4444',
			color: '#fff',
			weight: 2,
			opacity: 1,
			fillOpacity: 1
		})
			.bindTooltip('End')
			.addTo(map);

		map.fitBounds(polyline.getBounds(), { padding: [20, 20] });
	});

	onDestroy(() => {
		map?.remove();
		map = null;
	});
</script>

<div bind:this={mapContainer} class="h-full w-full rounded-box"></div>
