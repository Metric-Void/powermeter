extern crate num_cpus;
use clap::Parser;
use std::process::Command;
use std::fs;
use std::thread;
use std::time::{Duration, Instant};
use std::collections::HashMap;

#[derive(Parser)]
struct Args {
    program: Option<String>,

    #[arg(short, long, help="Portion of startup time to ignore.", default_value="0.2")]
    startup_ignore: f64,

    #[arg(short, long, help="Portion of terminating time to ignore.", default_value="0.2")]
    end_ignore: f64,

    #[arg(short, long, help="Sampling interval (ms)", default_value="50")]
    period: u32,

    #[arg(short, long, help="Record to disk instead of memory", default_value="false")]
    disk: bool,
}

struct CollectedPoint {
    time: Instant,
    power0: u64,
    power1: u64,
    power2: u64,
    sched: u64
}

fn main() {
    let ncpus = num_cpus::get();

    let args = Args::parse();
    let program = args.program.expect("Specify the program you want to run.");

    println!("{:?}", program);

    println!("{ncpus} CPUs detected, sampling interval is {} ms", args.period);

    let mut results = Vec::<CollectedPoint>::new();
    let target_spawn = Command::new("sh")
        .arg("-c")
        .arg("exec ".to_owned() + &program)
        .spawn();
    let mut target = target_spawn.expect("Failed to start process");
    let target_pid: u32 = target.id();

    println!("Started process {}", target.id());

    let mut child_stats_dict = HashMap::<String, u64>::new();

    loop {
        let power0: u64;
        let power1: u64;
        let power2: u64;

        let sched = fs::read_to_string(format!("/proc/{target_pid}/schedstat"));
        
        let pgrep_children_process = Command::new("/usr/bin/pgrep").arg("-P").arg(target_pid.to_string()).output().unwrap();

        let children = String::from_utf8_lossy(&pgrep_children_process.stdout);
        let children_list = children.lines();

        if children != "" {
            for child in children_list {
                let child_sched = fs::read_to_string(format!("/proc/{child}/schedstat"));
                if(child_sched.is_ok()) {
                    let child_cputime = child_sched.unwrap().trim().split_whitespace().next().unwrap().parse::<u64>().unwrap();
                    child_stats_dict.insert(child.to_owned(), child_cputime);
                }
            }
        }

        let mut cputime = sched.unwrap_or("0".to_string()).trim().split_whitespace().next().unwrap().parse::<u64>().unwrap();

        for (k, v) in &child_stats_dict {
            cputime += v;
        }

        power0 = fs::read_to_string("/sys/bus/i2c/drivers/ina3221x/6-0040/iio:device0/in_power0_input").expect("Failed to read Power 0 (System Power).").trim().parse::<u64>().unwrap();
        power1 = fs::read_to_string("/sys/bus/i2c/drivers/ina3221x/6-0040/iio:device0/in_power1_input").expect("Failed to read Power 1 (GPU Power).").trim().parse::<u64>().unwrap();
        power2 = fs::read_to_string("/sys/bus/i2c/drivers/ina3221x/6-0040/iio:device0/in_power2_input").expect("Failed to read Power 2 (CPU Power).").trim().parse::<u64>().unwrap();
        
        let data = CollectedPoint {
            time : Instant::now(),
            power0 : power0,
            power1 : power1,
            power2 : power2,
            sched : cputime
        };

        results.push(data);
        match target.try_wait() {
            Ok(Some(status)) => {
                println!("Target process has exited with {status}");
                break;
            }
            Ok(None) => {

            }
            Err(_) => { }
        }

        thread::sleep_ms(args.period);
    }

    target.wait();

    println!("Post-processing...");
    println!("Total datapoints collected: {}", results.len());

    if results.len() <= 3 {
        println!("Too little datapoints. Consider a longer-running program or reducing the sampling interval.");
        std::process::exit(1);
    }

    let mut start_index: usize = ((results.len() as f64) * args.startup_ignore).ceil() as usize;
    if start_index == 0 { start_index=1; }

    let end_index: usize = (results.len() as f64 - (results.len() as f64) * args.end_ignore).floor() as usize;

    // Units in mJ
    let mut energy_system_total: f64 = 0.0;
    let mut energy_cpu_total: f64 = 0.0;
    let mut energy_gpu_total: f64 = 0.0;
    let mut energy_cpu_share: f64 = 0.0;

    for i in (start_index .. end_index) {
        let sched_start = results[i-1].sched;
        let sched_end = results[i].sched;

        if(sched_start == 0) {
            println!("Err at Datapoint {}", sched_start);
            continue;
        }

        if(sched_end == 0) {
            println!("Err at Datapoint {}", sched_end);
            continue;
        }

        let sched_time = sched_end - sched_start;

        let dur = results[i].time.duration_since(results[i-1].time).as_nanos();

        // println!("During DP#{i}, time progressed {dur} and process used {sched_time} CPU time. System power is {} mW", results[i].power0);

        energy_system_total += results[i].power0 as f64 * (dur as f64 / 1000000000.0);
        energy_gpu_total += results[i].power1 as f64 * (dur as f64 / 1000000000.0);
        energy_cpu_total += results[i].power2 as f64 * (dur as f64 / 1000000000.0);
        energy_cpu_share += results[i].power2 as f64 * (dur as f64 / 1000000000.0) * (sched_time as f64 / (ncpus as f64 * dur as f64));

        // println!("{}", results[i-1].sched);
        // println!("{}", results[i].sched);
    }   

    let time_ns = (results[end_index].time - results[start_index].time).as_nanos();

    println!("");
    println!("During {time_ns} ns of running: ");
    println!("    {energy_system_total} mJ energy is consumed.");
    println!("    {energy_cpu_total} mJ energy is consumed by the CPU.");
    println!("        {energy_cpu_share} mJ energy can be attributed to the target.");
    println!("    {energy_gpu_total} mJ is consumed by the GPU.");
    println!("");
    println!("System Power is {} W", energy_system_total as f64 / time_ns as f64 * 1000000.0);
    println!("CPU Power is {} W", energy_cpu_total as f64 / time_ns as f64 * 1000000.0);
    println!("  Process CPU Power is {} W", energy_cpu_share as f64 / time_ns as f64 * 1000000.0);
    println!("GPU Power is {} W", energy_gpu_total as f64 / time_ns as f64 * 1000000.0);
}
