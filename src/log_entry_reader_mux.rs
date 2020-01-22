use crate::log_entry::LogEntry;
use std::cmp::Ordering;

pub struct LogEntryReaderMux<I: Iterator<Item = LogEntry>> {
    input_iters: Vec<I>,
    entries: Vec<(usize, Option<LogEntry>)>,
}

impl<I: Iterator<Item = LogEntry>> LogEntryReaderMux<I> {
    pub fn new(mut input_iters: Vec<I>) -> Self {
        let entries = input_iters
            .iter_mut()
            .map(|i| i.next())
            .enumerate()
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
        self.entries.retain(|(_, entry)| entry.is_some());

        let item = self.entries.iter_mut().min_by(|(_, x), (_, y)| {
            if let (Some(x), Some(y)) = (x, y) {
                x.timestamp().cmp(&y.timestamp())
            } else if x.is_some() {
                Ordering::Less
            } else if y.is_some() {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        });

        if let Some((i, log_entry)) = item {
            let ret_log_entry = log_entry.take();
            *log_entry = self.input_iters[*i].next();
            ret_log_entry.map(|e| e.with_source(*i))
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
            reader.next().and_then(|e| e.timestamp()),
            Some(NaiveDate::from_ymd(2020, 01, 01).and_hms(20, 0, 0))
        );

        assert_eq!(
            reader.next().and_then(|e| e.timestamp()),
            Some(NaiveDate::from_ymd(2020, 01, 01).and_hms(20, 30, 0))
        );

        assert_eq!(
            reader.next().and_then(|e| e.timestamp()),
            Some(NaiveDate::from_ymd(2020, 01, 01).and_hms(21, 0, 0))
        );

        assert_eq!(
            reader.next().and_then(|e| e.timestamp()),
            Some(NaiveDate::from_ymd(2020, 01, 01).and_hms(21, 30, 0))
        );

        assert_eq!(reader.next(), None);
    }
}
