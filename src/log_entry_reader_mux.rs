use crate::log_entry::LogEntry;

pub struct LogEntryReaderMux<I: Iterator<Item = LogEntry>> {
    input_iters: Vec<I>,
    entries: Vec<LogEntry>,
}

impl<I: Iterator<Item = LogEntry>> LogEntryReaderMux<I> {
    pub fn new(mut input_iters: Vec<I>) -> Self {
        let entries = input_iters
            .iter_mut()
            .enumerate()
            .filter_map(|(index, iter)| iter.next().map(|entry| entry.with_source(index)))
            .collect();
        LogEntryReaderMux {
            input_iters,
            entries,
        }
    }
}

impl<I: Iterator<Item = LogEntry>> Iterator for LogEntryReaderMux<I> {
    type Item = LogEntry;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self
            .entries
            .iter_mut()
            .enumerate()
            .min_by_key(|(_, e)| e.timestamp());

        if let Some((entries_index, log_entry)) = item {
            let source = log_entry.source();
            if let Some(mut entry) = self.input_iters[source].next() {
                entry = entry.with_source(source);
                std::mem::swap(log_entry, &mut entry);
                Some(entry)
            } else {
                Some(self.entries.remove(entries_index))
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn log_entry_reader_mux() {
        let input1 = vec![
            LogEntry::new(b"-info:<16866> 2020-01-01 20:00:00.000 UTC [A]: B".to_vec()),
            LogEntry::new(b"-info:<16866> 2020-01-01 21:00:00.000 UTC [A]: B".to_vec()),
        ];
        let input2 = vec![
            LogEntry::new(b"-info:<16866> 2020-01-01 20:30:00.000 UTC [A]: B".to_vec()),
            LogEntry::new(b"-info:<16866> 2020-01-01 21:30:00.000 UTC [A]: B".to_vec()),
        ];

        let iterators = vec![input1.into_iter(), input2.into_iter()];
        let mut reader = LogEntryReaderMux::new(iterators);

        assert_eq!(
            reader.next().map(|e| (e.source(), e.timestamp())),
            Some((0, Some(NaiveDate::from_ymd(2020, 01, 01).and_hms(20, 0, 0))))
        );

        assert_eq!(
            reader.next().map(|e| (e.source(), e.timestamp())),
            Some((
                1,
                Some(NaiveDate::from_ymd(2020, 01, 01).and_hms(20, 30, 0))
            ))
        );

        assert_eq!(
            reader.next().map(|e| (e.source(), e.timestamp())),
            Some((0, Some(NaiveDate::from_ymd(2020, 01, 01).and_hms(21, 0, 0))))
        );

        assert_eq!(
            reader.next().map(|e| (e.source(), e.timestamp())),
            Some((
                1,
                Some(NaiveDate::from_ymd(2020, 01, 01).and_hms(21, 30, 0))
            ))
        );

        assert_eq!(reader.next(), None);
    }
}
