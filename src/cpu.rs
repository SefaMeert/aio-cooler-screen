//! Built-in CPU sensors, so no extra monitoring app is needed:
//! - temperature through the PawnIO driver (installed by our installer): AMD Zen
//!   (families 17h-1Ah) via the SMN thermal register, Intel via the package thermal MSR;
//! - total load from Windows (GetSystemTimes).

use std::ffi::{c_char, c_void};
use std::path::PathBuf;

use libloading::os::windows::Library;

use crate::config;

type Handle = *mut c_void;
type OpenFn = unsafe extern "system" fn(*mut Handle) -> i32;
type LoadFn = unsafe extern "system" fn(Handle, *const u8, usize) -> i32;
type ExecuteFn = unsafe extern "system" fn(Handle, *const c_char, *const u64, usize, *mut u64, usize, *mut usize) -> i32;
type CloseFn = unsafe extern "system" fn(Handle) -> i32;

/// AMD family 17h+ THM_TCON_CUR_TMP: current Tctl in bits 31:21 (0.125 °C steps).
const AMD_THM_TCON_CUR_TMP: u64 = 0x0005_9800;
const AMD_TEMP_RANGE_SEL: u64 = 0x8_0000;
const INTEL_TEMPERATURE_TARGET: u64 = 0x1A2;
const INTEL_PACKAGE_THERM_STATUS: u64 = 0x1B1;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Vendor {
    Amd,
    Intel,
    Other,
}

#[derive(Clone, Debug)]
pub struct CpuInfo {
    vendor: Vendor,
    family: u32,
    pub name: String,
}

pub fn cpu_info() -> CpuInfo {
    use std::arch::x86_64::__cpuid;
    {
        let v = __cpuid(0);
        let vendor_bytes: Vec<u8> = [v.ebx, v.edx, v.ecx].iter().flat_map(|r| r.to_le_bytes()).collect();
        let vendor = match &vendor_bytes[..] {
            b"AuthenticAMD" => Vendor::Amd,
            b"GenuineIntel" => Vendor::Intel,
            _ => Vendor::Other,
        };
        let eax = __cpuid(1).eax;
        let base = (eax >> 8) & 0xF;
        let family = if base == 0xF { base + ((eax >> 20) & 0xFF) } else { base };
        let mut brand = Vec::with_capacity(48);
        if __cpuid(0x8000_0000).eax >= 0x8000_0004 {
            for leaf in 0x8000_0002..=0x8000_0004u32 {
                let r = __cpuid(leaf);
                brand.extend([r.eax, r.ebx, r.ecx, r.edx].iter().flat_map(|x| x.to_le_bytes()));
            }
        }
        let name = String::from_utf8_lossy(&brand).trim_matches(char::from(0)).trim().to_string();
        CpuInfo { vendor, family, name: if name.is_empty() { "CPU".into() } else { name } }
    }
}

fn module_path(name: &str) -> Option<PathBuf> {
    let exe_dir = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.join("pawnio")));
    [Some(config::install_dir().join("pawnio")), exe_dir].into_iter().flatten().map(|d| d.join(name)).find(|p| p.exists())
}

fn pawnio_lib() -> PathBuf {
    let base = std::env::var_os("ProgramFiles").map_or_else(|| PathBuf::from(r"C:\Program Files"), PathBuf::from);
    base.join(r"PawnIO\PawnIOLib.dll")
}

/// CPU temperature through PawnIO. Needs admin (the service runs as SYSTEM).
pub struct CpuTemp {
    _lib: Library,
    handle: Handle,
    execute: ExecuteFn,
    close: CloseFn,
    pci_mutex: Handle,
    cpu: CpuInfo,
    tjmax: f64,
}

// The PawnIO handle is used from the one sensor thread that owns this value.
unsafe impl Send for CpuTemp {}

impl CpuTemp {
    pub fn load() -> Result<Self, String> {
        use windows_sys::Win32::System::Threading::CreateMutexW;
        let cpu = cpu_info();
        let module = match cpu.vendor {
            Vendor::Amd if (0x17..=0x1A).contains(&cpu.family) => "AMDFamily17.bin",
            Vendor::Intel => "IntelMSR.bin",
            _ => return Err(format!("{} isn't supported yet", cpu.name)),
        };
        let dll = pawnio_lib();
        if !dll.exists() {
            return Err("PawnIO driver isn't installed (reinstall AIO Cooler Screen)".into());
        }
        let blob = module_path(module)
            .and_then(|p| std::fs::read(p).ok())
            .ok_or_else(|| format!("sensor module {module} is missing (reinstall AIO Cooler Screen)"))?;

        // SAFETY: PawnIOLib's documented C API; the handle stays owned by this struct.
        unsafe {
            let lib = Library::new(&dll).map_err(|e| format!("cannot load PawnIOLib: {e}"))?;
            let open = *lib.get::<OpenFn>(b"pawnio_open\0").map_err(|e| e.to_string())?;
            let load = *lib.get::<LoadFn>(b"pawnio_load\0").map_err(|e| e.to_string())?;
            let execute = *lib.get::<ExecuteFn>(b"pawnio_execute\0").map_err(|e| e.to_string())?;
            let close = *lib.get::<CloseFn>(b"pawnio_close\0").map_err(|e| e.to_string())?;

            let mut handle: Handle = std::ptr::null_mut();
            let hr = open(&mut handle);
            if hr < 0 {
                return Err(format!("cannot open PawnIO (0x{hr:08X}); it needs administrator rights"));
            }
            let hr = load(handle, blob.as_ptr(), blob.len());
            if hr < 0 {
                close(handle);
                return Err(format!("PawnIO rejected {module} (0x{hr:08X})"));
            }
            // Shared with HWiNFO, LibreHardwareMonitor and others that use the PCI index/data ports.
            let name: Vec<u16> = r"Global\Access_PCI".encode_utf16().chain(Some(0)).collect();
            let pci_mutex = CreateMutexW(std::ptr::null(), 0, name.as_ptr());
            let mut t = CpuTemp { _lib: lib, handle, execute, close, pci_mutex, cpu, tjmax: 100.0 };
            if t.cpu.vendor == Vendor::Intel {
                if let Ok(v) = t.execute1(c"ioctl_read_msr", INTEL_TEMPERATURE_TARGET) {
                    let tj = ((v >> 16) & 0xFF) as f64;
                    if tj > 0.0 {
                        t.tjmax = tj;
                    }
                }
            }
            t.read().ok_or("the CPU temperature register returned no value")?;
            Ok(t)
        }
    }

