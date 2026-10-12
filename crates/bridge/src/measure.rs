use std::cell::RefCell;
use std::time::Duration;
// std's clock panics on wasm32-unknown-unknown, where this one asks the host.
use web_time::Instant;

/// Work a run leaves out so that what is left can be measured. A conversion that
/// skipped any is not a validated one, and says so.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Skip {
    Schema,
    Shapes,
    Findings,
}

impl Skip {
    pub fn named(name: &str) -> Option<Self> {
        match name {
            "schema" => Some(Self::Schema),
            "shapes" => Some(Self::Shapes),
            "findings" => Some(Self::Findings),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Schema => "schema",
            Self::Shapes => "shapes",
            Self::Findings => "findings",
        }
    }
}

/// The parts of a run that are timed, in the order they are reached. The first
/// four are loading an adapter; the rest are converting a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    LoadTables,
    LoadQueries,
    LoadSchemas,
    LoadVocabulary,
    DocumentTable,
    Lift,
    SourceValidation,
    Accounting,
    Dataset,
    Index,
    Mapping,
    Records,
    OutputValidation,
    Findings,
    UnresolvedAddresses,
    Detect,
    Summary,
    Serialising,
}

impl Stage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LoadTables => "loadTables",
            Self::LoadQueries => "loadQueries",
            Self::LoadSchemas => "loadSchemas",
            Self::LoadVocabulary => "loadVocabulary",
            Self::DocumentTable => "documentTable",
            Self::Lift => "lift",
            Self::SourceValidation => "sourceValidation",
            Self::Accounting => "accounting",
            Self::Dataset => "dataset",
            Self::Index => "index",
            Self::Mapping => "mapping",
            Self::Records => "records",
            Self::OutputValidation => "outputValidation",
            Self::Findings => "findings",
            Self::UnresolvedAddresses => "unresolvedAddresses",
            Self::Detect => "detect",
            Self::Summary => "summary",
            Self::Serialising => "serialising",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StageTime {
    pub stage: Stage,
    pub elapsed: Duration,
}

/// Time spent in each stage. A stage entered inside another is taken out of the
/// other, so the times add up to the time the outermost stages took. Reads no clock
/// unless it is on.
#[derive(Default)]
pub(crate) struct Stopwatch {
    on: bool,
    totals: RefCell<Vec<StageTime>>,
    running: RefCell<Vec<(Stage, Instant)>>,
}

impl Stopwatch {
    pub(crate) fn new(on: bool) -> Self {
        Self {
            on,
            ..Self::default()
        }
    }

    pub(crate) fn during<T>(&self, stage: Stage, work: impl FnOnce() -> T) -> T {
        if !self.on {
            return work();
        }
        self.enter(stage);
        let done = work();
        self.leave();
        done
    }

    pub(crate) fn current(&self) -> Option<Stage> {
        self.running.borrow().last().map(|&(stage, _)| stage)
    }

    pub(crate) fn spent_in(&self, stage: Stage) -> Duration {
        self.totals
            .borrow()
            .iter()
            .find(|time| time.stage == stage)
            .map_or(Duration::ZERO, |time| time.elapsed)
    }

    pub(crate) fn times(&self) -> Vec<StageTime> {
        let mut times = self.totals.borrow().clone();
        times.sort_by_key(|time| time.stage);
        times
    }

    fn enter(&self, stage: Stage) {
        let now = Instant::now();
        let mut running = self.running.borrow_mut();
        if let Some(&(outer, since)) = running.last() {
            self.credit(outer, now.duration_since(since));
        }
        running.push((stage, now));
    }

    fn leave(&self) {
        let now = Instant::now();
        let mut running = self.running.borrow_mut();
        if let Some((stage, since)) = running.pop() {
            self.credit(stage, now.duration_since(since));
        }
        if let Some((_, since)) = running.last_mut() {
            *since = now;
        }
    }

    fn credit(&self, stage: Stage, elapsed: Duration) {
        let mut totals = self.totals.borrow_mut();
        match totals.iter_mut().find(|time| time.stage == stage) {
            Some(time) => time.elapsed += elapsed,
            None => totals.push(StageTime { stage, elapsed }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Stage, Stopwatch};
    use std::time::Duration;
    use web_time::Instant;

    fn spend(milliseconds: u64) {
        std::thread::sleep(Duration::from_millis(milliseconds));
    }

    #[test]
    fn a_stage_entered_inside_another_is_taken_out_of_it() {
        let watch = Stopwatch::new(true);
        let started = Instant::now();
        watch.during(Stage::Mapping, || {
            spend(10);
            watch.during(Stage::Index, || spend(30));
            spend(10);
        });
        let whole = started.elapsed();
        let spent = |wanted: Stage| {
            watch
                .times()
                .into_iter()
                .find(|time| time.stage == wanted)
                .map(|time| time.elapsed)
                .expect("a stage that ran")
        };
        let (index, mapping) = (spent(Stage::Index), spent(Stage::Mapping));
        assert!(index >= Duration::from_millis(30), "{index:?}");
        assert!(mapping >= Duration::from_millis(20), "{mapping:?}");
        assert!(
            index + mapping <= whole,
            "{index:?} + {mapping:?} > {whole:?}"
        );
    }

    #[test]
    fn a_stage_run_again_adds_to_its_time_and_the_times_come_in_stage_order() {
        let watch = Stopwatch::new(true);
        watch.during(Stage::Findings, || spend(1));
        watch.during(Stage::Lift, || spend(1));
        watch.during(Stage::Lift, || spend(1));
        let stages: Vec<Stage> = watch.times().iter().map(|time| time.stage).collect();
        assert_eq!(stages, [Stage::Lift, Stage::Findings]);
    }

    #[test]
    fn a_stopwatch_that_is_off_runs_the_work_and_keeps_nothing() {
        let watch = Stopwatch::default();
        assert_eq!(watch.during(Stage::Lift, || 7), 7);
        assert!(watch.times().is_empty());
        assert_eq!(watch.current(), None);
    }

    #[test]
    fn a_query_is_run_in_the_stage_that_is_running() {
        let watch = Stopwatch::new(true);
        assert_eq!(
            watch.during(Stage::Mapping, || watch
                .during(Stage::Index, || watch.current())),
            Some(Stage::Index)
        );
        assert_eq!(
            watch.during(Stage::Findings, || watch.current()),
            Some(Stage::Findings)
        );
    }
}
