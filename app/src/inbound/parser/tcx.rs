use std::collections::HashMap;

use chrono::{DateTime, Utc};
use itertools::Itertools;
use roxmltree::Document;

use crate::{
    domain::models::activity::{ActivityStartTime, ActivityStatistic, ActivityStatistics, Sport},
    inbound::parser::ParseFile,
};

#[derive(Clone)]
pub struct TcxParser {}

impl ParseFile for TcxParser {
    fn try_bytes_into_domain(
        &self,
        bytes: Vec<u8>,
    ) -> Result<super::ParsedFileContent, super::ParseBytesError> {
        let content = String::from_utf8(bytes).expect("cannot build string");

        let doc = roxmltree::Document::parse(&content).expect("cannot parse string");

        todo!()
    }
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

fn find_activity_statistics(doc: &Document) -> ActivityStatistics {
    let mut stats = HashMap::new();

    if let Some(duration) = doc
        .descendants()
        .find(|node| node.has_tag_name("TotalTimeSeconds"))
        .and_then(|node| node.text()?.parse::<f64>().ok())
    {
        stats.insert(ActivityStatistic::Duration, duration);
    };

    if let Some(distance) = doc
        .descendants()
        .find(|node| node.has_tag_name("DistanceMeters"))
        .and_then(|node| node.text()?.parse::<f64>().ok())
    {
        stats.insert(ActivityStatistic::Distance, distance);
    };

    if let Some(calories) = doc
        .descendants()
        .find(|node| node.has_tag_name("Calories"))
        .and_then(|node| node.text()?.parse::<f64>().ok())
    {
        stats.insert(ActivityStatistic::Calories, calories);
    };

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

#[cfg(test)]
mod test_txc_parser {

    use std::fs;

    use chrono::{DateTime, FixedOffset};

    use crate::domain::models::activity::{ActivityStartTime, ActivityStatistic};

    use super::*;

    // #[test]
    // fn test() {
    //     let file = fs::read(".test.tcx").expect("Unable to load tcx test file");
    //     let parser = TcxParser {};

    //     let parsed_file = parser
    //         .try_bytes_into_domain(file)
    //         .expect("Should have returned Ok");

    //     assert_eq!(
    //         parsed_file.start_time().date(),
    //         &"2024-08-28T07:12:54.000+00:00"
    //             .parse::<DateTime<FixedOffset>>()
    //             .unwrap()
    //     );
    // }

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
            f64::min(1386.0 - 1399.199951171875, 0.)
                + f64::min(1399.199951171875 - 1399.4000244140625, 0.)
        );
    }
}
