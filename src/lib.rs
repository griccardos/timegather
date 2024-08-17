use indexmap::IndexMap;
use std::fmt::Display;
use std::sync::{Arc, Mutex};
use std::thread::ThreadId;
use std::time::{Duration, Instant};

/// Timer is a simple utility to measure the time taken by different parts of the code.
/// It is thread-safe and can be used to measure the time taken by different threads.
/// multi threaded example:
/// ```rust
/// use timegather::Timer;
/// use std::thread;
/// let timer = Timer::new();
/// let timer_clone = timer.clone(); //timer can be cloned and moved to another thread
/// let h1 = std::thread::spawn(move || {
///     timer_clone.update("dummy"); //for more accurate measurement of next line
///     std::thread::sleep(std::time::Duration::from_secs(1));
///     timer_clone.update("sleep");
/// });
/// h1.join().unwrap();
/// println!("{}", timer);
/// println!("{}", timer.stats().get("sleep").unwrap());
/// ```
#[derive(Clone)]
pub struct Timer(Arc<Mutex<TimerInner>>);
impl Display for Timer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.lock().unwrap())
    }
}

impl Timer {
    /// Creates a new Timer.
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(TimerInner::new())))
    }
    /// Used AFTER the code you want to measure, and saves duration with the tag.
    /// Uses the duration since the last measurement.
    /// If you want to measure a specific part of the code, you can add a dummy tag before the code.
    /// e.g.
    /// ```rust
    /// use timegather::Timer;
    /// let timer = Timer::new();
    /// std::thread::sleep(std::time::Duration::from_secs(1));//code to measure
    /// timer.update("sleep");
    /// assert_eq!(timer.stats().get("sleep").unwrap().count, 1);
    /// assert!((timer.stats().get("sleep").unwrap().total.as_secs_f64() - 1.).abs() < 0.01);
    /// ```
    ///
    pub fn update(&self, tag: &str) {
        let thread_id = std::thread::current().id();
        let mut inner = self.0.lock().unwrap();
        let now = Instant::now();
        let start = inner.start.clone();
        let last_instant = inner.instant.entry(thread_id.clone()).or_insert(start);
        let d = now.duration_since(last_instant.clone());
        *last_instant = now;
        inner
            .stats
            .entry((thread_id, tag.to_string()))
            .or_insert_with(Stat::new)
            .add_duration(d);
    }

    pub fn stats(&self) -> IndexMap<String, Stat> {
        combine(&self.0.lock().unwrap().stats)
    }
    pub fn total(&self) -> Duration {
        self.0.lock().unwrap().total()
    }
    #[allow(dead_code)]
    fn debug_stats(&self) -> IndexMap<(ThreadId, String), Stat> {
        self.0.lock().unwrap().stats.clone()
    }
}

pub struct TimerInner {
    stats: IndexMap<(ThreadId, String), Stat>, //key is the threadid and the tag, once the thread is done, we move to threadid 0
    instant: IndexMap<ThreadId, Instant>,      //need an instant for each thread
    start: Instant,
    name: Option<String>,
}

impl TimerInner {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            stats: Default::default(),
            instant: Default::default(),
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
        //combine all the stats, ignore the threadid
        let combined = combine(&self.stats);
        match combined.len() {
            0 => write!(f, "No tags")?,
            _ => {
                for (i, (k, v)) in combined.iter().enumerate() {
                    write!(f, "{}: {}", k, v)?;
                    if i < combined.len() - 1 {
                        write!(f, "\n")?;
                    }
                }
            }
        }
        if combined.len() > 1 {
            write!(f, "\nTotal: {:.4}s", self.total().as_secs_f64())?;
        }
        Ok(())
    }
}

fn combine(stats: &IndexMap<(ThreadId, String), Stat>) -> IndexMap<String, Stat> {
    stats
        .iter()
        .fold(IndexMap::new(), |mut acc, ((_, tag), stat)| {
            acc.entry(tag.clone()).or_insert_with(Stat::new).add(stat);
            acc
        })
}

