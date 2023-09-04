# Jetson Power Meter

Measure the power consumption of a program on the Jetson Nano platform, using built-in power meters.

## Building
After installing the Rust compiler, run
```
cargo build
```

or, if you want to build a release variant

```
cargo build --profile=release
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

## Setting the period
The sampling period defaults to 50ms, which seems to work good under the debug profile. However, in the release profile the loop seems to complete much faster, which means more overhead.  
Use a longer sampling period if the program is built with the release profile.

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
