use tokio::sync::watch;
use tokio::time::{self, Duration};
use tracing::{error, info, warn};

#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::SystemInformation::GetTickCount;

use nvml_wrapper::enum_wrappers::device::TemperatureSensor;
use nvml_wrapper::Nvml;

/// Physical thermal ceiling to pause execution (82°C).
pub const MAX_TEMP_C: u32 = 82;

/// Physical thermal floor to resume execution (70°C).
pub const MIN_TEMP_C: u32 = 70;

/// Idle time threshold in seconds (5 minutes = 300 seconds).
pub const IDLE_THRESHOLD_SECS: u64 = 300;

/// Global system state emitted by the Thermal Governor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemState {
    Active,
    Throttled,
    Paused,
}

/// Retrieves the idle time of the user in seconds using Win32 GetLastInputInfo in O(1).
#[cfg(target_os = "windows")]
pub fn get_idle_seconds() -> u64 {
    unsafe {
        let mut lii = LASTINPUTINFO {
            cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
            dwTime: 0,
        };
        if GetLastInputInfo(&mut lii) != 0 {
            let now = GetTickCount();
            let elapsed_ms = now.wrapping_sub(lii.dwTime);
            (elapsed_ms / 1000) as u64
        } else {
            0
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn get_idle_seconds() -> u64 {
    0
}

/// Reads the current GPU temperature via NVML (Fail-Closed error handling).
pub fn read_gpu_temperature(nvml: &Nvml) -> Result<u32, String> {
    let device = nvml
        .device_by_index(0)
        .map_err(|e| format!("Failed to get GPU device index 0: {e}"))?;

    let temp = device
        .temperature(TemperatureSensor::Gpu)
        .map_err(|e| format!("Failed to read GPU temperature: {e}"))?;

    Ok(temp)
}

/// Evaluates the system state transition given the previous state, GPU temperature result, and idle time.
pub fn evaluate_state(
    current_state: SystemState,
    temp_result: Result<u32, String>,
    idle_secs: u64,
) -> SystemState {
    // Fail-Closed: If thermal read fails, log error and force Paused state.
    let temp = match temp_result {
        Ok(t) => t,
        Err(err) => {
            error!("Thermal Governor: Temperature reading failed. Entering Fail-Closed (Paused) state: {}", err);
            return SystemState::Paused;
        }
    };

    // IDLE CHECK: If user is active (idle < 300s), pause background workload.
    if idle_secs < IDLE_THRESHOLD_SECS {
        return SystemState::Paused;
    }

    // THERMAL HYSTERESIS & STATE MACHINE:
    if temp >= MAX_TEMP_C {
        // Upper thermal limit exceeded (>= 82°C) -> Pause immediately
        SystemState::Paused
    } else if current_state == SystemState::Paused {
        // Hysteresis rule: If was Paused, ONLY return to Active when temperature drops <= MIN_TEMP_C (70°C)
        if temp <= MIN_TEMP_C {
            SystemState::Active
        } else {
            SystemState::Paused
        }
    } else if temp >= 78 {
        // Approaching thermal ceiling while active -> Throttle workload
        SystemState::Throttled
    } else {
        SystemState::Active
    }
}

/// Spawns the Thermal Governor daemon loop on Tokio.
/// Returns the watch receiver for subscribing to SystemState changes.
pub fn spawn_thermal_governor() -> watch::Receiver<SystemState> {
    let (tx, rx) = watch::channel(SystemState::Active);

    tokio::spawn(async move {
        let mut current_state = SystemState::Active;
        let mut nvml_instance = match Nvml::init() {
            Ok(n) => Some(n),
            Err(e) => {
                error!("Thermal Governor: Failed to initialize NVML C-FFI: {}. Operating in Fail-Closed mode.", e);
                None
            }
        };

        let mut interval = time::interval(Duration::from_secs(5));

        loop {
            interval.tick().await;

            // Attempt re-initialization if NVML failed previously
            if nvml_instance.is_none() {
                if let Ok(n) = Nvml::init() {
                    info!("Thermal Governor: NVML driver successfully re-initialized.");
                    nvml_instance = Some(n);
                }
            }

            let temp_res = match &nvml_instance {
                Some(nvml) => read_gpu_temperature(nvml),
                None => Err("NVML C-FFI driver unavailable".to_string()),
            };

            let idle_secs = get_idle_seconds();
            let next_state = evaluate_state(current_state, temp_res, idle_secs);

            if next_state != current_state {
                info!(
                    "Thermal Governor: State transition [{:?} -> {:?}]",
                    current_state, next_state
                );
                current_state = next_state;
                if tx.send(current_state).is_err() {
                    warn!("Thermal Governor: All state receivers dropped.");
                }
            }
        }
    });

    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thermal_hysteresis_and_idle() {
        // Test 1: User active (< 300s) -> Paused
        assert_eq!(
            evaluate_state(SystemState::Active, Ok(65), 100),
            SystemState::Paused
        );

        // Test 2: User idle (>= 300s), low temp -> Active
        assert_eq!(
            evaluate_state(SystemState::Active, Ok(65), 300),
            SystemState::Active
        );

        // Test 3: Temp >= 82°C -> Paused
        assert_eq!(
            evaluate_state(SystemState::Active, Ok(82), 350),
            SystemState::Paused
        );

        // Test 4: Hysteresis hold: was Paused, temp is 75°C (> 70°C) -> Remains Paused
        assert_eq!(
            evaluate_state(SystemState::Paused, Ok(75), 350),
            SystemState::Paused
        );

        // Test 5: Cooled down: was Paused, temp drops to 70°C (<= 70°C) -> Returns to Active
        assert_eq!(
            evaluate_state(SystemState::Paused, Ok(70), 350),
            SystemState::Active
        );

        // Test 6: Fail-closed on NVML error -> Paused
        assert_eq!(
            evaluate_state(SystemState::Active, Err("NVML Error".into()), 350),
            SystemState::Paused
        );
    }
}
