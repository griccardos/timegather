use indexmap::IndexMap;
use std::fmt::Display;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub struct TimeGatherer {
    stats: Arc<Mutex<IndexedStats>>,
    sender: Sender<IndexedStats>,
}
impl Display for TimeGatherer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.stats.lock().unwrap())?;
        Ok(())
    }
}

impl TimeGatherer {
    pub fn new() -> Self {
        let (sender, receiver) = channel::<IndexedStats>();
        let stats = Arc::new(Mutex::new(IndexedStats(IndexMap::new())));
        let stats_clone = stats.clone();
        std::thread::spawn(move || {
            loop {
                if let Ok(stats) = receiver.recv() {
                    //updates our stats
                    let mut current = stats_clone.lock().unwrap();
                    for (tag, stat) in stats.0.iter() {
                        let curr_stat = current.0.entry(tag.clone()).or_insert(Stat::new());
                        curr_stat.max = curr_stat.max.max(stat.max);
                        curr_stat.min = curr_stat.min.min(stat.min);
                        curr_stat.total += stat.total;
                        curr_stat.count += stat.count;
                    }
                }
            }
        });
        Self { stats, sender }
    }
    pub fn timer(&self) -> Timer {
        Timer::new(self.sender.clone())
    }
}

pub struct Timer {
    stats: IndexedStats,
    instant: Instant,
    sender: Sender<IndexedStats>,
    start: Instant,
}

impl Timer {
    fn new(sender: Sender<IndexedStats>) -> Self {
        let now = Instant::now();
        Self {
            stats: IndexedStats::default(),
            instant: now.clone(),
            sender,
            start: now,
        }
    }
    pub fn update(&mut self, tag: &str) {
        let now = Instant::now();
        let d = now.duration_since(self.instant);
        self.instant = now;
        let stat = self.stats.0.entry(tag.to_string()).or_insert(Stat::new());
        stat.add(d);
    }
    pub fn total(&self) -> Duration {
        self.start.elapsed()
    }
}
impl Drop for Timer {
    fn drop(&mut self) {
        let stats = std::mem::take(&mut self.stats);

        let _ = self.sender.send(stats);
    }
}

impl Display for Timer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{} Total:{:.4}s", self.stats, self.total().as_secs_f64())?;
        Ok(())
    }
}

#[derive(Default)]
pub struct IndexedStats(IndexMap<String, Stat>);
impl Display for IndexedStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (k, v) in self.0.iter() {
            writeln!(f, "{}: {}", k, v)?;
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
