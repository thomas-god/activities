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
            print(
                f"Failed to delete activity {activity_id}: {del_response.status_code}"
            )


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
            data = response.json()
            created_ids = data.get("created_ids", [])
            activity_ids.extend(created_ids)
            print(f"Uploading {filename}... 201 - Created ID(s): {created_ids}")
        else:
            print(f"Uploading {filename}... {response.status_code} - {response.text}")

    print(f"Total activities created: {len(activity_ids)}")
    return activity_ids


def update_activity(
    activity_id: str,
    name: str | None = None,
    rpe: int | None = None,
    workout_type: str | None = None,
    bonk_status: str | None = None,
    nutrition_details: str | None = None,
    feedback: str | None = None,
) -> bool:
    """Update activity metadata using PATCH endpoint."""
    # Build query parameters
    params = {}
    if name is not None:
        params["name"] = name
    if rpe is not None:
        params["rpe"] = rpe
    if workout_type is not None:
        params["workout_type"] = workout_type
    if bonk_status is not None:
        params["bonk_status"] = bonk_status
    if nutrition_details is not None:
        params["nutrition_details"] = nutrition_details

    # Build request body for feedback
    body = None
    if feedback is not None:
        body = {"feedback": feedback}

    # Make PATCH request
    response = requests.patch(
        f"{API_URL}/activity/{activity_id}",
        params=params,
        json=body if body else None,
        headers={"Content-Type": "application/json"} if body else None,
    )

    if response.status_code == 200:
        print(f"Updated activity {activity_id}")
        return True
    else:
        print(
            f"Failed to update activity {activity_id}: {response.status_code} - {response.text}"
        )
        return False


def generate_activity(
    date: str,
    sport: str,
    duration_seconds: int,
    distance_meters: int,
    avg_hr: int,
    name: str | None = None,
    rpe: int | None = None,
    workout_type: str | None = None,
    bonk_status: str | None = None,
    nutrition_details: str | None = None,
    feedback: str | None = None,
) -> str | None:
    """Generate a TCX file, upload it, and update with additional metadata.

    Returns the created activity ID if successful, None otherwise.
    """
    # Generate unique filename
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S_%f")
    filename = f"activity_{timestamp}.tcx"

    # Generate TCX file
    generate_tcx(date, sport, duration_seconds, distance_meters, avg_hr, filename)

    # Upload the file
    tcx_file = TEMP_DIR / filename
    with open(tcx_file, "rb") as f:
        files = {"file.tcx": (filename, f, "application/xml")}
        response = requests.post(f"{API_URL}/activity", files=files)

    if response.status_code != 201:
        print(f"Failed to upload activity: {response.status_code} - {response.text}")
        return None

    # # Extract created activity ID
    # print(f"Upload response (status {response.status_code}):")
    # print(f"  Content-Type: {response.headers.get('content-type')}")
    # print(f"  Body length: {len(response.text)}")
    # print(f"  Body: {response.text}")

    if not response.text:
        print("Response body is empty!")
        return None

    try:
        data = response.json()
        created_ids = data.get("created_ids", [])
        if not created_ids:
            print("No activity ID returned from upload")
            return None
    except Exception as e:
        print(f"Failed to parse JSON: {e}")
        return None

    activity_id = created_ids[0]
    print(f"Created activity {activity_id}")

    # Update activity with additional metadata
    if any([name, rpe, workout_type, bonk_status, nutrition_details, feedback]):
        update_activity(
            activity_id,
            name=name,
            rpe=rpe,
            workout_type=workout_type,
            bonk_status=bonk_status,
            nutrition_details=nutrition_details,
            feedback=feedback,
        )

    return activity_id


