use crate::cli::FilteringOptions;
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
) -> impl StreamingIterator<Item = LogEntry> {
    input
        .skip_while(move |entry| {
            if let (Some(timestamp), Some(since)) = (entry.timestamp(), since) {
                timestamp < since
            } else {
                false
            }
        })
        .take_while(move |entry| {
            if let (Some(timestamp), Some(until)) = (entry.timestamp(), until) {
                timestamp < until
            } else {
                true
            }
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
