use timegather::Timer;

fn main() {
    let timer = Timer::new();
    std::thread::sleep(std::time::Duration::from_secs(1));
    timer.update("sleep");
    timer.update("nothing");
    std::thread::sleep(std::time::Duration::from_secs(1));
    timer.update("sleep");
    std::thread::sleep(std::time::Duration::from_secs(1));
    timer.update("sleep");
    println!("{}", timer);

    //timer with id
    let timer = Timer::new();
    let id = timer.session();
    std::thread::sleep(std::time::Duration::from_secs(1));
    timer.update_with_session("sleep", id);
    timer.update_with_session("nothing", id);
    std::thread::sleep(std::time::Duration::from_secs(1));
    timer.update_with_session("sleep", id);
    let id = timer.session();
    std::thread::sleep(std::time::Duration::from_secs(1));
    timer.update_with_session("sleep", id);
    timer.update_with_session("nothing", id);
    std::thread::sleep(std::time::Duration::from_secs(1));
    timer.update_with_session("sleep", id);

    println!("{}", timer);
    println!("{:?}", timer.stats());
    println!("{:?}", timer.session_stats());
}
