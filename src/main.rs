use timegather::TimeGather;

fn main() {
    let mut tg = TimeGather::new();
    let mut timer = tg.timer_named("My timer");
    let mut timer2 = tg.timer_named("My timer 2");
    std::thread::sleep(std::time::Duration::from_secs(1));
    timer.update("sleep");
    timer.update("nothing");
    std::thread::sleep(std::time::Duration::from_secs(1));
    timer.update("sleep");
    timer2.update("full");
    println!("{}", timer.to_string());
    println!("{}", timer2.to_string());
    println!("{}", tg.to_string());
}
