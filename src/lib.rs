use indexmap::IndexMap;
use std::fmt::Display;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// A simple utility to measure the time taken by different parts of the code.
/// It is thread-safe and can be used to measure the time taken by different threads.
///
/// A multi-threaded example:
/// ```rust
/// use timegather::Timer;
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
impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}

impl Timer {
    /// Creates a new Timer.
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(TimerInner::new())))
    }
    /// Used AFTER the code you want to measure, and saves duration since the last measurement with a given `tag`.
    /// Uses the duration since the last measurement. (see `update_with_session` function to measure against a session when using threads which may run concurrently e.g. rayon )
    /// If you want to measure a specific part of the code and ignore everything before, you can add a dummy tag before the code.
    ///
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
        let mut inner = self.0.lock().unwrap();
        let now = Instant::now();
        let d = now.duration_since(inner.last);
        inner.last = now;
        inner
            .stats
            .entry((Session::none(), tag.to_string()))
            .or_default()
            .add_duration(d);
    }

    /// create session at beginning of thread or block you want to measure
    pub fn session(&self) -> Session {
        let session = Session::new();
        let mut inner = self.0.lock().unwrap();
        let now = Instant::now();
        inner.instant.entry(session).or_insert(now);
        session
    }

    /// Same as `update`, but only measures time elapsed since last update agaist this session.
    /// This is useful when you have multiple threads running concurrently and you want to measure the time taken by each thread,
    /// for example rayon or thread::spawn.
    ///
    /// use `session()` to get the session id
    pub fn update_with_session(&self, tag: &str, session: Session) {
        let mut inner = self.0.lock().unwrap();
        let now = Instant::now();
        let last_instant = inner.instant.entry(session).or_insert(now);
        let d = now.duration_since(*last_instant);
        *last_instant = now;
        inner
            .stats
            .entry((session, tag.to_string()))
            .or_default()
            .add_duration(d);
    }

    pub fn stats(&self) -> IndexMap<String, Stat> {
        combine(&self.0.lock().unwrap().stats)
    }
    pub fn total(&self) -> Duration {
        self.0.lock().unwrap().total()
    }
    pub fn session_stats(&self) -> IndexMap<(Session, String), Stat> {
        self.0.lock().unwrap().stats.clone()
    }
}

pub(crate) struct TimerInner {
    stats: IndexMap<(Session, String), Stat>, //key is the session and the tag
    instant: IndexMap<Session, Instant>,      //need an instant for each session
    last: Instant,
    start: Instant,
    name: Option<String>,
}
#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct Session(usize);
impl Session {
    pub(crate) fn none() -> Self {
        Self(0)
    }
    pub(crate) fn new() -> Self {
        //generate a unique id using rand, cannot be 0
        Self(rand::random::<usize>().max(1))
    }
}

impl TimerInner {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            stats: Default::default(),
            instant: Default::default(),
            last: now,
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
                        writeln!(f)?;
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

fn combine(stats: &IndexMap<(Session, String), Stat>) -> IndexMap<String, Stat> {
    stats
        .iter()
        .fold(IndexMap::new(), |mut acc, ((_, tag), stat)| {
            acc.entry(tag.clone()).or_default().add(stat);
            acc
        })
}

#[derive(Default, Clone, Debug)]
pub struct Stat {
    pub count: usize,
    pub total: Duration,
    pub min: Duration,
    pub max: Duration,
    pub id_count: usize,
}
impl Stat {
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
        self.id_count += 1;
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
            self.max.as_secs_f64(),
        )?;
        if self.id_count > 1 {
            write!(f, " [over {} Ids]", self.id_count)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use rayon::iter::{IntoParallelIterator, ParallelIterator};
    use tokio::join;

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
            let id = timer_clone.session();
            std::thread::sleep(std::time::Duration::from_secs(1));
            timer_clone.update_with_session("sleep", id);
        });
        let timer3 = timer.clone();
        let h2 = std::thread::spawn(move || {
            let id = timer3.session();
            std::thread::sleep(std::time::Duration::from_secs(1));
            timer3.update_with_session("sleep", id);
        });
        timer.update("nothing");
        h1.join().unwrap();
        h2.join().unwrap();
        timer.update("join");
        println!("{}", timer.to_string());
        let stats = timer.stats();
        println!("{:?}", timer.session_stats());
        assert_eq!(stats.len(), 3);
        assert_eq!(stats["sleep"].count, 2);
        assert_eq!(stats["nothing"].count, 1);
        assert_eq!(stats["sleep"].total.as_millis().round_to(2), 2000);
        assert_eq!(stats["nothing"].total.as_millis().round_to(2), 0);
        assert_eq!(stats["join"].total.as_millis().round_to(2), 1000);
        assert_eq!(timer.total().as_millis().round_to(2), 1000);
    }

    #[test]
    fn rayon() {
        let timer = Timer::new();
        let timer_clone = timer.clone();
        let count = 4;
        (0..count).into_par_iter().for_each(move |_| {
            let id = timer_clone.session();
            std::thread::sleep(std::time::Duration::from_secs(1));
            timer_clone.update_with_session("sleep", id);
        });

        println!("{}", timer.to_string());
        let stats = timer.stats();
        println!("{:?}", timer.session_stats());
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 5)]
    async fn in_async_function() {
        let timer = Timer::new();

        for _ in 0..5 {
            let timer_clone = timer.clone();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                timer_clone.update("sleep");
            })
            .await
            .unwrap();
        }

        println!("{}", timer);
        assert_eq!(timer.total().as_millis().round_to(2), 500);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 5)]
    async fn in_async_function_join() {
        let timer = Timer::new();

        async fn sleep(timer: Timer) {
            let id = timer.session();
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            timer.update_with_session("sleep", id);
        }
        let a = sleep(timer.clone());
        let b = sleep(timer.clone());
        join!(a, b);

        println!("{}", timer);
        assert_eq!(timer.total().as_millis().round_to(2), 100);
        assert_eq!(
            timer
                .stats()
                .get("sleep")
                .unwrap()
                .total
                .as_millis()
                .round_to(2),
            200
        );
    }

    #[test]
    fn test_speed() {
        let timer = Timer::new();
        for _ in 0..1000 {
            timer.update("dummy");
        }
        let time = timer.total().as_millis();
        println!("{time}",);
        assert!(time < 1);
    }
    #[test]
    fn test_speed_par() {
        let timer = Timer::new();
        (0..1000).into_par_iter().for_each(|_| {
            timer.update("dummy");
        });
        let time = timer.total().as_millis();
        println!("{time}",);
        assert!(time < 5);
    }
}
