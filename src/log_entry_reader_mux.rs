use crate::direction::Direction;
use crate::log_entry::LogEntry;
use std::cmp::Ordering;
use streaming_iterator::StreamingIterator;

pub struct LogEntryReaderMux<I: StreamingIterator<Item = LogEntry>> {
    input_iters: Vec<I>,
    curr: Option<usize>,
    cmp_fn: fn(&(usize, &I), &(usize, &I)) -> Ordering,
}

impl<I: StreamingIterator<Item = LogEntry>> LogEntryReaderMux<I> {
    fn min_iter((_, iter1): &(usize, &I), (_, iter2): &(usize, &I)) -> Ordering {
        iter1
            .get()
            .expect("Finished iter1 called")
            .timestamp()
            .cmp(&iter2.get().expect("Finished iter2 called").timestamp())
    }

    fn max_iter(t1: &(usize, &I), t2: &(usize, &I)) -> Ordering {
        Self::min_iter(t1, t2).reverse()
    }
}

impl<I: StreamingIterator<Item = LogEntry>> LogEntryReaderMux<I> {
    pub fn new(input_iters: Vec<I>, direction: Direction) -> Self {
        LogEntryReaderMux {
            input_iters,
            curr: None,
            cmp_fn: match direction {
                Direction::Forward => Self::min_iter,
                Direction::Reverse => Self::max_iter,
            },
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
            .min_by(self.cmp_fn)
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
    use chrono::{Datelike, Timelike};

    const LOG_INPUTS: &[&[&[u8]]] = &[
        &[
            b"-info:<16866> 2020-01-01 20:00:00.000 UTC [A]: B",
            b"-info:<16866> 2020-01-01 21:00:00.000 UTC [A]: B",
        ],
        &[
            b"-info:<16866> 2020-01-01 20:30:00.000 UTC [A]: B",
            b"-info:<16866> 2020-01-01 21:30:00.000 UTC [A]: B",
        ],
    ];

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct GoldenResult(usize, Option<(i32, u32, u32, u32, u32, u32)>);

    impl From<LogEntry> for GoldenResult {
        fn from(e: LogEntry) -> Self {
            GoldenResult(
                e.source(),
                e.timestamp().map(|t| {
                    (
                        t.year(),
                        t.month(),
                        t.day(),
                        t.hour(),
                        t.minute(),
                        t.second(),
                    )
                }),
            )
        }
    }

    const GOLDEN_RESULTS: &[GoldenResult] = &[
        GoldenResult(0, Some((2020, 01, 01, 20, 0, 0))),
        GoldenResult(1, Some((2020, 01, 01, 20, 30, 0))),
        GoldenResult(0, Some((2020, 01, 01, 21, 0, 0))),
        GoldenResult(1, Some((2020, 01, 01, 21, 30, 0))),
    ];

    #[test]
    fn log_entry_reader_mux() {
        let iterators = LOG_INPUTS
            .iter()
            .enumerate()
            .map(|(source, log)| {
                log.iter()
                    .map(|lines| LogEntry::from_contents(lines.to_vec()).with_source(source))
                    .collect::<Vec<_>>()
            })
            .map(streaming_iterator::convert)
            .collect();

        let reader = LogEntryReaderMux::new(iterators, Direction::Forward);
        let results: Vec<GoldenResult> = reader.owned().map(Into::into).collect();

        assert_eq!(results.as_slice(), GOLDEN_RESULTS);
    }

    #[test]
    fn log_entry_reader_mux_reverse() {
        let iterators = LOG_INPUTS
            .iter()
            .enumerate()
            .map(|(source, log)| {
                log.iter()
                    .map(|lines| LogEntry::from_contents(lines.to_vec()).with_source(source))
                    .rev()
                    .collect::<Vec<_>>()
            })
            .map(streaming_iterator::convert)
            .collect();

        let reader = LogEntryReaderMux::new(iterators, Direction::Reverse);
        let results: Vec<GoldenResult> = reader.owned().map(Into::into).collect();

        assert_eq!(
            results,
            GOLDEN_RESULTS.iter().copied().rev().collect::<Vec<_>>()
        );
    }
}
