use crate::cli::FilteringOptions;
use crate::direction::Direction;
use crate::log_entry::LogEntry;
use streaming_iterator::StreamingIterator;
use subslice::SubsliceExt;

pub fn filtering_iter(
    input: impl StreamingIterator<Item = LogEntry>,
    FilteringOptions {
        since,
        until,
        min_level,
        contains,
    }: FilteringOptions,
    direction: Direction,
) -> impl StreamingIterator<Item = LogEntry> {
    input
        .skip_while(move |entry| {
            entry
                .timestamp()
                .and_then(|timestamp| match direction {
                    Direction::Forward => since.map(|since| timestamp < since),
                    Direction::Reverse => until.map(|until| timestamp >= until),
                })
                .unwrap_or(false)
        })
        .take_while(move |entry| {
            entry
                .timestamp()
                .and_then(|timestamp| match direction {
                    Direction::Forward => until.map(|until| timestamp < until),
                    Direction::Reverse => since.map(|since| timestamp >= since),
                })
                .unwrap_or(true)
        })
        .filter(move |entry| {
            if let (Some(min_level), Some(level)) = (min_level, entry.level()) {
                (level as i32) >= (min_level as i32)
            } else {
                true
            }
        })
        .filter(move |entry| {
            if let Some(contains) = &contains {
                entry.contents().find(contains.as_ref()).is_some()
            } else {
                true
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log_entry::LogLevel;
    use chrono::NaiveDate;

    const LOG_INPUT: &[&[u8]] = &[
        b"-debug:<16866> 2020-01-01 20:00:00.000 UTC [A]: Text1",
        b"-info:<16866> 2020-01-01 21:00:00.000 UTC [A]: Text2",
        b"-warning:<16866> 2020-01-01 21:30:00.000 UTC [A]: Text3",
        b"-critical:<16866> 2020-01-01 22:00:00.000 UTC [A]: Text4",
        b"-fatal:<16866> 2020-01-01 22:30:00.000 UTC [A]: Text5",
    ];

    fn to_log_iter(
        input_iter: impl DoubleEndedIterator<Item = &'static [u8]>,
    ) -> impl StreamingIterator<Item = LogEntry> {
        streaming_iterator::convert(input_iter.map(|lines| LogEntry::from_contents(lines.to_vec())))
    }

    #[test]
    fn filtering_iter_no_filter() {
        let log_iter = to_log_iter(LOG_INPUT.iter().copied());
        let log_iter = filtering_iter(
            log_iter,
            FilteringOptions {
                since: None,
                until: None,
                contains: None,
                min_level: None,
            },
            Direction::Forward,
        );

        assert_eq!(log_iter.owned().collect::<Vec<_>>(), LOG_INPUT.to_vec());
    }

    #[test]
    fn filtering_iter_no_filter_rev() {
        let log_iter = to_log_iter(LOG_INPUT.iter().copied().rev());
        let log_iter = filtering_iter(
            log_iter,
            FilteringOptions {
                since: None,
                until: None,
                contains: None,
                min_level: None,
            },
            Direction::Reverse,
        );

        assert_eq!(
            log_iter.owned().collect::<Vec<_>>(),
            LOG_INPUT.iter().copied().rev().collect::<Vec<_>>()
        );
    }

    #[test]
    fn filtering_iter_since() {
        let log_iter = to_log_iter(LOG_INPUT.iter().copied());
        let log_iter = filtering_iter(
            log_iter,
            FilteringOptions {
                since: Some(NaiveDate::from_ymd(2020, 01, 01).and_hms(21, 30, 00)),
                until: None,
                contains: None,
                min_level: None,
            },
            Direction::Forward,
        );

        assert_eq!(
            log_iter.owned().collect::<Vec<_>>(),
            LOG_INPUT[2..].to_vec()
        );
    }

    #[test]
    fn filtering_iter_since_rev() {
        let log_iter = to_log_iter(LOG_INPUT.iter().copied().rev());
        let log_iter = filtering_iter(
            log_iter,
            FilteringOptions {
                since: Some(NaiveDate::from_ymd(2020, 01, 01).and_hms(21, 30, 00)),
                until: None,
                contains: None,
                min_level: None,
            },
            Direction::Reverse,
        );

        assert_eq!(
            log_iter.owned().collect::<Vec<_>>(),
            LOG_INPUT[2..].iter().copied().rev().collect::<Vec<_>>()
        );
    }

    #[test]
    fn filtering_iter_until() {
        let log_iter = to_log_iter(LOG_INPUT.iter().copied());
        let log_iter = filtering_iter(
            log_iter,
            FilteringOptions {
                since: None,
                until: Some(NaiveDate::from_ymd(2020, 01, 01).and_hms(21, 30, 00)),
                contains: None,
                min_level: None,
            },
            Direction::Forward,
        );

        assert_eq!(
            log_iter.owned().collect::<Vec<_>>(),
            LOG_INPUT[..2].to_vec()
        );
    }

    #[test]
    fn filtering_iter_until_rev() {
        let log_iter = to_log_iter(LOG_INPUT.iter().copied().rev());
        let log_iter = filtering_iter(
            log_iter,
            FilteringOptions {
                since: None,
                until: Some(NaiveDate::from_ymd(2020, 01, 01).and_hms(21, 30, 00)),
                contains: None,
                min_level: None,
            },
            Direction::Reverse,
        );

        assert_eq!(
            log_iter.owned().collect::<Vec<_>>(),
            LOG_INPUT[..2].iter().copied().rev().collect::<Vec<_>>()
        );
    }

    #[test]
    fn filtering_iter_min_level() {
        let log_iter = to_log_iter(LOG_INPUT.iter().copied());
        let log_iter = filtering_iter(
            log_iter,
            FilteringOptions {
                since: None,
                until: None,
                contains: None,
                min_level: Some(LogLevel::Critical),
            },
            Direction::Forward,
        );

        assert_eq!(
            log_iter.owned().collect::<Vec<_>>(),
            LOG_INPUT[3..].to_vec()
        );
    }

    #[test]
    fn filtering_iter_min_level_rev() {
        let log_iter = to_log_iter(LOG_INPUT.iter().copied().rev());
        let log_iter = filtering_iter(
            log_iter,
            FilteringOptions {
                since: None,
                until: None,
                contains: None,
                min_level: Some(LogLevel::Critical),
            },
            Direction::Reverse,
        );

        assert_eq!(
            log_iter.owned().collect::<Vec<_>>(),
            LOG_INPUT[3..].iter().copied().rev().collect::<Vec<_>>()
        );
    }

    #[test]
    fn filtering_iter_contains() {
        let log_iter = to_log_iter(LOG_INPUT.iter().copied());
        let log_iter = filtering_iter(
            log_iter,
            FilteringOptions {
                since: None,
                until: None,
                contains: Some("Text2".into()),
                min_level: None,
            },
            Direction::Forward,
        );

        assert_eq!(
            log_iter.owned().collect::<Vec<_>>(),
            LOG_INPUT[1..2].to_vec()
        );
    }

    #[test]
    fn filtering_iter_contains_rev() {
        let log_iter = to_log_iter(LOG_INPUT.iter().copied().rev());
        let log_iter = filtering_iter(
            log_iter,
            FilteringOptions {
                since: None,
                until: None,
                contains: Some("Text2".into()),
                min_level: None,
            },
            Direction::Reverse,
        );

        assert_eq!(
            log_iter.owned().collect::<Vec<_>>(),
            LOG_INPUT[1..2].iter().copied().rev().collect::<Vec<_>>()
        );
    }
}
