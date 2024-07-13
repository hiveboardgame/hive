use centis::Centis;
pub mod centis;
pub mod clock_config;
pub mod decaying_stats;
pub mod lag_tracker;
pub mod stats;
use rand::Rng;

fn main() {
    let mut rng = rand::thread_rng();
    let base = rng.gen_range(1..20);
    let inc = rng.gen_range(1..20);
    for (base, inc) in [(1, 2), (3, 3), (5, 4), (10, 10), (20,20)] {
        println!("Playing a {}+{}", base, inc);
        let mut lt = lag_tracker::LagTracker::new(base * 60, inc);
        println!("Default quota is: {}", lt.quota.0);
        for _i in 0..10 {
            let num = rng.gen_range(30..501);
            println!(
                "Ping: {}ms Comp: {} Quota: {}",
                num,
                lt.on_move(Centis(num as f64 / 1000.0)).0,
                lt.quota.0
            );
        }
    }
}
