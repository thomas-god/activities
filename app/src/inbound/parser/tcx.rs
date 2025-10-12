use std::collections::HashMap;

use chrono::{DateTime, FixedOffset, Utc};
use itertools::Itertools;
use roxmltree::Document;

use crate::{
    domain::models::activity::{
        ActiveTime, ActivityStartTime, ActivityStatistic, ActivityStatistics, ActivityTimeseries,
        Sport, Timeseries, TimeseriesActiveTime, TimeseriesMetric, TimeseriesTime, TimeseriesValue,
    },
    inbound::parser::{ParseBytesError, ParsedFileContent, SupportedExtension},
};

pub fn try_tcx_bytes_into_domain(
    bytes: Vec<u8>,
) -> Result<super::ParsedFileContent, super::ParseBytesError> {
    let content =
        String::from_utf8(bytes.clone()).map_err(|_err| ParseBytesError::InvalidContent)?;

    let doc = roxmltree::Document::parse(content.trim())
        .map_err(|_err| ParseBytesError::InvalidContent)?;

    let start_time = find_activity_start_time(&doc).ok_or(ParseBytesError::NoStartTimeFound)?;
    let sport = find_sport(&doc);
    let statistics = find_activity_statistics(&doc);
    let timeseries = parse_timeseries(&doc, start_time.date());

    Ok(ParsedFileContent::new(
        sport,
        start_time,
        statistics,
        timeseries,
        SupportedExtension::TCX.suffix().to_string(),
        bytes,
    ))
}

fn find_sport(doc: &Document) -> Sport {
    let Some(sport_node) = doc.descendants().find(|node| node.has_attribute("Sport")) else {
        return Sport::Other;
    };

    match sport_node.attribute("Sport") {
        Some("Running") => Sport::Running,
        Some("Biking") => Sport::Cycling,
        Some(_) => Sport::Other,
        None => Sport::Other,
    }
}

fn find_activity_start_time(doc: &Document) -> Option<ActivityStartTime> {
    let start_time_nodes = doc.descendants().filter_map(|node| {
        node.attribute("StartTime").and_then(|content| {
            content
                .parse::<DateTime<Utc>>()
                .map(|dt| dt.fixed_offset())
                .ok()
        })
    });

    let start_time = start_time_nodes.min()?;

    Some(ActivityStartTime::new(start_time))
}

fn accumulate_lap_tag_values(doc: &Document, tag: &str) -> Option<f64> {
    doc.descendants()
        .filter(|node| node.has_tag_name("Lap"))
        .filter_map(|lap| {
            lap.children().find_map(|node| {
                if !node.has_tag_name(tag) {
                    return None;
                }
                node.text()?.parse::<f64>().ok()
            })
        })
        .reduce(|acc, current| acc + current)
}

fn find_activity_statistics(doc: &Document) -> ActivityStatistics {
    let mut stats = HashMap::new();

    let tags = [
        ("TotalTimeSeconds", ActivityStatistic::Duration),
        ("DistanceMeters", ActivityStatistic::Distance),
        ("Calories", ActivityStatistic::Calories),
    ];

    for (tag, metric) in tags {
        if let Some(value) = accumulate_lap_tag_values(doc, tag) {
            stats.insert(metric, value);
        }
    }

    let elevation_gain = doc
        .descendants()
        .filter_map(|node| {
            if node.has_tag_name("AltitudeMeters") {
                node.text().and_then(|txt| txt.parse::<f64>().ok())
            } else {
                None
            }
        })
        .tuple_windows::<(f64, f64)>()
        .fold(0., |elev, (a, b)| elev + f64::min(b - a, 0.));
    if elevation_gain != 0. {
        stats.insert(ActivityStatistic::Elevation, elevation_gain);
    }

    ActivityStatistics::new(stats)
}

