#!/bin/bash
# Generate demo activities and training data using the API

API_URL="${API_URL:-http://activities:8080/api}"
TEMP_DIR="./tmp/demo-tcx"
mkdir -p "$TEMP_DIR"

# Calculate dates relative to today
TODAY=$(date +"%Y-%m-%d")

echo "Generating demo data with dates relative to today: $TODAY"

# Generate TCX files with various activities
generate_tcx() {
  local date="$1"
  local sport="$2"
  local duration_seconds="$3"
  local distance_meters="$4"
  local avg_hr="$5"
  local filename="$6"

  # Convert date to ISO format for TCX
  local start_time="${date}T$(printf "%02d" $((RANDOM % 12 + 6))):$(printf "%02d" $((RANDOM % 60))):00Z"
  
  # Calculate some intermediate points for more realistic data
  local num_points=10
  local time_step=$((duration_seconds / num_points))
  local dist_step=$((distance_meters / num_points))
  
  cat > "$TEMP_DIR/$filename" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<TrainingCenterDatabase xmlns="http://www.garmin.com/xmlschemas/TrainingCenterDatabase/v2">
  <Activities>
    <Activity Sport="$sport">
      <Id>$start_time</Id>
      <Lap StartTime="$start_time">
        <TotalTimeSeconds>$duration_seconds</TotalTimeSeconds>
        <DistanceMeters>$distance_meters</DistanceMeters>
        <Calories>$((duration_seconds * 10 / 60))</Calories>
        <AverageHeartRateBpm><Value>$avg_hr</Value></AverageHeartRateBpm>
        <MaximumHeartRateBpm><Value>$((avg_hr + 20))</Value></MaximumHeartRateBpm>
        <Intensity>Active</Intensity>
        <TriggerMethod>Manual</TriggerMethod>
        <Track>
EOF

  # Generate trackpoints with varying data
  local i=0
  while [ $i -le $num_points ]; do
    local point_time=$((i * time_step))
    local point_dist=$((i * dist_step))
    # Generate random HR variation between -5 and +5
    local hr_variation=$(awk -v min=-5 -v max=5 'BEGIN{srand(); print int(min+rand()*(max-min+1))}')
    local point_hr=$((avg_hr + hr_variation))
    
    # Use date command to add seconds to start time
    local point_timestamp=$(date -u -d "@$(($(date -u -d "$start_time" +%s) + point_time))" +"%Y-%m-%dT%H:%M:%SZ")
    
    cat >> "$TEMP_DIR/$filename" <<TRACKPOINT
          <Trackpoint>
            <Time>$point_timestamp</Time>
            <DistanceMeters>$point_dist</DistanceMeters>
            <HeartRateBpm><Value>$point_hr</Value></HeartRateBpm>
          </Trackpoint>
TRACKPOINT
    i=$((i + 1))
  done

  cat >> "$TEMP_DIR/$filename" <<EOF
        </Track>
      </Lap>
    </Activity>
  </Activities>
</TrainingCenterDatabase>
EOF
}

# Generate 12 weeks of activities (4 per week)
echo "Generating activities..."
week=0
while [ $week -lt 12 ]; do
  # Activity 1: Easy run
  days_ago=$((week * 7 + 1))
  date=$(date -d "$TODAY - $days_ago days" +"%Y-%m-%d")
  generate_tcx "$date" "Running" 1800 5000 135 "run_easy_week${week}_1.tcx"
  
  # Activity 2: Tempo run
  days_ago=$((week * 7 + 3))
  date=$(date -d "$TODAY - $days_ago days" +"%Y-%m-%d")
  generate_tcx "$date" "Running" 2400 8000 155 "run_tempo_week${week}_2.tcx"
  
  # Activity 3: Bike ride
  days_ago=$((week * 7 + 5))
  date=$(date -d "$TODAY - $days_ago days" +"%Y-%m-%d")
  generate_tcx "$date" "Biking" 5400 40000 140 "bike_week${week}_3.tcx"
  
  # Activity 4: Long run (weekend)
  days_ago=$((week * 7 + 6))
  date=$(date -d "$TODAY - $days_ago days" +"%Y-%m-%d")
  generate_tcx "$date" "Running" 4500 15000 145 "run_long_week${week}_4.tcx"
  
  week=$((week + 1))
done

