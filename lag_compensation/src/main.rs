pub mod clock_config;
pub mod decaying_stats;
pub mod lag_tracker;
pub mod stats;
use rand::Rng;

fn main() {
    let mut rng = rand::thread_rng();
    let base = rng.gen_range(1..20);
    let inc = rng.gen_range(1..20);
    for (base, inc) in [(300, 4)] {
        //, (30, 0), (60, 0), (120, 0), (180, 0)] {
        println!("Playing a {}+{}", base, inc);
        let mut lt = lag_tracker::LagTracker::new(base, inc);
        println!("Default quota is: {}", lt.quota);
        for _i in 0..35 {
            let num = rng.gen_range(500..601);
            println!(
                "Ping: {}ms Comp: {} Quota: {}",
                num,
                lt.on_move(num as f64 / 1000.0),
                lt.quota
            );
        }
    }
}