fn parse_timeseries(doc: &Document, reference_time: &DateTime<FixedOffset>) -> ActivityTimeseries {
    let mut time_values = Vec::new();
    let mut speed_values = vec![];
    let mut power_values = vec![];
    let mut cadence_values = vec![];
    let mut distance_values = vec![];
    let mut altitude_values = vec![];
    let mut heart_rate_values = vec![];

    for node in doc.descendants() {
        if !node.has_tag_name("Trackpoint") {
            continue;
        }

        let Some(time) = node
            .descendants()
            .find(|elem| elem.has_tag_name("Time"))
            .and_then(|elem| {
                elem.text()
                    .and_then(|txt| txt.parse::<DateTime<FixedOffset>>().ok())
                    .map(|time| (time - reference_time).num_seconds() as usize)
            })
        else {
            continue;
        };
        time_values.push(time);

        let speed = node
            .descendants()
            .find(|elem| elem.has_tag_name("Speed"))
            .and_then(|elem| elem.text().and_then(|txt| txt.parse::<f64>().ok()))
            .map(TimeseriesValue::Float);
        speed_values.push(speed);

        let distance = node
            .descendants()
            .find(|elem| elem.has_tag_name("DistanceMeters"))
            .and_then(|elem| elem.text().and_then(|txt| txt.parse::<f64>().ok()))
            .map(TimeseriesValue::Float);
        distance_values.push(distance);

        let heart_rate = node
            .descendants()
            .find_map(|elem| {
                if !elem.has_tag_name("HeartRateBpm") {
                    return None;
                }
                elem.children().find(|child| child.has_tag_name("Value"))
            })
            .and_then(|elem| elem.text().and_then(|txt| txt.parse::<f64>().ok()))
            .map(TimeseriesValue::Float);
        heart_rate_values.push(heart_rate);

        let power = node
            .descendants()
            .find(|elem| elem.has_tag_name("Watts"))
            .and_then(|elem| elem.text().and_then(|txt| txt.parse::<f64>().ok()))
            .map(TimeseriesValue::Float);
        power_values.push(power);

        let cadence = node
            .descendants()
            .find(|elem| elem.has_tag_name("Cadence"))
            .and_then(|elem| elem.text().and_then(|txt| txt.parse::<f64>().ok()))
            .map(TimeseriesValue::Float);
        cadence_values.push(cadence);

        let altitude = node
            .descendants()
            .find(|elem| elem.has_tag_name("AltitudeMeters"))
            .and_then(|elem| elem.text().and_then(|txt| txt.parse::<f64>().ok()))
            .map(TimeseriesValue::Float);
        altitude_values.push(altitude);
    }

    let metrics = vec![
        Timeseries::new(TimeseriesMetric::Speed, speed_values),
        Timeseries::new(TimeseriesMetric::Distance, distance_values),
        Timeseries::new(TimeseriesMetric::HeartRate, heart_rate_values),
        Timeseries::new(TimeseriesMetric::Power, power_values),
        Timeseries::new(TimeseriesMetric::Cadence, cadence_values),
        Timeseries::new(TimeseriesMetric::Altitude, altitude_values),
    ];

    // TCX does not support pause, so active time = time
    let active_time = TimeseriesActiveTime::new(
        time_values
            .iter()
            .cloned()
            .map(ActiveTime::Running)
            .collect(),
    );

    ActivityTimeseries::new(TimeseriesTime::new(time_values), active_time, metrics)
}

#[cfg(test)]
mod test_txc_parser {

    use std::fs;

    use chrono::{DateTime, FixedOffset};

    use crate::{
        domain::models::activity::{ActivityStartTime, ActivityStatistic},
        inbound::parser::ParseBytesError,
    };

    use super::*;

    #[test]
    fn test_parse_file_ok() {
        let file = fs::read("src/inbound/parser/test.tcx").expect("Unable to load tcx test file");

        try_tcx_bytes_into_domain(file).expect("Should have returned Ok");
    }

    #[test]
    fn test_parse_file_content_with_trailing_whitespaces() {
        let file = "    <root StartTime=\"2017-01-05T09:29:35.000Z\"/>   "
            .to_string()
            .into_bytes();
        try_tcx_bytes_into_domain(file).expect("Should have returned Ok");
    }

    #[test]
    fn test_parse_file_content_is_not_a_valid_string() {
        let file = vec![0xFF, 0xFE, 0xFD];

        assert_eq!(
            try_tcx_bytes_into_domain(file).unwrap_err(),
            ParseBytesError::InvalidContent
        );
    }

