# /// script
# dependencies = [
#   "requests>=2.31.0",
# ]
# ///
"""Reset demo environment with fresh data using the API."""

from math import floor
import sys
import time
from typing import Any
import random
from datetime import datetime, timedelta
from pathlib import Path


import requests

TEMP_DIR = Path("./tmp/demo-tcx")
API_URL = "http://activities/api"


def wait_for_api() -> None:
    """Wait for the API to be available."""
    print("Waiting for API to be available...")
    while True:
        try:
            response = requests.get(f"{API_URL}/activities", timeout=2)
            if response.status_code == 200:
                print("API is available, proceeding with reset...")
                return
        except requests.exceptions.RequestException:
            pass
        time.sleep(2)


def delete_all_activities() -> None:
    """Delete all existing activities."""
    print("Deleting all existing activities...")
    response = requests.get(f"{API_URL}/activities")
    if response.status_code != 200:
        print(f"Failed to fetch activities: {response.status_code}")
        return

    activities: list[dict[str, Any]] = response.json()
    for activity in activities:
        activity_id = activity["id"]
        del_response = requests.delete(f"{API_URL}/activity/{activity_id}")
        if del_response.status_code == 200:
            print(f"Deleted activity: {activity_id}")
        else:
            print(f"Failed to delete activity {activity_id}: {del_response.status_code}")


def delete_all_periods() -> None:
    """Delete all existing training periods."""
    print("Deleting all existing training periods...")
    response = requests.get(f"{API_URL}/training/periods")
    if response.status_code != 200:
        print(f"Failed to fetch periods: {response.status_code}")
        return

    periods: list[dict[str, Any]] = response.json()
    for period in periods:
        period_id = period["id"]
        del_response = requests.delete(f"{API_URL}/training/period/{period_id}")
        if del_response.status_code == 200:
            print(f"Deleted period: {period_id}")
        else:
            print(f"Failed to delete period {period_id}: {del_response.status_code}")


def delete_all_notes() -> None:
    """Delete all existing training notes."""
    print("Deleting all existing training notes...")
    response = requests.get(f"{API_URL}/training/notes")
    if response.status_code != 200:
        print(f"Failed to fetch notes: {response.status_code}")
        return

    notes: list[dict[str, Any]] = response.json()
    for note in notes:
        note_id = note["id"]
        del_response = requests.delete(f"{API_URL}/training/note/{note_id}")
        if del_response.status_code == 200:
            print(f"Deleted note: {note_id}")
        else:
            print(f"Failed to delete note {note_id}: {del_response.status_code}")


def delete_all_metrics() -> None:
    """Delete all existing training metrics."""
    print("Deleting all existing training metrics...")
    try:
        response = requests.get(
            f"{API_URL}/training/metrics?start=2020-01-01T00:00:00%2B00:00"
        )
        if response.status_code != 200:
            print(f"Failed to fetch metrics: {response.status_code}")
            return

        metrics: list[dict[str, Any]] = response.json()
        for metric in metrics:
            metric_id = metric["id"]
            del_response = requests.delete(f"{API_URL}/training/metric/{metric_id}")
            if del_response.status_code == 200:
                print(f"Deleted metric: {metric_id}")
            else:
                print(
                    f"Failed to delete metric {metric_id}: {del_response.status_code}"
                )
    except Exception as e:
        print(f"Error deleting metrics: {e}")


