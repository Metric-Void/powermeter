# Jetson Power Meter

Measure the power consumption of a program on the Jetson Nano platform, using built-in power meters.

## Building
After installing the Rust compiler, run
```
cargo build
```

## Using
```
Usage: jetson-meter [OPTIONS] [PROGRAM]

Arguments:
  [PROGRAM]  

Options:
  -s, --startup-ignore <STARTUP_IGNORE>  Portion of startup time to ignore. [default: 0.2]
  -e, --end-ignore <END_IGNORE>          Portion of terminating time to ignore. [default: 0.2]
  -p, --period <PERIOD>                  Sampling interval (ms) [default: 50]
  -h, --help                             Print help
```

The `[PROGRAM]` argument can be any bash script, including output redirect directives ( `>1`, etc.) Quote the script.

## Examples
The program needs superuser to access some counters.

```
sudo ./target/debug/jetson-meter --period 10 "/home/metricv/eembc/coremark/coremark.exe 0x0 0x0 0x66 100000 7 1 2000 > coremark.log"
```

The output will be:
```
During 9699742584 ns of running: 
    45308.607027613965 mJ energy is consumed.
    30559.177843179998 mJ energy is consumed by the CPU.
        27808.64723665399 mJ energy can be attributed to the target.
    0 mJ is consumed by the GPU.

System Power is 4.671114375999193 W
CPU Power is 3.1505143129868443 W
  Process CPU Power is 2.8669469314087923 W
GPU Power is 0 W
```