echo "Uploading activities to API..."
for tcx_file in "$TEMP_DIR"/*.tcx; do
  filename=$(basename "$tcx_file")
  response=$(mktemp)
  status=$(curl -s -w "%{http_code}" -o "$response" -X POST \
    -F "file.tcx=@$tcx_file" \
    "$API_URL/activity")
  body=$(cat "$response")
  rm "$response"
  echo "Uploading $filename... $status - $body"
done

# Create training periods (4 periods over the 12 weeks)
echo "Creating training periods..."

# Period 1: Base Building (weeks 0-3, days 84-63)
period1_start=$(date -d "$TODAY - 84 days" +%Y-%m-%d)
period1_end=$(date -d "$TODAY - 63 days" +%Y-%m-%d)
response=$(mktemp)
status=$(curl -s -w "%{http_code}" -o "$response" -X POST "$API_URL/training/period" \
  -H "Content-Type: application/json" \
  -d "{\"name\":\"Base Building\",\"start\":\"$period1_start\",\"end\":\"$period1_end\",\"description\":\"Building aerobic base with easy miles\",\"sports\":[]}")
echo "Period 1 (Base Building): $status"
cat "$response"
rm "$response"
echo ""

# Period 2: Build Phase (weeks 4-7, days 56-35)
period2_start=$(date -d "$TODAY - 56 days" +%Y-%m-%d)
period2_end=$(date -d "$TODAY - 35 days" +%Y-%m-%d)
response=$(mktemp)
status=$(curl -s -w "%{http_code}" -o "$response" -X POST "$API_URL/training/period" \
  -H "Content-Type: application/json" \
  -d "{\"name\":\"Build Phase\",\"start\":\"$period2_start\",\"end\":\"$period2_end\",\"description\":\"Introducing tempo runs and longer rides\",\"sports\":[]}")
echo "Period 2 (Build Phase): $status"
cat "$response"
rm "$response"
echo ""

# Period 3: Peak Training (weeks 8-10, days 28-7)
period3_start=$(date -d "$TODAY - 28 days" +%Y-%m-%d)
period3_end=$(date -d "$TODAY - 7 days" +%Y-%m-%d)
response=$(mktemp)
status=$(curl -s -w "%{http_code}" -o "$response" -X POST "$API_URL/training/period" \
  -H "Content-Type: application/json" \
  -d "{\"name\":\"Peak Training\",\"start\":\"$period3_start\",\"end\":\"$period3_end\",\"description\":\"Maximum training load with quality workouts\",\"sports\":[]}")
echo "Period 3 (Peak Training): $status"
cat "$response"
rm "$response"
echo ""

# Period 4: Race Week (week 11, days 6-0)
period4_start=$(date -d "$TODAY - 6 days" +%Y-%m-%d)
period4_end=$TODAY
response=$(mktemp)
status=$(curl -s -w "%{http_code}" -o "$response" -X POST "$API_URL/training/period" \
  -H "Content-Type: application/json" \
  -d "{\"name\":\"Race Week\",\"start\":\"$period4_start\",\"end\":\"$period4_end\",\"description\":\"Taper week with reduced volume\",\"sports\":[]}")
echo "Period 4 (Race Week): $status"
cat "$response"
rm "$response"
echo ""

# Create some training metrics
echo "Creating training metrics..."

# Metric 1: Weekly total calories for all activities
response=$(mktemp)
status=$(curl -s -w "%{http_code}" -o "$response" -X POST "$API_URL/training/metric" \
  -H "Content-Type: application/json" \
  -d "{\"source\":{\"Statistic\":\"Calories\"},\"granularity\":\"Weekly\",\"aggregate\":\"Sum\",\"filters\":{},\"initial_date_range\":{\"start\":\"$metric_start\",\"end\":\"$TODAY\"}}")
body=$(cat "$response")
rm "$response"
echo "Creating metric (Weekly Total Calories)... $status - $body"

# Metric 2: Weekly duration by sport category (grouped)
response=$(mktemp)
status=$(curl -s -w "%{http_code}" -o "$response" -X POST "$API_URL/training/metric" \
  -H "Content-Type: application/json" \
  -d "{\"source\":{\"Statistic\":\"Duration\"},\"granularity\":\"Weekly\",\"aggregate\":\"Sum\",\"filters\":{},\"group_by\":\"SportCategory\",\"initial_date_range\":{\"start\":\"$metric_start\",\"end\":\"$TODAY\"}}")
body=$(cat "$response")
rm "$response"
echo "Creating metric (Weekly Duration by Sport)... $status - $body"

# Create some training notes
echo "Creating training notes..."

# Note from last week (3 days ago)
note_date=$(date -d "$TODAY - 3 days" +%Y-%m-%d)
response=$(mktemp)
status=$(curl -s -w "%{http_code}" -o "$response" -X POST "$API_URL/training/note" \
  -H "Content-Type: application/json" \
  -d "{\"date\":\"$note_date\",\"content\":\"Felt great during today's tempo run. HR was well controlled and pace felt comfortable. Ready for race week!\"}")
body=$(cat "$response")
rm "$response"
echo "Creating note for $note_date... $status - $body"

# Note from 2 weeks ago (10 days ago)
note_date=$(date -d "$TODAY - 10 days" +%Y-%m-%d)
response=$(mktemp)
status=$(curl -s -w "%{http_code}" -o "$response" -X POST "$API_URL/training/note" \
  -H "Content-Type: application/json" \
  -d "{\"date\":\"$note_date\",\"content\":\"Completed longest run of the training block. Legs felt tired but recovered well with proper nutrition and stretching.\"}")
body=$(cat "$response")
rm "$response"
echo "Creating note for $note_date... $status - $body"

# Cleanup
# rm -rf "$TEMP_DIR"

echo "Demo data generation complete!"
echo "Generated:"
echo "  - 48 activities (12 weeks × 4 activities/week)"
echo "  - 4 training periods"
echo "  - 3 training metrics"
echo "  - 2 training notes"