def generate_tcx(
    date: str,
    sport: str,
    duration_seconds: int,
    distance_meters: int,
    avg_hr: int,
    filename: str,
) -> None:
    """Generate a TCX file with activity data."""
    # Random start time between 6am and 6pm
    hour = random.randint(6, 17)
    minute = random.randint(0, 59)
    start_time = f"{date}T{hour:02d}:{minute:02d}:00Z"

    # Calculate trackpoint data
    num_points = floor(duration_seconds)
    time_step = 1
    dist_step = distance_meters / num_points

    # Parse start time to timestamp for trackpoint calculation
    start_dt = datetime.fromisoformat(start_time.replace("Z", "+00:00"))

    # Build TCX content
    tcx_content = f"""<?xml version="1.0" encoding="UTF-8"?>
<TrainingCenterDatabase xsi:schemaLocation="http://www.garmin.com/xmlschemas/TrainingCenterDatabase/v2 http://www.garmin.com/xmlschemas/TrainingCenterDatabasev2.xsd"
  xmlns:ns5="http://www.garmin.com/xmlschemas/ActivityGoals/v1"
  xmlns:ns3="http://www.garmin.com/xmlschemas/ActivityExtension/v2"
  xmlns:ns2="http://www.garmin.com/xmlschemas/UserProfile/v2"
  xmlns="http://www.garmin.com/xmlschemas/TrainingCenterDatabase/v2"
  xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:ns4="http://www.garmin.com/xmlschemas/ProfileExtension/v1">
  <Activities>
    <Activity Sport="{sport}">
      <Id>{start_time}</Id>
      <Lap StartTime="{start_time}">
        <TotalTimeSeconds>{duration_seconds}</TotalTimeSeconds>
        <DistanceMeters>{distance_meters}</DistanceMeters>
        <Calories>{duration_seconds * 10 // 60}</Calories>
        <AverageHeartRateBpm><Value>{avg_hr}</Value></AverageHeartRateBpm>
        <MaximumHeartRateBpm><Value>{avg_hr + 20}</Value></MaximumHeartRateBpm>
        <Intensity>Active</Intensity>
        <TriggerMethod>Manual</TriggerMethod>
        <Track>
"""

    # Generate trackpoints
    for i in range(num_points + 1):
        point_time = i * time_step
        point_dist = i * dist_step
        hr_variation = random.randint(-5, 5)
        point_hr = avg_hr + hr_variation
        point_timestamp = (start_dt + timedelta(seconds=point_time)).strftime(
            "%Y-%m-%dT%H:%M:%SZ"
        )

        tcx_content += f"""          <Trackpoint>
            <Time>{point_timestamp}</Time>
            <DistanceMeters>{point_dist}</DistanceMeters>
            <HeartRateBpm><Value>{point_hr}</Value></HeartRateBpm>
            <Cadence>{80 + random.randint(-5, 5)}</Cadence>
              <Extensions>
              <ns3:TPX>
                 <ns3:Speed>{3 * random.random()}</ns3:Speed>
                <ns3:Watts>{150 + random.randint(-50, 50)}</ns3:Watts>
              </ns3:TPX>
            </Extensions>
          </Trackpoint>
"""

    tcx_content += """        </Track>
      </Lap>
    </Activity>
  </Activities>
</TrainingCenterDatabase>
"""

    # Write file
    output_path = TEMP_DIR / filename
    output_path.write_text(tcx_content)


def upload_activities() -> list[str]:
    """Upload all generated TCX files and return created activity IDs."""
    print("Uploading activities to API...")
    activity_ids = []

    for tcx_file in sorted(TEMP_DIR.glob("*.tcx")):
        filename = tcx_file.name
        with open(tcx_file, "rb") as f:
            files = {"file.tcx": (filename, f, "application/xml")}
            response = requests.post(f"{API_URL}/activity", files=files)

        if response.status_code == 201:
            print(f"Uploading {filename}... 201 - Response: {response.text}")
        else:
            print(f"Uploading {filename}... {response.status_code} - {response.text}")

    print(f"Total activities created: {len(activity_ids)}")
    return activity_ids


def create_training_period(
    name: str, start: str, end: str, description: str
) :
    """Create a training period."""
    payload = {
        "name": name,
        "start": start,
        "end": end,
        "description": description,
        "sports": None,
    }
    response = requests.post(
        f"{API_URL}/training/period", json=payload, headers={"Content-Type": "application/json"}
    )
    print(f"Period ({name}): {response.status_code}")
    if response.status_code == 201:
        data = response.text
        print(data)
        return data
    else:
        print(response.text)
    return None


def create_training_metric(
    source: dict[str, str],
    granularity: str,
    aggregate: str,
    metric_start: str,
    today: str,
    group_by: str | None = None,
) -> dict[str, Any] | None:
    """Create a training metric."""
    payload: dict[str, Any] = {
        "source": source,
        "granularity": granularity,
        "aggregate": aggregate,
        "filters": {},
        "initial_date_range": {"start": metric_start, "end": today},
    }
    if group_by:
        payload["group_by"] = group_by

    response = requests.post(
        f"{API_URL}/training/metric", json=payload, headers={"Content-Type": "application/json"}
    )
    if response.status_code == 201:
        data = response.text
        print(f"Creating metric... 201 - {data}")
        return data
    else:
        print(f"Creating metric... {response.status_code} - {response.text}")
    return None


def create_training_note(date: str, content: str) -> dict[str, Any] | None:
    """Create a training note."""
    payload = {"date": date, "content": content}
    response = requests.post(
        f"{API_URL}/training/note", json=payload, headers={"Content-Type": "application/json"}
    )
    if response.status_code == 201:
        data = response.json()
        print(f"Creating note for {date}... 201 - {data}")
        return data
    else:
        print(f"Creating note for {date}... {response.status_code} - {response.text}")
    return None


