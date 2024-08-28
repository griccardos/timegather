# timegather

Timegather is a simple utility to measure the time taken by different parts of the code.  
It is thread-safe and can be used to measure the time taken by different threads.  

Usually we only need to create the timer with `Timer::new()` and then use `update` to measure the time taken by different parts of the code. 

One only needs to use `session` and `update_with_session` if we are in a parallel environment or measurements will be happening at the same time

## Basic example:

```rust
use timegather::Timer;
let timer = Timer::new();
timer.update("dummy"); // (optional) for more accurate measurement of next line
std::thread::sleep(std::time::Duration::from_secs(1));
timer.update("sleep");//measures the time since the last measurement (or creation of timer)
println!("{}", timer);
println!("{}", timer.stats().get("sleep").unwrap());
```


## A multi-threaded example:
Because threads are run at the same time, if we compare to last measurement, we will get inaccurate results.  
Instead we create a session each time, and the updates will be unique to that session.  
The results of all sessions are then merged together.
```rust
let timer = Timer::new();
let timer_clone = timer.clone();
let count = 4;
(0..count).into_par_iter().for_each(move |_| {
    let id = timer_clone.session();
    std::thread::sleep(std::time::Duration::from_secs(1));
    timer_clone.update_with_session("sleep", id);
});
println!("{}", timer.to_string());
```