    #[test]
    fn test_parse_file_content_is_not_a_valid_xml() {
        let file = "blabla".to_string().into_bytes();

        assert_eq!(
            try_tcx_bytes_into_domain(file).unwrap_err(),
            ParseBytesError::InvalidContent
        );
    }

    #[test]
    fn test_parse_file_content_does_not_have_a_start_time() {
        let file = "<root/>".to_string().into_bytes();

        assert_eq!(
            try_tcx_bytes_into_domain(file).unwrap_err(),
            ParseBytesError::NoStartTimeFound
        );
    }

    #[test]
    fn test_find_sport() {
        assert_eq!(
            find_sport(&roxmltree::Document::parse("<Activity Sport=\"Biking\" />").unwrap()),
            Sport::Cycling
        );

        assert_eq!(
            find_sport(&roxmltree::Document::parse("<Activity Sport=\"Running\" />").unwrap()),
            Sport::Running
        );

        assert_eq!(
            find_sport(&roxmltree::Document::parse("<Activity Sport=\"Other\" />").unwrap()),
            Sport::Other
        );

        assert_eq!(
            find_sport(&roxmltree::Document::parse("<NotActivityTag Sport=\"Biking\" />").unwrap()),
            Sport::Cycling
        );

        assert_eq!(
            find_sport(
                &roxmltree::Document::parse("<NotActivityTag NotTheGoodAttribute=\"Biking\" />")
                    .unwrap()
            ),
            Sport::Other
        );
    }

    #[test]
    fn test_find_activity_start_time() {
        assert_eq!(
            find_activity_start_time(
                &roxmltree::Document::parse("<Lap StartTime=\"2024-08-28T07:12:54.000Z\" />")
                    .unwrap()
            ),
            Some(ActivityStartTime::new(
                "2024-08-28T07:12:54+00:00"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap()
            ))
        );

        assert_eq!(
            find_activity_start_time(
                &roxmltree::Document::parse(
                    "<AnotherTag StartTime=\"2024-08-28T07:12:54.000Z\" />"
                )
                .unwrap()
            ),
            Some(ActivityStartTime::new(
                "2024-08-28T07:12:54+00:00"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap()
            ))
        );

        assert_eq!(
            find_activity_start_time(
                &roxmltree::Document::parse(
                    "<AnotherTag WrongAttributeStartTime=\"2024-08-28T07:12:54.000Z\" />"
                )
                .unwrap()
            ),
            None
        );

        assert_eq!(
            find_activity_start_time(
                &roxmltree::Document::parse("<Lap StartTime=\"not-a-valid-date-time\" />").unwrap()
            ),
            None
        );
    }

    #[test]
    fn test_find_activity_start_time_multi_laps() {
        assert_eq!(
            find_activity_start_time(
                &roxmltree::Document::parse(
                    "
                    <root>
                    <Lap StartTime=\"2024-08-28T07:12:54.000Z\" />
                    <Lap StartTime=\"2024-08-28T06:12:54.000Z\" />
                    </root>"
                )
                .unwrap()
            ),
            Some(ActivityStartTime::new(
                "2024-08-28T06:12:54+00:00"
                    .parse::<DateTime<FixedOffset>>()
                    .unwrap()
            ))
        );
    }

    #[test]
    fn test_find_activity_statistics() {
        let content = String::from_utf8(
            fs::read("src/inbound/parser/test.tcx").expect("Unable to load tcx test file"),
        )
        .unwrap();
        let doc = roxmltree::Document::parse(&content).unwrap();

        let statistics = find_activity_statistics(&doc);

        assert_eq!(
            *statistics
                .get(&ActivityStatistic::Duration)
                .expect("Stats should have a duration"),
            22574.324
        );

        assert_eq!(
            *statistics
                .get(&ActivityStatistic::Distance)
                .expect("Stats should have a distance"),
            105420.05
        );

        assert_eq!(
            *statistics
                .get(&ActivityStatistic::Calories)
                .expect("Stats should have a calories"),
            3359.
        );

        assert_eq!(
            *statistics
                .get(&ActivityStatistic::Elevation)
                .expect("Stats should have an elevation"),
            f64::min(1386.0 - 1399.19, 0.) + f64::min(1399.19 - 1399.40, 0.)
        );
    }

