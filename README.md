# Power Meter

Measure the CPU power consumption of a program on Intel and AMD CPUs

## Supported Models

### Intel
Supported:
- Intel Atom® Processors with a CPUID Signature DisplayFamily_DisplayModel Value of 06_37H, 06_4AH, 06_5AH, or 06_5DH
  - Intel Atom processor E3000 series, Z3600 series, Z3700 series
  - Intel Atom processor Z3400 series
  - Intel Atom processor Z3500 series
  - Intel Atom processor X3-C3000 based on Silvermont microarchitecture
- Intel Atom® Processors Based on Goldmont Microarchitecture
- 2nd Generation Intel® Core™ Processors (Sandy Bridge Microarchitecture)
- Intel® Xeon® Processors E5 Family Based on Sandy Bridge Microarchitecture
- 3rd Generation Intel® Core™ Processors Based on Ivy Bridge Microarchitecture
- Intel® Xeon® Processor E5 v2 Product Family Based on Ivy Bridge-E Microarchitecture
- 4th Generation Intel® Core™ Processors (Haswell Microarchitecture)
- Intel® Core™ M Processors and 5th Generation Intel® Core™ Processors
- 6th Generation, 7th Generation, 8th Generation, 9th Generation, 10th Generation, 11th Generation, 12th Generation, and 13th Generation Intel® Core™ Processors, Intel® Xeon® Scalable Processor Family, 2nd, 3rd, and 4th Generation Intel® Xeon® Scalable Processor Family, 8th Generation Intel® Core™ i3 Processors, and Intel® Xeon® E Processors
- Intel® Xeon Phi™ Processors with a CPUID Signature DisplayFamily_DisplayModel Value of 06_57H or 06_85H
  - Intel® Xeon Phi™ Processor 7215, 7285, 7295 Series based on Knights Mill microarchitecture
  - Intel® Xeon Phi™ Processor 3200, 5200, 7200 Series based on Knights Landing microarchitecture

Not Supported (Can only read package energy. No core and process energy.):
- Intel® Xeon® Processor E5 v3 Family
- Intel® Xeon® Processor D and the Intel® Xeon® Processor E5 v4 Family Based on Broadwell Microarchitecture
- Intel® Xeon® Scalable Processor Family with a CPUID Signature DisplayFamily_DisplayModel Value of 06_55H
  - Intel® Xeon® Scalable Processor Family based on Skylake microarchitecture,
  - 2nd generation Intel® Xeon® Scalable Processor Family based on Cascade Lake product,
  - 3rd generation Intel® Xeon® Scalable Processor Family based on Cooper Lake product

### AMD
AMD documentation is really incomplete, and the MSRs used for measuring power consumption is pulled out of nowhere. Try for yourself.

Verified Working
- AMD Ryzen 9 7950X

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
sudo ./target/debug/intel-meter "/home/metricv/eembc/coremark/coremark.exe 0x0 0x0 0x66 100000 7 1 2000 > coremark.log"
```

The output will be:
```
During 9173236413 ns (9.173236413s) of running:
    259.1939697265625 J package energy is consumed.
    3896.0634765625 J energy is consumed by the CPU.
        162.31696084183224 J energy can be attributed to the target.

System Power is 28.255455114973554 W
CPU Power is 424.7207093715726 W
  Process CPU Power is 17.694623089818347 W
```
