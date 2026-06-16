use std::sync::atomic::{AtomicUsize, Ordering};

static ACTIVE_CAPTURE_SESSIONS: AtomicUsize = AtomicUsize::new(0);

/// Prevent Windows 11 from putting Tandem into EcoQoS / timer-throttling when minimized.
pub fn disable_process_power_throttling() {
    use windows::Win32::Media::timeBeginPeriod;
    use windows::Win32::System::Threading::{
        GetCurrentProcess, ProcessPowerThrottling, SetProcessInformation,
        PROCESS_POWER_THROTTLING_CURRENT_VERSION, PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
        PROCESS_POWER_THROTTLING_IGNORE_TIMER_RESOLUTION, PROCESS_POWER_THROTTLING_STATE,
    };

    let state = PROCESS_POWER_THROTTLING_STATE {
        Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
        ControlMask: PROCESS_POWER_THROTTLING_EXECUTION_SPEED
            | PROCESS_POWER_THROTTLING_IGNORE_TIMER_RESOLUTION,
        StateMask: 0,
    };

    unsafe {
        let _ = timeBeginPeriod(1);

        if let Err(error) = SetProcessInformation(
            GetCurrentProcess(),
            ProcessPowerThrottling,
            &raw const state as *const _,
            std::mem::size_of::<PROCESS_POWER_THROTTLING_STATE>() as u32,
        ) {
            tracing::warn!(%error, "could not disable Windows process power throttling");
        } else {
            tracing::info!("disabled Windows background power throttling for tandem-client");
        }
    }
}

/// Raise timer resolution and thread priority for capture / streaming workers.
pub fn configure_high_priority_worker_thread() {
    use windows::Win32::Media::timeBeginPeriod;
    use windows::Win32::System::Threading::{
        AvSetMmThreadCharacteristicsW, GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_HIGHEST,
    };

    unsafe {
        let _ = timeBeginPeriod(1);

        if let Err(error) = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_HIGHEST) {
            tracing::debug!(%error, "could not raise worker thread priority");
        }

        let mut task_index = 0u32;
        match AvSetMmThreadCharacteristicsW(windows::core::w!("Capture"), &mut task_index) {
            Ok(handle) => {
                std::mem::forget(handle);
            }
            Err(error) => {
                tracing::debug!(%error, "could not mark worker thread as multimedia");
            }
        }
    }
}

pub struct ActiveCaptureGuard;

impl ActiveCaptureGuard {
    pub fn acquire() -> Self {
        let previous = ACTIVE_CAPTURE_SESSIONS.fetch_add(1, Ordering::SeqCst);
        if previous == 0 {
            set_streaming_execution_state(true);
        }

        Self
    }
}

impl Drop for ActiveCaptureGuard {
    fn drop(&mut self) {
        let previous = ACTIVE_CAPTURE_SESSIONS.fetch_sub(1, Ordering::SeqCst);
        if previous == 1 {
            set_streaming_execution_state(false);
        }
    }
}

fn set_streaming_execution_state(active: bool) {
    use windows::Win32::System::Power::{SetThreadExecutionState, ES_CONTINUOUS, ES_SYSTEM_REQUIRED};

    let flags = if active {
        ES_CONTINUOUS | ES_SYSTEM_REQUIRED
    } else {
        ES_CONTINUOUS
    };

    unsafe {
        let _ = SetThreadExecutionState(flags);
    }
}
