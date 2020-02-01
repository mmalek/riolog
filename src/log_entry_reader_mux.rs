use crate::log_entry::LogEntry;
use streaming_iterator::StreamingIterator;

pub struct LogEntryReaderMux<I: StreamingIterator<Item = LogEntry>> {
    input_iters: Vec<I>,
    curr: Option<usize>,
}

impl<I: StreamingIterator<Item = LogEntry>> LogEntryReaderMux<I> {
    pub fn new(input_iters: Vec<I>) -> Self {
        LogEntryReaderMux {
            input_iters,
            curr: None,
        }
    }
}

impl<I: StreamingIterator<Item = LogEntry>> StreamingIterator for LogEntryReaderMux<I> {
    type Item = LogEntry;

    fn advance(&mut self) {
        if let Some(curr) = self.curr {
            let curr_iter = &mut self.input_iters[curr];
            curr_iter.advance();
            if curr_iter.get().is_none() {
                self.input_iters.remove(curr);
                self.curr = None;
            }
        } else {
            self.input_iters.iter_mut().for_each(|i| i.advance());
            self.input_iters.retain(|i| i.get().is_some());
        }

        self.curr = self
            .input_iters
            .iter()
            .enumerate()
            .min_by_key(|(_, iter)| iter.get().expect("Finished iter called").timestamp())
            .map(|(index, _)| index);
    }

    fn get(&self) -> Option<&Self::Item> {
        self.curr
            .and_then(|idx| self.input_iters.get(idx))
            .and_then(|iter| iter.get())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn log_entry_reader_mux() {
        let input1 = vec![
            LogEntry::from_contents(b"-info:<16866> 2020-01-01 20:00:00.000 UTC [A]: B".to_vec())
                .with_source(0),
            LogEntry::from_contents(b"-info:<16866> 2020-01-01 21:00:00.000 UTC [A]: B".to_vec())
                .with_source(0),
        ];
        let input2 = vec![
            LogEntry::from_contents(b"-info:<16866> 2020-01-01 20:30:00.000 UTC [A]: B".to_vec())
                .with_source(1),
            LogEntry::from_contents(b"-info:<16866> 2020-01-01 21:30:00.000 UTC [A]: B".to_vec())
                .with_source(1),
        ];

        let iterators = vec![
            streaming_iterator::convert(input1),
            streaming_iterator::convert(input2),
        ];
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