def create_training_period(name: str, start: str, end: str, description: str):
    """Create a training period."""
    payload = {
        "name": name,
        "start": start,
        "end": end,
        "note": description,
        "sports": None,
    }
    response = requests.post(
        f"{API_URL}/training/period",
        json=payload,
        headers={"Content-Type": "application/json"},
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
    name: str,
    source: dict[str, str],
    granularity: str,
    aggregate: str,
    metric_start: str,
    today: str,
    group_by: str | None = None,
) -> dict[str, Any] | None:
    """Create a training metric."""
    payload: dict[str, Any] = {
        "name": name,
        "source": source,
        "granularity": granularity,
        "aggregate": aggregate,
        "filters": {},
        "initial_date_range": {"start": metric_start, "end": today},
    }
    if group_by:
        payload["group_by"] = group_by

    response = requests.post(
        f"{API_URL}/training/metric",
        json=payload,
        headers={"Content-Type": "application/json"},
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
        f"{API_URL}/training/note",
        json=payload,
        headers={"Content-Type": "application/json"},
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

    # Find the most recent Monday (or today if it's Monday)
    days_since_monday = today.weekday()  # 0=Monday, 6=Sunday
    most_recent_monday = today - timedelta(days=days_since_monday)
    print(f"Aligning activities to weeks starting from Monday: {most_recent_monday}")

    # Generate 12 weeks of activities (4 per week)
    print("Generating activities...")

    for week in range(12):
        week_start = most_recent_monday - timedelta(weeks=week)

        # Activity 1: Easy run (Tuesday)
        date = week_start + timedelta(days=1)
        duration = random.randint(25, 35) * 60  # 25-35 minutes
        distance = random.randint(4000, 6000)  # 4-6 km
        if date < today:
            generate_activity(
                date=date.strftime("%Y-%m-%d"),
                sport="Running",
                duration_seconds=duration,
                distance_meters=distance,
                avg_hr=random.randint(130, 140),
                name="Easy Recovery Run",
                rpe=random.randint(2, 4),
                workout_type="easy",
            )

        # Activity 2: Tempo run (Thursday)
        date = week_start + timedelta(days=3)
        duration = random.randint(35, 45) * 60  # 35-45 minutes
        distance = random.randint(7000, 9000)  # 7-9 km
        if date < today:
            generate_activity(
                date=date.strftime("%Y-%m-%d"),
                sport="Running",
                duration_seconds=duration,
                distance_meters=distance,
                avg_hr=random.randint(150, 160),
                name="Tempo Run",
                rpe=random.randint(6, 7),
                workout_type="tempo",
                feedback="Solid tempo effort, maintained target pace",
            )

        # Activity 3: Bike ride (Saturday)
        date = week_start + timedelta(days=5)
        duration = random.randint(80, 100) * 60  # 80-100 minutes
        distance = random.randint(35000, 45000)  # 35-45 km
        if date < today:
            generate_activity(
                date=date.strftime("%Y-%m-%d"),
                sport="Biking",
                duration_seconds=duration,
                distance_meters=distance,
                avg_hr=random.randint(135, 145),
                name="Endurance Bike Ride",
                rpe=random.randint(3, 5),
                workout_type="easy",
            )

        # Activity 4: Long run (Sunday)
        date = week_start + timedelta(days=6)
        duration = random.randint(70, 90) * 60  # 70-90 minutes
        distance = random.randint(13000, 17000)  # 13-17 km
        if date < today:
            generate_activity(
                date=date.strftime("%Y-%m-%d"),
                sport="Running",
                duration_seconds=duration,
                distance_meters=distance,
                avg_hr=random.randint(140, 150),
                name="Long Run",
                rpe=random.randint(5, 7),
                workout_type="long_run",
                feedback="Legs felt strong throughout. Good hydration strategy.",
            )

    # Today's activity:
    generate_activity(
        date=today.strftime("%Y-%m-%d"),
        sport="Running",
        duration_seconds=3600,
        distance_meters=10000,
        avg_hr=random.randint(130, 140),
        name="An imported sport activity",
        rpe=random.randint(2, 4),
        workout_type="easy",
        feedback='On top of tracking the RPE, workout type and nutrition for each activtiy, you can add a description to an activity to track your feedback  and what\'s happened more precisely (e.g. "slight pain in my left calf during the first two kilometers", "skipped last rep because too much ice").',
    )

    # Create training periods
    print("\nCreating training periods...")

    period1_start = (today - timedelta(days=84)).strftime("%Y-%m-%d")
    period1_end = today_str
    create_training_period(
        "This is a training period",
        period1_start,
        period1_end,
        "Training periods help you keep track of specific training blocks, e.g. base training, race specific preparation, etc.",
    )

    # Create training metrics
    print("\nCreating training metrics...")
    metric_start = (today - timedelta(days=120)).strftime("%Y-%m-%d")

    create_training_metric(
        "Weekly calories",
        {"Statistic": "Calories"},
        "Weekly",
        "Sum",
        metric_start,
        today_str,
    )

    create_training_metric(
        "Weekly distance",
        {"Statistic": "Distance"},
        "Weekly",
        "Sum",
        metric_start,
        today_str,
        group_by="SportCategory",
    )

    create_training_metric(
        "Weekly duration",
        {"Statistic": "Duration"},
        "Weekly",
        "Sum",
        metric_start,
        today_str,
        group_by="WorkoutType",
    )

    # Create training notes
    print("\nCreating training notes...")

    note_date = (today - timedelta(days=1)).strftime("%Y-%m-%d")
    create_training_note(
        note_date,
        'Training notes are useful to track what\'s happening outside of a specific activity and can help add context to your training data (e.g. "bad sleep this week").\nYou can also use training notes to track important decisions like changing your training focus because you registered to a new race or event.',
    )

    note_date = (today - timedelta(days=12)).strftime("%Y-%m-%d")
    create_training_note(
        note_date,
        "Out of town these past 3 days with no access to a track, so pace was a bit off but legs felt good.",
    )

    note_date = (today - timedelta(days=9)).strftime("%Y-%m-%d")
    create_training_note(
        note_date,
        "Completed longest run of the training block. Legs felt tired but recovery was ok after 2 days.",
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