    pub fn cpu_name(&self) -> &str {
        &self.cpu.name
    }

    fn execute1(&self, func: &std::ffi::CStr, input: u64) -> Result<u64, String> {
        let mut out = 0u64;
        let mut written = 0usize;
        // SAFETY: one input and one output u64, as the module functions declare.
        let hr = unsafe { (self.execute)(self.handle, func.as_ptr(), &input, 1, &mut out, 1, &mut written) };
        if hr < 0 || written != 1 {
            return Err(format!("PawnIO call failed (0x{hr:08X})"));
        }
        Ok(out)
    }

    /// Current temperature in °C and the reading's name.
    pub fn read(&self) -> Option<(f64, &'static str)> {
        use windows_sys::Win32::Foundation::{WAIT_ABANDONED, WAIT_OBJECT_0};
        use windows_sys::Win32::System::Threading::{ReleaseMutex, WaitForSingleObject};
        match self.cpu.vendor {
            Vendor::Amd => {
                // SAFETY: valid mutex handle (or null, then we read unguarded).
                let locked = !self.pci_mutex.is_null()
                    && matches!(unsafe { WaitForSingleObject(self.pci_mutex, 100) }, WAIT_OBJECT_0 | WAIT_ABANDONED);
                let v = self.execute1(c"ioctl_read_smn", AMD_THM_TCON_CUR_TMP);
                if locked {
                    unsafe { ReleaseMutex(self.pci_mutex) };
                }
                let v = v.ok()?;
                let mut t = ((v >> 21) & 0x7FF) as f64 * 0.125;
                if v & AMD_TEMP_RANGE_SEL != 0 {
                    t -= 49.0;
                }
                (t > 0.0).then_some((t, "CPU (Tctl/Tdie)"))
            }
            Vendor::Intel => {
                let v = self.execute1(c"ioctl_read_msr", INTEL_PACKAGE_THERM_STATUS).ok()?;
                // Bit 31: reading valid; bits 22:16: degrees below TjMax.
                (v & (1 << 31) != 0).then(|| (self.tjmax - ((v >> 16) & 0x7F) as f64, "CPU Package"))
            }
            Vendor::Other => None,
        }
    }
}

impl Drop for CpuTemp {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::CloseHandle;
        // SAFETY: handles owned by this struct.
        unsafe {
            (self.close)(self.handle);
            if !self.pci_mutex.is_null() {
                CloseHandle(self.pci_mutex);
            }
        }
    }
}

/// Total CPU load from the difference between two GetSystemTimes samples.
#[derive(Default)]
pub struct CpuLoad {
    prev: Option<(u64, u64)>,
}

impl CpuLoad {
    pub fn read(&mut self) -> Option<f64> {
        use windows_sys::Win32::Foundation::FILETIME;
        use windows_sys::Win32::System::Threading::GetSystemTimes;
        let ft = |f: FILETIME| (u64::from(f.dwHighDateTime) << 32) | u64::from(f.dwLowDateTime);
        let zero = || FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
        let (mut idle, mut kernel, mut user) = (zero(), zero(), zero());
        // SAFETY: three valid out-pointers.
        if unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) } == 0 {
            return None;
        }
        // Kernel time includes idle time.
        let (idle, total) = (ft(idle), ft(kernel) + ft(user));
        let prev = self.prev.replace((idle, total))?;
        let (di, dt) = (idle.saturating_sub(prev.0), total.saturating_sub(prev.1));
        (dt > 0).then(|| (dt - di.min(dt)) as f64 * 100.0 / dt as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_this_cpu() {
        let c = cpu_info();
        assert_ne!(c.name, "");
        assert!(c.family > 0);
    }

    #[test]
    fn cpu_load_is_a_percentage() {
        let mut l = CpuLoad::default();
        assert!(l.read().is_none(), "first sample has no delta");
        std::thread::sleep(std::time::Duration::from_millis(200));
        let v = l.read().unwrap();
        assert!((0.0..=100.0).contains(&v));
    }
}