def generate_demo_data() -> int:
    """Generate demo data."""
    today = datetime.now().date()
    today_str = today.strftime("%Y-%m-%d")

    print(f"Generating demo data with dates relative to today: {today_str}")

    # Create temp directory
    TEMP_DIR.mkdir(parents=True, exist_ok=True)

    # Generate 12 weeks of activities (4 per week)
    print("Generating activities...")
    for week in range(12):
        # Activity 1: Easy run
        days_ago = week * 7 + 1
        date = (today - timedelta(days=days_ago)).strftime("%Y-%m-%d")
        generate_tcx(date, "Running", 1800, 5000, 135, f"run_easy_week{week}_1.tcx")

        # Activity 2: Tempo run
        days_ago = week * 7 + 3
        date = (today - timedelta(days=days_ago)).strftime("%Y-%m-%d")
        generate_tcx(date, "Running", 2400, 8000, 155, f"run_tempo_week{week}_2.tcx")

        # Activity 3: Bike ride
        days_ago = week * 7 + 5
        date = (today - timedelta(days=days_ago)).strftime("%Y-%m-%d")
        generate_tcx(date, "Biking", 5400, 40000, 140, f"bike_week{week}_3.tcx")

        # Activity 4: Long run (weekend)
        days_ago = week * 7 + 6
        date = (today - timedelta(days=days_ago)).strftime("%Y-%m-%d")
        generate_tcx(date, "Running", 4500, 15000, 145, f"run_long_week{week}_4.tcx")

    # Upload activities
    activity_ids = upload_activities()

    # Create training periods
    print("\nCreating training periods...")

    period1_start = (today - timedelta(days=84)).strftime("%Y-%m-%d")
    period1_end = (today - timedelta(days=64)).strftime("%Y-%m-%d")
    create_training_period(
        "Base Building",
        period1_start,
        period1_end,
        "Building aerobic base with easy miles",
    )

    period2_start = (today - timedelta(days=56)).strftime("%Y-%m-%d")
    period2_end = (today - timedelta(days=36)).strftime("%Y-%m-%d")
    create_training_period(
        "Build Phase",
        period2_start,
        period2_end,
        "Introducing tempo runs and longer rides",
    )

    period3_start = (today - timedelta(days=28)).strftime("%Y-%m-%d")
    period3_end = (today - timedelta(days=8)).strftime("%Y-%m-%d")
    create_training_period(
        "Peak Training",
        period3_start,
        period3_end,
        "Maximum training load with quality workouts",
    )

    period4_start = (today - timedelta(days=6)).strftime("%Y-%m-%d")
    period4_end = today_str
    create_training_period(
        "Race Week", period4_start, period4_end, "Taper week with reduced volume"
    )

    # Create training metrics
    print("\nCreating training metrics...")
    metric_start = (today - timedelta(days=120)).strftime("%Y-%m-%d")

    create_training_metric(
        {"Statistic": "Calories"}, "Weekly", "Sum", metric_start, today_str
    )

    create_training_metric(
        {"Statistic": "Duration"},
        "Weekly",
        "Sum",
        metric_start,
        today_str,
        group_by="SportCategory",
    )

    # Create training notes
    print("\nCreating training notes...")

    note_date = (today - timedelta(days=3)).strftime("%Y-%m-%d")
    create_training_note(
        note_date,
        "Felt great during today's tempo run. HR was well controlled and pace felt comfortable. Ready for race week!",
    )

    note_date = (today - timedelta(days=10)).strftime("%Y-%m-%d")
    create_training_note(
        note_date,
        "Completed longest run of the training block. Legs felt tired but recovered well with proper nutrition and stretching.",
    )

    # Cleanup
    import shutil

    shutil.rmtree(TEMP_DIR)

    print("\nDemo data generation complete!")
    print("Generated:")
    print("  - 48 activities (12 weeks × 4 activities/week)")
    print("  - 4 training periods")
    print("  - 2 training metrics")
    print("  - 2 training notes")

    return 0


def main() -> int:
    """Reset demo environment."""
    print(f"Starting demo reset at {time.strftime('%Y-%m-%d %H:%M:%S')}")

    wait_for_api()

    delete_all_activities()
    delete_all_periods()
    delete_all_notes()
    delete_all_metrics()

    generate_demo_data()

    print(f"Demo reset completed at {time.strftime('%Y-%m-%d %H:%M:%S')}")
    return 0


if __name__ == "__main__":
    print("hello world")
    sys.exit(main())
