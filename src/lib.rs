use indexmap::IndexMap;
use std::fmt::Display;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub struct TimeGather {
    timers: Vec<Arc<Mutex<TimerInner>>>,
}
impl Display for TimeGather {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.timers.len() {
            0 => writeln!(f, "No timers")?,
            1 => write!(f, "")?,
            n => writeln!(f, "{} timers", n)?,
        }
        for (i, timer) in self.timers.iter().enumerate() {
            write!(f, "{}", timer.lock().unwrap())?;
            if i < self.timers.len() - 1 {
                write!(f, "\n")?;
            }
        }
        Ok(())
    }
}

impl TimeGather {
    pub fn new() -> Self {
        Self { timers: Vec::new() }
    }
    pub fn timer(&mut self) -> Timer {
        let timer = Arc::new(Mutex::new(TimerInner::new()));
        self.timers.push(timer.clone());
        Timer(timer)
    }
    pub fn timer_named(&mut self, name: &str) -> Timer {
        let mut inner = TimerInner::new();
        inner.name = Some(name.to_string());
        let timer = Timer(Arc::new(Mutex::new(inner)));
        self.timers.push(timer.0.clone());
        timer
    }
}
pub struct Timer(Arc<Mutex<TimerInner>>);
impl Display for Timer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.lock().unwrap())
    }
}

impl Timer {
    pub fn update(&mut self, tag: &str) {
        let now = Instant::now();
        let mut timer = self.0.lock().unwrap();
        let d = now.duration_since(timer.instant);
        timer.instant = now;
        let stat = timer.stats.0.entry(tag.to_string()).or_insert(Stat::new());
        stat.add(d);
    }
    pub fn total(&self) -> Duration {
        self.0.lock().unwrap().total()
    }
}

pub struct TimerInner {
    stats: IndexedStats,
    instant: Instant,
    start: Instant,
    name: Option<String>,
}

impl TimerInner {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            stats: IndexedStats::default(),
            instant: now.clone(),
            start: now,
            name: None,
        }
    }

    fn total(&self) -> Duration {
        self.start.elapsed()
    }
}

impl Display for TimerInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(name) = &self.name {
            writeln!(f, "{}:", name)?;
        }
        match self.stats.0.len() {
            0 => write!(f, "No tags")?,
            _ => write!(f, "{}", self.stats,)?,
        }
        if self.stats.0.len() > 1 {
            write!(f, "\nTotal: {:.4}s", self.total().as_secs_f64())?;
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct IndexedStats(IndexMap<String, Stat>);
impl Display for IndexedStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, (k, v)) in self.0.iter().enumerate() {
            write!(f, "{}: {}", k, v)?;
            if i < self.0.len() - 1 {
                write!(f, "\n")?;
            }
        }
        Ok(())
    }
}
#[derive(Default)]
struct Stat {
    count: usize,
    total: Duration,
    min: Duration,
    max: Duration,
}
impl Stat {
    fn new() -> Self {
        Self {
            count: 0,
            total: Duration::default(),
            min: Duration::from_secs(1000000),
            max: Duration::from_secs(0),
        }
    }
    fn add(&mut self, d: Duration) {
        self.count += 1;
        self.total += d;
        self.min = self.min.min(d);
        self.max = self.max.max(d);
    }
}

impl Display for Stat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Count({}), Total({:.4}s), Min({:.4}s), Avg({:.4}s), Max({:.4}s)",
            self.count,
            self.total.as_secs_f64(),
            self.min.as_secs_f64(),
            self.total.as_secs_f64() / self.count as f64,
            self.max.as_secs_f64()
        )
    }
}
