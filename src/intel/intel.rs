use std::{fs::File, os::unix::prelude::FileExt, usize};

extern crate num_cpus;

const MSR_RAPL_POWER_UNIT:      u64 = 0x606;
const MSR_PP0_ENERGY_STATUS:    u64 = 0x639;
const MSR_PKG_ENERGY_STATUS:    u64 = 0x611;

const TIME_UNIT_MASK:       u64 = 0xF0000;
const ENERGY_UNIT_MASK:     u64 = 0x1F00;
const POWER_UNIT_MASK:      u64 = 0xF;

#[derive(Debug)]
pub struct IntelCpuContext {
    cores: usize,

    core_msr_fds: Vec<File>,

    time_units: Vec<f64>,

    energy_units: Vec<f64>,

    power_units: Vec<f64>
}

fn read_msr(core: u64, addr: u64) -> Result<u64, std::io::Error> {
    let msr_file = File::open(format!("/dev/cpu/{core}/msr"))?;

    let mut buffer: u64 = 0;
    let buffer_ptr_u64 = std::ptr::addr_of_mut!(buffer);

    unsafe {
        let buffer_ptr: &mut [u8; 8] = &mut*buffer_ptr_u64.cast::<[u8; 8]>();

        msr_file.read_at(buffer_ptr , addr)?;
    }
    return Ok(buffer);
}

fn read_msr_safe(core: u64, addr: u64) -> Result<u64, std::io::Error> {
    let msr_file = File::open(format!("/dev/cpu/{core}/msr"))?;

    let mut buffer: [u8; 8] = [0; 8];

    msr_file.read_at(&mut buffer , addr)?;

    return Ok(u64::from_le_bytes(buffer));
}

impl IntelCpuContext {
    fn __read_u64_msr_with_fd(fd: &File, addr: u64) -> Result<u64, std::io::Error> {
        let mut buffer: [u8; 8] = [0; 8];

        fd.read_at(&mut buffer, addr)?;
        return Ok(u64::from_le_bytes(buffer));
    }

    fn __read_u64_msr_from_core(&self, core: usize, addr: u64) -> Result<u64, std::io::Error> {
        let mut buffer: [u8; 8] = [0; 8];

        self.core_msr_fds[core].read_at(&mut buffer, addr)?;
        Ok(u64::from_le_bytes(buffer))
    }

    pub fn new() -> Result<IntelCpuContext, std::io::Error> {
        let ncores = num_cpus::get();
        let realcores = num_cpus::get_physical();

        let mut core_msr_fds = Vec::<File>::new();
        let mut time_units: Vec<f64> = Vec::<f64>::new();
        let mut energy_units: Vec<f64> = Vec::<f64>::new();
        let mut power_units: Vec<f64> = Vec::<f64>::new();

        for i in 0..realcores {
            let core_fd = File::options().read(true).write(false).open(format!("/dev/cpu/{}/msr", i))?;
            
            let amd_msr_pwr_unit = IntelCpuContext::__read_u64_msr_with_fd(&core_fd, MSR_RAPL_POWER_UNIT)?;
            
            let time_unit_raw: u64 = (amd_msr_pwr_unit & TIME_UNIT_MASK) >> 16;
            let energy_unit_raw: u64 = (amd_msr_pwr_unit & ENERGY_UNIT_MASK) >> 8;
            let power_unit_raw: u64 = amd_msr_pwr_unit & POWER_UNIT_MASK;

            core_msr_fds.push(core_fd);
            time_units.push(0.5_f64.powi(time_unit_raw as i32));
            energy_units.push(0.5_f64.powi(energy_unit_raw as i32));
            power_units.push(0.5_f64.powi(power_unit_raw as i32));
        }

        Ok(IntelCpuContext{
            cores: realcores,
            core_msr_fds,
            time_units,
            energy_units,
            power_units
        })
    }

    pub fn get_cores(&self) -> usize { self.cores }

    pub fn read_package_energy(&self) -> Result<f64, std::io::Error> {
        let package_raw = self.__read_u64_msr_from_core(0, MSR_PKG_ENERGY_STATUS)?;
        
        Ok(package_raw as f64 * self.energy_units[0])
    }

    pub fn read_core_energy(&self, core: usize) -> Option<f64> {
        if core >= self.cores {
            return None
        }

        let core_raw = self.__read_u64_msr_from_core(core, MSR_PP0_ENERGY_STATUS);
        if core_raw.is_ok() {
            Some(core_raw.unwrap() as f64 * self.energy_units[core])
        } else {
            None
        }
    }

    pub fn all_core_energy_sum(&self) -> Result<f64, std::io::Error> {
        let mut sum = 0.0_f64;
        for core in 0..self.cores {
            let core_raw = self.__read_u64_msr_from_core(core, MSR_PP0_ENERGY_STATUS);
            if core_raw.is_ok() {
                sum += core_raw.unwrap() as f64 * self.energy_units[core];
            } else {
                return Err(core_raw.unwrap_err())
            }
        }

        Ok(sum)
    }

    pub fn all_core_energy(&self) -> Result<Vec<f64>, std::io::Error> {
        let mut result = Vec::<f64>::new();

        for core in 0..self.cores {
            let core_raw = self.__read_u64_msr_from_core(core, MSR_PP0_ENERGY_STATUS);
            if core_raw.is_ok() {
                result.push(core_raw.unwrap() as f64 * self.energy_units[core]);
            } else {
                return Err(core_raw.unwrap_err())
            }
        }

        Ok(result)
    }

    pub fn rollover(&self, core: usize, val: f64) -> f64 {
        if val < 0_f64 {
            val + u32::MAX as f64 * self.energy_units[core]
        } else {
            val
        }
    }
}