#[derive(Default, Clone, Debug)]
pub struct Stat {
    pub count: usize,
    pub total: Duration,
    pub min: Duration,
    pub max: Duration,
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
    pub fn avg(&self) -> Duration {
        self.total / self.count as u32
    }
    fn add_duration(&mut self, d: Duration) {
        self.count += 1;
        self.total += d;
        self.min = self.min.min(d);
        self.max = self.max.max(d);
    }
    fn add(&mut self, other: &Stat) {
        self.count += other.count;
        self.total += other.total;
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
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
            self.avg().as_secs_f64(),
            self.max.as_secs_f64()
        )
    }
}

#[cfg(test)]
mod tests {
    use rayon::iter::{IntoParallelIterator, ParallelIterator};

    use super::*;
    trait RoundTo {
        fn round_to(&self, places: u32) -> Self;
    }
    impl RoundTo for u128 {
        fn round_to(&self, places: u32) -> Self {
            let factor = 10u128.pow(places);
            (self + factor / 2) / factor * factor
        }
    }
    #[test]
    fn test_timer() {
        let timer = Timer::new();
        std::thread::sleep(std::time::Duration::from_secs(1));
        timer.update("sleep");
        timer.update("nothing");
        std::thread::sleep(std::time::Duration::from_secs(1));
        timer.update("sleep");

        let stats = timer.stats();
        println!("{timer}");
        assert_eq!(stats.len(), 2);
        assert_eq!(stats["sleep"].count, 2);
        assert_eq!(stats["nothing"].count, 1);
        assert_eq!(stats["sleep"].total.as_millis().round_to(2), 2000);
        assert_eq!(stats["nothing"].total.as_millis().round_to(2), 0);
    }

    #[test]
    fn multiple_threads() {
        let timer = Timer::new();
        let timer_clone = timer.clone();
        let h1 = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(1));
            timer_clone.update("sleep");
        });
        let timer3 = timer.clone();
        let h2 = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(1));
            timer3.update("sleep");
        });
        timer.update("nothing");
        h1.join().unwrap();
        h2.join().unwrap();
        println!("{}", timer.to_string());
        let stats = timer.stats();
        println!("{:?}", timer.debug_stats());
        assert_eq!(stats.len(), 2);
        assert_eq!(stats["sleep"].count, 2);
        assert_eq!(stats["nothing"].count, 1);
        assert_eq!(stats["sleep"].total.as_millis().round_to(2), 2000);
        assert_eq!(stats["nothing"].total.as_millis().round_to(2), 0);
    }

    #[test]
    fn rayon() {
        let timer = Timer::new();
        let timer2 = timer.clone();
        let count = 4;
        (0..count).into_par_iter().for_each(move |_| {
            std::thread::sleep(std::time::Duration::from_secs(1));
            timer2.update("sleep");
        });

        println!("{}", timer.to_string());
        let stats = timer.stats();
        println!("{:?}", timer.debug_stats());
        assert_eq!(stats.len(), 1);
        assert_eq!(stats["sleep"].count, count);
        assert_eq!(
            stats["sleep"].total.as_millis().round_to(3),
            count as u128 * 1000
        );
        assert_eq!(stats["sleep"].avg().as_millis().round_to(2), 1000);
    }
    #[test]
    fn doc() {
        let timer = Timer::new();
        timer.update("dummy");
        std::thread::sleep(std::time::Duration::from_secs(1));
        timer.update("sleep");
        assert_eq!(timer.stats().get("sleep").unwrap().count, 1);
        assert!((timer.stats().get("sleep").unwrap().total.as_secs_f64() - 1.).abs() < 0.01);
    }
    #[test]
    fn doc2() {
        let timer = Timer::new();
        let timer_clone = timer.clone(); //timer can be cloned and moved to another thread
        let h1 = std::thread::spawn(move || {
            timer_clone.update("dummy"); //for more accurate measurement of next line
            std::thread::sleep(std::time::Duration::from_secs(1));
            timer_clone.update("sleep");
        });
        h1.join().unwrap();
        println!("{}", timer);
        println!("{}", timer.stats().get("sleep").unwrap());
    }
}
