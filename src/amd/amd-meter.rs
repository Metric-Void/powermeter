mod amd;

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
    package: f64,
    cpu_power: f64,
    sched: u64
}

fn main() {
    let args = Args::parse();
    let ncpus = num_cpus::get();
    let program = args.program.expect("Specify the program you want to run.");

    let ctx_result = amd::AmdCpuContext::new();

    if ctx_result.is_err() {
        eprintln!("Unable to establish CPU context. Are you root?");
        eprintln!("{:#?}", ctx_result.unwrap_err());
        std::process::exit(1);
    }

    let ctx = ctx_result.unwrap();

    println!("{:?}", program);

    println!("{} Physical CPUs detected, sampling interval is {} ms", ctx.get_cores(), args.period);

    let mut results = Vec::<CollectedPoint>::new();
    let target_spawn = Command::new("sh")
        .arg("-c")
        .arg("exec ".to_owned() + &program)
        .spawn();
    let mut target = target_spawn.expect("Failed to start process");
    let target_pid: u32 = target.id();

    println!("Started process {}", target.id());

    let mut child_stats_dict = HashMap::<String, u64>::new();
    let mut cpu_energy_last = ctx.all_core_energy().unwrap();
    let mut package_energy_last: f64 = ctx.read_package_energy().unwrap();

    loop {
        /*
            CPU Time slicing
         */

        let sched = fs::read_to_string(format!("/proc/{target_pid}/schedstat"));
        
        let pgrep_children_process = Command::new("/usr/bin/pgrep").arg("-P").arg(target_pid.to_string()).output().unwrap();

        let children = String::from_utf8_lossy(&pgrep_children_process.stdout);
        let children_list = children.lines();

        if children != "" {
            for child in children_list {
                let child_sched = fs::read_to_string(format!("/proc/{child}/schedstat"));
                if child_sched.is_ok() {
                    let child_cputime = child_sched.unwrap().trim().split_whitespace().next().unwrap().parse::<u64>().unwrap();
                    child_stats_dict.insert(child.to_owned(), child_cputime);
                }
            }
        }

        let mut cputime = sched.unwrap_or("0".to_string()).trim().split_whitespace().next().unwrap().parse::<u64>().unwrap();

        for (_, v) in &child_stats_dict {
            cputime += v;
        }

        /*
            Power reading
         */

        let cpu_energy_r = ctx.all_core_energy();
        if cpu_energy_r.is_err() {
            eprintln!("Read MSR Error: Cannot read core energy.");
            std::process::exit(1);
        }
        let cpu_energy = cpu_energy_r.unwrap();

        let cpu_energy_delta: Vec<f64> = cpu_energy.clone().into_iter().zip(&cpu_energy_last).zip(0..ctx.get_cores()).map(|((a, b), c)| ctx.rollover(c, a - b)).collect();

        let cpu_energy_delta_sum: f64 = cpu_energy_delta.into_iter().sum();

        let pkg_energy_r = ctx.read_package_energy();
        if pkg_energy_r.is_err() {
            eprintln!("Read MSR Error: Cannot read package energy.");
            std::process::exit(1);
        }

        let pkg_energy = pkg_energy_r.unwrap();
        let pkg_energy_delta = ctx.rollover(0, pkg_energy - package_energy_last);


        let data = CollectedPoint {
            time : Instant::now(),
            package : pkg_energy_delta,
            cpu_power : cpu_energy_delta_sum,
            sched : cputime
        };

        results.push(data);
        match target.try_wait() {
            Ok(Some(status)) => {
                println!("Target process has exited with {status}");
                break;
            }
            Ok(None) => { }
            Err(_) => { }
        }

        cpu_energy_last = cpu_energy;
        package_energy_last = pkg_energy;

        thread::sleep(Duration::from_millis(args.period.into()));
    }

    let _ = target.wait();

    println!("Post-processing...");
    println!("Total datapoints collected: {}", results.len());

    if results.len() <= 3 {
        println!("Too few datapoints. Consider a longer-running program or reducing the sampling interval.");
        std::process::exit(1);
    }

    let mut start_index: usize = ((results.len() as f64) * args.startup_ignore).ceil() as usize;
    if start_index == 0 { start_index=1; }

    let end_index: usize = (results.len() as f64 - (results.len() as f64) * args.end_ignore).floor() as usize;

    // Units in mJ
    let mut energy_package_total: f64 = 0.0;
    let mut energy_cpu_total: f64 = 0.0;
    let mut energy_cpu_share: f64 = 0.0;

    for i in start_index .. end_index {
        let sched_start = results[i-1].sched;
        let sched_end = results[i].sched;

        if sched_start == 0 {
            println!("Err at Datapoint {}", sched_start);
            continue;
        }

        if sched_end == 0 {
            println!("Err at Datapoint {}", sched_end);
            continue;
        }

        let sched_time = sched_end - sched_start;

        let dur = results[i].time.duration_since(results[i-1].time).as_nanos();

        // println!("During DP#{i}, time progressed {dur} and process used {sched_time} CPU time. System power is {} mW", results[i].power0);

        energy_package_total += results[i].package as f64;
        energy_cpu_total += results[i].cpu_power as f64;
        energy_cpu_share += results[i].cpu_power as f64 * (sched_time as f64 / (ncpus as f64 * dur as f64));

        // println!("{}", results[i-1].sched);
        // println!("{}", results[i].sched);
    }   

    let time_ns = (results[end_index].time - results[start_index].time).as_nanos();

    println!("");
    println!("During {time_ns} ns ({}s) of running: ", time_ns as f64 / 1000000000.0);
    println!("    {energy_package_total} J package energy is consumed.");
    println!("    {energy_cpu_total} J energy is consumed by the CPU.");
    println!("        {energy_cpu_share} J energy can be attributed to the target.");
    println!("");
    println!("System Power is {} W", energy_package_total as f64 / time_ns as f64 * 1000000000.0);
    println!("CPU Power is {} W", energy_cpu_total as f64 / time_ns as f64 * 1000000000.0);
    println!("  Process CPU Power is {} W", energy_cpu_share as f64 / time_ns as f64 * 1000000000.0);
}
