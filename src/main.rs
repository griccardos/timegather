use timegather::Timer;

fn main() {
    let timer = Timer::new();
    std::thread::sleep(std::time::Duration::from_secs(1));
    timer.update("sleep");
    timer.update("nothing");
    std::thread::sleep(std::time::Duration::from_secs(1));
    timer.update("sleep");
    println!("{}", timer.to_string());
}
