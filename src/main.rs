extern crate num_cpus;
use clap::Parser;
use std::process::Command;
use std::time::Duration;
use std::fs;

#[derive(Parser)]
struct Args {
    program: Option<String>,

    #[arg(short, long, help="Portion of startup time to ignore.", default_value="0.2")]
    startup_ignore: f64,

    #[arg(short, long, help="Portion of terminating time to ignore.", default_value="0.2")]
    end_ignore: f64,

    #[arg(short, long, help="Sampling interval (ms)", default_value="50")]
    period: u64
}

struct CollectedPoint {
    time: u64,
    power0: u64,
    power1: u64,
    power2: u64,
    sched: String
}

fn collect_stats(pid: u32, results: &mut Vec<CollectedPoint>) {
    // Read Powers
    let power0: u64;
    let power1: u64;
    let power2: u64;

    power0 = fs::read_to_string("/sys/bus/i2c/drivers/ina3221x/6-0040/iio:device0/in_power0_input").expect("Failed to read Power 0 (System Power).").parse::<u64>().unwrap();
    power1 = fs::read_to_string("/sys/bus/i2c/drivers/ina3221x/6-0040/iio:device0/in_power1_input").expect("Failed to read Power 1 (GPU Power).").parse::<u64>().unwrap();
    power2 = fs::read_to_string("/sys/bus/i2c/drivers/ina3221x/6-0040/iio:device0/in_power2_input").expect("Failed to read Power 2 (CPU Power).").parse::<u64>().unwrap();

    let sched = fs::read_to_string(format!("/proc/{pid}/schedstat"));
}

fn main() {
    let ncpus = num_cpus::get();

    println!("{ncpus} CPUs detected.");

    let args = Args::parse();
    let program = args.program.expect("Specify the program you want to run.");

    println!("{:?}", program);

    let mut results = Vec::<CollectedPoint>::new();
    let target_spawn = Command::new("sh")
        .arg("-c")
        .arg("exec ".to_owned() + &program)
        .spawn();
    let mut target = target_spawn.expect("Failed to start process");
    let target_pid: u32 = target.id();

    println!("Started process {}", target.id());

    let mut planner = periodic::Planner::new();
    planner.add(
        || collect_stats(target_pid, &mut results),
        periodic::Every::new(Duration::from_millis(args.period))
    );

    target.wait();
}
