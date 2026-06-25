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
from datetime import datetime, timedelta, time as dt_time
from pathlib import Path
import xml.etree.ElementTree as ET


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
        if del_response.status_code == 204:
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
        if del_response.status_code == 204:
            print(f"Deleted note: {note_id}")
        else:
            print(f"Failed to delete note {note_id}: {del_response.status_code}")


def delete_all_global_metrics() -> None:
    """Delete all existing global training metrics (i.e. those not linked to a training period)."""
    print("Deleting all existing training metrics...")
    try:
        response = requests.get(
            f"{API_URL}/training/metrics?start=2020-01-01T00:00:00%2B00:00&scope=global"
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


def upload_activity_from_content(
    file_content: bytes,
    filename: str | None = None,
    name: str | None = None,
    rpe: int | None = None,
    workout_type: str | None = None,
    bonk_status: str | None = None,
    nutrition_details: str | None = None,
    feedback: str | None = None,
) -> str | None:
    """Upload a single TCX activity from raw file content and apply metadata."""
    if filename is None:
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S_%f")
        filename = f"activity_{timestamp}.tcx"

    files = {"file.tcx": (filename, file_content, "application/xml")}
    response = requests.post(f"{API_URL}/activity", files=files)

    if response.status_code != 201:
        print(f"Failed to upload activity: {response.status_code} - {response.text}")
        return None

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


def create_training_period(
    name: str, start: str, end: str, description: str
) -> str | None:
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
        data = response.json()
        return data["id"]
    else:
        print(response.text)
    return None


def create_global_training_metric(
    name: str,
    metric: str,
    granularity: str | None,
    aggregate: str | None,
    group_by: str | None = None,
    average: bool = False,
) -> dict[str, Any] | None:
    """Create a training metric."""
    payload: dict[str, Any] = {
        "name": name,
        "metric": metric,
        "filters": {},
        "scope": {"type": "global"},
    }
    if aggregate:
        payload["window"] = {
            "granularity": granularity,
            "aggregate": aggregate,
        }
        if group_by:
            payload["window"]["group_by"] = group_by

    if average:
        payload["summary"] = {"average": {"include_zeros": False}}

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


def create_scoped_training_metric(
    name: str,
    metric: str,
    granularity: str | None,
    aggregate: str | None,
    period_id: str,
    group_by: str | None = None,
    average: bool = False,
) -> dict[str, Any] | None:
    """Create a training metric."""
    payload: dict[str, Any] = {
        "name": name,
        "metric": metric,
        "filters": {},
        "scope": {"type": "trainingPeriod", "trainingPeriodId": period_id},
    }

    if aggregate:
        payload["window"] = {
            "granularity": granularity,
            "aggregate": aggregate,
        }
        if group_by:
            payload["window"]["group_by"] = group_by

    if average:
        payload["summary"] = {"average": {"include_zeros": False}}

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


def adjust_tcx_file_time(file: str, start: datetime) -> bytes:
    tree = ET.parse(file)

    laps = tree.getroot().find("Activities").find("Activity").findall("Lap")
    initial_start = datetime.fromisoformat(laps[0].attrib["StartTime"]).replace(
        tzinfo=None
    )
    delta = start - initial_start
    previous_time = initial_start
    previous_distance = 0.0
    pause_time = timedelta(seconds=0)
    for lap in laps:
        lap_time = datetime.fromisoformat(lap.attrib["StartTime"]).replace(tzinfo=None)
        lap.attrib["StartTime"] = (lap_time + delta).strftime("%Y-%m-%dT%H:%M:%SZ")

        for point in lap.find("Track").findall("Trackpoint"):
            point_time = datetime.fromisoformat(point.find("Time").text).replace(
                tzinfo=None
            )
            if float(point.find("DistanceMeters").text) == previous_distance:
                if point_time - previous_time > timedelta(seconds=10):
                    pause_time += point_time - previous_time
            previous_distance = float(point.find("DistanceMeters").text)
            previous_time = point_time
            point.find("Time").text = (point_time + delta - pause_time).strftime(
                "%Y-%m-%dT%H:%M:%SZ"
            )

    return ET.tostring(tree.getroot(), encoding="utf8")


def easy_run(start: datetime) -> bytes:
    return adjust_tcx_file_time("easy_run.tcx", start)


def intervals(start: datetime) -> bytes:
    return adjust_tcx_file_time("intervals.tcx", start)


def long_ride(start: datetime) -> bytes:
    return adjust_tcx_file_time("long_ride.tcx", start)


def create_standalone_activity(
    calories: int,
    duration: int,
    date: datetime,
    sport: str,
    name: str | None = None,
    rpe: int | None = None,
    workout_type: str | None = None,
    bonk_status: str | None = None,
    nutrition_details: str | None = None,
    feedback: str | None = None,
):

    response = requests.post(
        f"{API_URL}/activity/standalone",
        json={
            "sport": sport,
            "calories": calories,
            "duration": duration,
            "start_time": date.strftime("%Y-%m-%dT%H:%M:%SZ"),
        },
    )

    if response.status_code != 201:
        print(
            f"Failed to create standalone activity: {response.status_code} - {response.text}"
        )
        return None

    if not response.text:
        print("Response body is empty!")
        return None

    try:
        data = response.json()
        created_id = data.get("id")
        if not created_id:
            print("No activity ID returned from upload")
            return None
    except Exception as e:
        print(f"Failed to parse JSON: {e}")
        return None

    print(f"Created activity {created_id}")

    if any([name, rpe, workout_type, bonk_status, nutrition_details, feedback]):
        update_activity(
            created_id,
            name=name,
            rpe=rpe,
            workout_type=workout_type,
            bonk_status=bonk_status,
            nutrition_details=nutrition_details,
            feedback=feedback,
        )

    return created_id


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
        date = week_start + timedelta(days=1) + timedelta(hours=8)
        if date < today:
            upload_activity_from_content(
                file_content=easy_run(datetime.combine(date, dt_time(hour=8))),
                name="Easy Recovery Run",
                rpe=random.randint(2, 4),
                workout_type="easy",
            )

        # Activity 2: Tempo run (Thursday)
        date = week_start + timedelta(days=3)
        if date < today:
            upload_activity_from_content(
                file_content=intervals(datetime.combine(date, dt_time(hour=8))),
                name="Tempo Run",
                rpe=random.randint(6, 7),
                workout_type="tempo",
                feedback="Solid tempo effort, maintained target pace",
            )

        # Activity 3: Bike ride (Saturday)
        date = week_start + timedelta(days=5)
        if date < today:
            upload_activity_from_content(
                file_content=long_ride(datetime.combine(date, dt_time(hour=8))),
                name="Long Ride",
                workout_type="long_run",
                rpe=random.randint(3, 5),
            )

        # Activity 4: Long run (Sunday)
        date = week_start + timedelta(days=6)
        if date < today:
            upload_activity_from_content(
                file_content=easy_run(datetime.combine(date, dt_time(hour=8))),
                name="Easy Recovery Run",
                rpe=random.randint(2, 4),
                workout_type="easy",
            )

    # Standalone activity
    create_standalone_activity(
        date=datetime.combine(today - timedelta(days=1), dt_time(hour=10)),
        calories=200,
        duration=45 * 60,
        sport="StrengthTraining",
        name="A standalone activity",
        rpe=random.randint(2, 4),
        workout_type="easy",
        feedback="You can also create standalone activities (i.e. without needing an activity file) to track sports that you are not recording your sessions.",
    )

    # Today's activity:
    upload_activity_from_content(
        file_content=easy_run(datetime.combine(today, dt_time(hour=8))),
        name="An imported sport activity",
        rpe=random.randint(2, 4),
        workout_type="easy",
        feedback='On top of tracking the RPE, workout type and nutrition for each activtiy, you can add a description to an activity to track your feedback  and what\'s happened more precisely (e.g. "slight pain in my left calf during the first two kilometers", "skipped last rep because too much ice").',
    )

    # Create training periods
    print("\nCreating training periods...")

    period1_start = (today - timedelta(days=84)).strftime("%Y-%m-%d")
    period1_end = today_str
    period_1_id = create_training_period(
        "This is a training period",
        period1_start,
        period1_end,
        "Training periods help you keep track of specific training blocks, e.g. base training, race specific preparation, etc.\nYou can define dedicated training metrics for this period, or import some from your globally defined ones.",
    )

    # Create training metrics
    print("\nCreating training metrics...")

    create_scoped_training_metric(
        "Average heart rate (only for this training period)",
        "AvgHeartRate",
        aggregate=None,
        granularity=None,
        period_id=period_1_id,
        average=True,
    )

    create_global_training_metric(
        "Weekly calories", "Calories", "Weekly", "Sum", average=True
    )

    create_global_training_metric(
        "Weekly distance",
        "Distance",
        "Weekly",
        "Sum",
        group_by="SportCategory",
    )

    create_global_training_metric(
        "Weekly duration",
        "ActiveDuration",
        "Weekly",
        "Sum",
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
    delete_all_global_metrics()

    generate_demo_data()

    print(f"Demo reset completed at {time.strftime('%Y-%m-%d %H:%M:%S')}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
