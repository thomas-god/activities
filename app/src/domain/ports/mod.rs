use std::ops::Add;

use chrono::{DateTime, Days, FixedOffset, NaiveDate};
use derive_more::Constructor;
use serde::Deserialize;

use crate::domain::models::training::TrainingMetricGranularity;

pub mod activity;
pub mod preferences;
pub mod training;

#[derive(Debug, Clone, PartialEq, Constructor)]
pub struct DateTimeRange {
    start: DateTime<FixedOffset>,
    end: Option<DateTime<FixedOffset>>,
}

impl DateTimeRange {
    pub fn start(&self) -> &DateTime<FixedOffset> {
        &self.start
    }

    pub fn end(&self) -> &Option<DateTime<FixedOffset>> {
        &self.end
    }
}

#[derive(Debug, Clone, PartialEq, Constructor, Deserialize)]
pub struct DateRange {
    start: NaiveDate,
    end: NaiveDate,
}

impl DateRange {
    pub fn start(&self) -> &NaiveDate {
        &self.start
    }

    pub fn end(&self) -> &NaiveDate {
        &self.end
    }

    /// Align the date range to a given granularity to ensure complete bins.
    /// For example, if granularity is Weekly and date_range starts on Wednesday,
    /// we align it to the Monday of that week to include the full week's data.
    pub fn align_to(&self, granularity: &TrainingMetricGranularity) -> DateRange {
        let bins = granularity.bins(self);
        if let (Some(first), Some(last)) = (bins.first(), bins.last()) {
            DateRange::new(*first.start(), *last.end())
        } else {
            self.clone()
        }
    }

    pub fn extend_end(&self, days_to_add: Days) -> DateRange {
        DateRange::new(self.start, self.end.add(days_to_add))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::training::TrainingMetricGranularity;

    #[test]
    fn test_align_to_daily_granularity() {
        // Given a date range that doesn't start/end on day boundaries
        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2024, 3, 15).unwrap(),
            NaiveDate::from_ymd_opt(2024, 3, 20).unwrap(),
        );
        let granularity = TrainingMetricGranularity::Daily;

        // When aligning to daily granularity
        let aligned = range.align_to(&granularity);

        // Then it should align to the first and last day boundaries
        assert_eq!(
            aligned,
            DateRange::new(
                NaiveDate::from_ymd_opt(2024, 3, 15).unwrap(),
                NaiveDate::from_ymd_opt(2024, 3, 21).unwrap(),
            )
        );
    }

    #[test]
    fn test_align_to_weekly_granularity() {
        // Given a date range in the middle of weeks
        // 2024-03-15 is a Friday, 2024-03-20 is a Wednesday
        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2024, 3, 15).unwrap(),
            NaiveDate::from_ymd_opt(2024, 3, 20).unwrap(),
        );
        let granularity = TrainingMetricGranularity::Weekly;

        // When aligning to weekly granularity
        let aligned = range.align_to(&granularity);

        // Then it should align to Monday of the first week and Monday after the last week
        // First Monday is 2024-03-11, second Monday is 2024-03-18
        assert_eq!(
            aligned,
            DateRange::new(
                NaiveDate::from_ymd_opt(2024, 3, 11).unwrap(),
                NaiveDate::from_ymd_opt(2024, 3, 25).unwrap(),
            )
        );
    }

    #[test]
    fn test_align_to_weekly_granularity_spanning_multiple_weeks() {
        // Given a date range spanning multiple weeks
        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2024, 1, 10).unwrap(),
            NaiveDate::from_ymd_opt(2024, 2, 15).unwrap(),
        );
        let granularity = TrainingMetricGranularity::Weekly;

        // When aligning to weekly granularity
        let aligned = range.align_to(&granularity);

        // Then it should align to the Monday boundaries
        // 2024-01-08 is the Monday before 2024-01-10
        // 2024-02-19 is the Monday after 2024-02-15
        assert_eq!(
            aligned,
            DateRange::new(
                NaiveDate::from_ymd_opt(2024, 1, 8).unwrap(),
                NaiveDate::from_ymd_opt(2024, 2, 19).unwrap(),
            )
        );
    }

    #[test]
    fn test_align_to_monthly_granularity() {
        // Given a date range in the middle of months
        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2024, 3, 15).unwrap(),
            NaiveDate::from_ymd_opt(2024, 5, 20).unwrap(),
        );
        let granularity = TrainingMetricGranularity::Monthly;

        // When aligning to monthly granularity
        let aligned = range.align_to(&granularity);

        // Then it should align to the first day of the months
        assert_eq!(
            aligned,
            DateRange::new(
                NaiveDate::from_ymd_opt(2024, 3, 1).unwrap(),
                NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
            )
        );
    }

    #[test]
    fn test_align_to_monthly_granularity_same_month() {
        // Given a date range within a single month
        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2024, 3, 15).unwrap(),
            NaiveDate::from_ymd_opt(2024, 3, 25).unwrap(),
        );
        let granularity = TrainingMetricGranularity::Monthly;

        // When aligning to monthly granularity
        let aligned = range.align_to(&granularity);

        // Then it should align to the month boundaries
        assert_eq!(
            aligned,
            DateRange::new(
                NaiveDate::from_ymd_opt(2024, 3, 1).unwrap(),
                NaiveDate::from_ymd_opt(2024, 4, 1).unwrap(),
            )
        );
    }

    #[test]
    fn test_align_to_already_aligned_daily() {
        // Given a date range already aligned to daily boundaries
        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2024, 3, 15).unwrap(),
            NaiveDate::from_ymd_opt(2024, 3, 16).unwrap(),
        );
        let granularity = TrainingMetricGranularity::Daily;

        // When aligning to daily granularity
        let aligned = range.align_to(&granularity);

        // Then the aligned range includes bins for both days
        // Bins are: [2024-03-15, 2024-03-16), [2024-03-16, 2024-03-17)
        // So aligned range is [2024-03-15, 2024-03-17)
        assert_eq!(
            aligned,
            DateRange::new(
                NaiveDate::from_ymd_opt(2024, 3, 15).unwrap(),
                NaiveDate::from_ymd_opt(2024, 3, 17).unwrap(),
            )
        );
    }

    #[test]
    fn test_align_to_same_date() {
        // Given a date range where start and end are the same
        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2024, 3, 15).unwrap(),
            NaiveDate::from_ymd_opt(2024, 3, 15).unwrap(),
        );
        let granularity = TrainingMetricGranularity::Daily;

        // When aligning to daily granularity
        let aligned = range.align_to(&granularity);

        // Then it should produce a one-day range
        assert_eq!(
            aligned,
            DateRange::new(
                NaiveDate::from_ymd_opt(2024, 3, 15).unwrap(),
                NaiveDate::from_ymd_opt(2024, 3, 16).unwrap(),
            )
        );
    }

    #[test]
    fn test_align_to_year_boundary_monthly() {
        // Given a date range spanning year boundaries
        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2023, 11, 15).unwrap(),
            NaiveDate::from_ymd_opt(2024, 2, 20).unwrap(),
        );
        let granularity = TrainingMetricGranularity::Monthly;

        // When aligning to monthly granularity
        let aligned = range.align_to(&granularity);

        // Then it should align to the month boundaries across years
        assert_eq!(
            aligned,
            DateRange::new(
                NaiveDate::from_ymd_opt(2023, 11, 1).unwrap(),
                NaiveDate::from_ymd_opt(2024, 3, 1).unwrap(),
            )
        );
    }
}