    #[test]
    fn test_find_activity_statistics_multi_laps() {
        let content = String::from(
            "<root>
                <Lap StartTime=\"2024-08-28T07:12:54.000Z\">
                    <TotalTimeSeconds>10</TotalTimeSeconds>
                    <DistanceMeters>120</DistanceMeters>
                    <Calories>33</Calories>
                </Lap>
                <Lap StartTime=\"2024-08-28T07:13:54.000Z\">
                    <TotalTimeSeconds>12</TotalTimeSeconds>
                    <DistanceMeters>10</DistanceMeters>
                    <Calories>30</Calories>
                </Lap>
            </root>",
        );
        let doc = roxmltree::Document::parse(&content).unwrap();

        let statistics = find_activity_statistics(&doc);

        assert_eq!(
            *statistics
                .get(&ActivityStatistic::Duration)
                .expect("Stats should have a duration"),
            10. + 12.
        );

        assert_eq!(
            *statistics
                .get(&ActivityStatistic::Distance)
                .expect("Stats should have a distance"),
            120. + 10.
        );

        assert_eq!(
            *statistics
                .get(&ActivityStatistic::Calories)
                .expect("Stats should have a calories"),
            33. + 30.
        );
    }

    #[test]
    fn test_parse_timeseries() {
        let content = String::from_utf8(
            fs::read("src/inbound/parser/test.tcx").expect("Unable to load tcx test file"),
        )
        .unwrap();
        let doc = roxmltree::Document::parse(&content).unwrap();
        let start_time = find_activity_start_time(&doc).expect("Should have a start time");

        let timeseries = parse_timeseries(&doc, start_time.date());

        assert_eq!(timeseries.time().len(), 3);

        assert_eq!(timeseries.time().values(), &vec![0, 1, 29707]);
        assert_eq!(
            timeseries
                .metrics()
                .iter()
                .find(|metric| metric.metric() == &TimeseriesMetric::Speed)
                .expect("Should have a speed timeseries")
                .values(),
            &vec![
                Some(TimeseriesValue::Float(1.83)),
                Some(TimeseriesValue::Float(2.10)),
                Some(TimeseriesValue::Float(0.0))
            ]
        );
        assert_eq!(
            timeseries
                .metrics()
                .iter()
                .find(|metric| metric.metric() == &TimeseriesMetric::Distance)
                .expect("Should have a distance timeseries")
                .values(),
            &vec![
                Some(TimeseriesValue::Float(0.0)),
                Some(TimeseriesValue::Float(2.50)),
                Some(TimeseriesValue::Float(105420.04))
            ]
        );
        assert_eq!(
            timeseries
                .metrics()
                .iter()
                .find(|metric| metric.metric() == &TimeseriesMetric::HeartRate)
                .expect("Should have a heartrate timeseries")
                .values(),
            &vec![
                Some(TimeseriesValue::Float(98.)),
                Some(TimeseriesValue::Float(99.)),
                Some(TimeseriesValue::Float(113.))
            ]
        );
        assert_eq!(
            timeseries
                .metrics()
                .iter()
                .find(|metric| metric.metric() == &TimeseriesMetric::Power)
                .expect("Should have a Power timeseries")
                .values(),
            &vec![
                Some(TimeseriesValue::Float(0.0)),
                Some(TimeseriesValue::Float(24.)),
                Some(TimeseriesValue::Float(0.0))
            ]
        );
        assert_eq!(
            timeseries
                .metrics()
                .iter()
                .find(|metric| metric.metric() == &TimeseriesMetric::Altitude)
                .expect("Should have a Power timeseries")
                .values(),
            &vec![
                Some(TimeseriesValue::Float(1399.4)),
                Some(TimeseriesValue::Float(1399.19)),
                Some(TimeseriesValue::Float(1386.0))
            ]
        );
        assert_eq!(
            timeseries
                .metrics()
                .iter()
                .find(|metric| metric.metric() == &TimeseriesMetric::Cadence)
                .expect("Should have a Cadence timeseries")
                .values(),
            &vec![
                Some(TimeseriesValue::Float(100.)),
                Some(TimeseriesValue::Float(100.)),
                Some(TimeseriesValue::Float(100.))
            ]
        );
    }
}
