use timegather::TimeGatherer;

fn main() {
    let tg = TimeGatherer::new();
    let mut timer = tg.timer();
    std::thread::sleep(std::time::Duration::from_secs(1));
    timer.update("sleep");
    std::thread::sleep(std::time::Duration::from_secs(1));
    timer.update("sleep");
    println!("{}", timer);
}
