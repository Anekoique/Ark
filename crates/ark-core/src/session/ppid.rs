//! Cross-platform parent process id provider.
//!
//! Production code uses [`RealPpid`], whose body is `cfg`-gated: Unix delegates
//! to `std::os::unix::process::parent_id`; Windows walks the toolhelp snapshot
//! to find the parent of `GetCurrentProcessId()`. Tests use [`StubPpid`] for
//! deterministic injection.
//!
//! Windows toolhelp failure degrades to `std::process::id()` so the CLI never
//! aborts on a one-time WinAPI hiccup. The trade-off is that the same shell's
//! next CLI invocation gets a different PID and therefore a fresh session id;
//! focus is lost between invocations on broken Windows. This is the documented
//! degradation contract.

/// Source of parent-process id for the current process.
///
/// Trait-shaped so tests can inject deterministic values without touching
/// global state. The CLI dispatch layer constructs [`RealPpid`] once and
/// passes a `&dyn Ppid` through the call graph.
pub trait Ppid {
    /// Returns the parent process id, or this process's own id on failure.
    fn parent_id(&self) -> u32;
}

/// Production parent-id provider.
#[derive(Debug, Default, Clone, Copy)]
pub struct RealPpid;

impl RealPpid {
    /// Creates a new provider.
    pub fn new() -> Self {
        Self
    }
}

#[cfg(unix)]
impl Ppid for RealPpid {
    fn parent_id(&self) -> u32 {
        std::os::unix::process::parent_id()
    }
}

#[cfg(windows)]
impl Ppid for RealPpid {
    fn parent_id(&self) -> u32 {
        windows::lookup_parent_id().unwrap_or_else(std::process::id)
    }
}

/// Test parent-id provider.
///
/// `StubPpid(12345).parent_id() == 12345`.
#[derive(Debug, Clone, Copy)]
pub struct StubPpid(pub u32);

impl Ppid for StubPpid {
    fn parent_id(&self) -> u32 {
        self.0
    }
}

#[cfg(windows)]
mod windows {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
        System::{
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
                TH32CS_SNAPPROCESS,
            },
            Threading::GetCurrentProcessId,
        },
    };

    /// Walks the toolhelp snapshot and returns the parent of the calling process.
    ///
    /// Returns `None` on snapshot creation failure or when the walk completes
    /// without finding the calling pid (which itself indicates a stale snapshot).
    pub(super) fn lookup_parent_id() -> Option<u32> {
        let me = unsafe { GetCurrentProcessId() };
        let snap = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
        if snap == INVALID_HANDLE_VALUE {
            return None;
        }
        let mut entry: PROCESSENTRY32W = unsafe { core::mem::zeroed() };
        entry.dwSize = core::mem::size_of::<PROCESSENTRY32W>() as u32;

        let mut found = None;
        // SAFETY: `entry.dwSize` is set; `snap` is a valid snapshot handle until
        // CloseHandle below; pointer is exclusive to this stack frame.
        if unsafe { Process32FirstW(snap, &mut entry) } != 0 {
            loop {
                if entry.th32ProcessID == me {
                    found = Some(entry.th32ParentProcessID);
                    break;
                }
                // SAFETY: same as above; loop terminates when Process32NextW
                // returns 0 (no more entries) or we find the matching pid.
                if unsafe { Process32NextW(snap, &mut entry) } == 0 {
                    break;
                }
            }
        }
        // SAFETY: `snap` was returned by CreateToolhelp32Snapshot and not yet closed.
        unsafe { CloseHandle(snap) };
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that the production provider returns a non-zero parent id on
    /// the current platform. This is a smoke test; it runs against whichever
    /// OS CI executes.
    #[test]
    fn real_ppid_returns_nonzero_on_current_platform() {
        assert_ne!(RealPpid::new().parent_id(), 0);
    }

    /// Verifies that the stub returns the held value verbatim.
    #[test]
    fn stub_ppid_returns_held_value() {
        assert_eq!(StubPpid(12345).parent_id(), 12345);
        assert_eq!(StubPpid(67890).parent_id(), 67890);
    }
}
