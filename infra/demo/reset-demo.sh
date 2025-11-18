#!/bin/sh
# Reset demo environment with fresh data using the API

echo "Starting demo reset at $(date)"

#!/bin/bash
set -e

API_URL="http://activities/api"
# API_URL="http://localhost:8080/api"

# Wait for the service to be available
echo "Waiting for API to be available..."
until curl -sf "$API_URL/activities" > /dev/null 2>&1; do
  sleep 2
done

echo "API is available, proceeding with reset..."

# Get all activities and delete them
echo "Deleting all existing activities..."
ACTIVITIES=$(curl -sf "$API_URL/activities" | jq -r '.[].id')
for id in $ACTIVITIES; do
  curl -sf -X DELETE "$API_URL/activity/$id"
  echo "Deleted activity: $id"
done

# Get all training periods and delete them
echo "Deleting all existing training periods..."
PERIODS=$(curl -sf "$API_URL/training/periods" | jq -r '.[].id')
for id in $PERIODS; do
  curl -sf -X DELETE "$API_URL/training/period/$id"
  echo "Deleted period: $id"
done

# Get all training notes and delete them
echo "Deleting all existing training notes..."
NOTES=$(curl -sf "$API_URL/training/notes" | jq -r '.[].id')
for id in $NOTES; do
  curl -sf -X DELETE "$API_URL/training/note/$id"
  echo "Deleted note: $id"
done

# Get all training metrics and delete them
echo "Deleting all existing training metrics..."
METRICS=$(curl -s "$API_URL/training/metrics?start=2020-01-01T00:00:00%2B00:00" 2>&1 | jq -r '.[].id' 2>&1)
for id in $METRICS; do
  curl -s -X DELETE "$API_URL/training/metric/$id" 2>&1 >/dev/null
  echo "Deleted metric: $id"
done


# Generate fresh demo data
echo "Generating fresh demo data..."
/usr/local/bin/generate-demo-data.sh
# ./generate-demo-data.sh

echo "Demo reset completed at $(date)"
