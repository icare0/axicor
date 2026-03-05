/// genesis-runtime IPC client — communicates with genesis-baker-daemon.
///
/// Transport:
///   - Data:    POSIX SHM `/genesis_shard_{zone_id}` (mmap, no copies)
///   - Control: Unix domain socket (JSON-line, single command per Night Phase)
///
/// Usage:
///   1. Call `BakerClient::connect(zone_id, socket_path)` at startup.
///   2. Call `run_night(weights, targets, padded_n, timeout)` during Night Phase.
///   3. Returns updated targets (with sprouted connections filled in).
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use genesis_core::ipc::{shm_name, shm_size, ShmHeader, ShmState, SHM_MAGIC, SHM_VERSION, NightPhaseRequest, NightPhaseResponse};
use serde_json;

// POSIX SHM wrappers (libc calls)
use std::ffi::CString;

/// Runtime-side IPC client for the baker daemon.
pub struct BakerClient {
    zone_id: u16,
    socket_path: std::path::PathBuf,
    pub shm_ptr: *mut u8,
    pub shm_len: usize,
}

// SAFETY: BakerClient is not Send/Sync by default due to raw pointer.
// We implement them manually — the mmap region is only accessed from the
// Night Phase (single-threaded path in runtime main loop).
unsafe impl Send for BakerClient {}
unsafe impl Sync for BakerClient {}

impl BakerClient {
    /// Open and mmap the SHM segment, then validate the header written by daemon.
    /// The daemon must already be running and have created the SHM before this is called.
    pub fn connect(zone_id: u16, socket_path: &Path) -> Result<Self> {
        let name = shm_name(zone_id);
        let c_name = CString::new(name.as_str()).unwrap();

        // Open existing SHM segment (daemon creates it at startup)
        let fd = unsafe { libc::shm_open(c_name.as_ptr(), libc::O_RDWR, 0o600) };
        if fd < 0 {
            bail!(
                "shm_open({}) failed: {}",
                name,
                std::io::Error::last_os_error()
            );
        }

        // Read header to learn the real size
        let header_size = std::mem::size_of::<ShmHeader>();
        let mut hdr = std::mem::MaybeUninit::<ShmHeader>::uninit();
        let n = unsafe { libc::read(fd, hdr.as_mut_ptr() as *mut _, header_size) };
        if n < header_size as isize {
            unsafe { libc::close(fd) };
            bail!("SHM too small to read header");
        }
        let hdr = unsafe { hdr.assume_init() };
        hdr.validate()
            .map_err(|e| anyhow::anyhow!("SHM header invalid: {e}"))?;

        let shm_len = shm_size(hdr.padded_n as usize);

        // Map the full segment
        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                shm_len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            )
        };
        unsafe { libc::close(fd) };

        if ptr == libc::MAP_FAILED {
            bail!("mmap failed: {}", std::io::Error::last_os_error());
        }

        Ok(Self {
            zone_id,
            socket_path: socket_path.to_path_buf(),
            shm_ptr: ptr as *mut u8,
            shm_len,
        })
    }

    /// Run one Night Phase Sprouting cycle:
    /// 1. Write weights+targets into SHM
    /// 2. Signal daemon (`night_start`)
    /// 3. Wait for `night_done` (or `error`) with timeout
    /// 4. Return updated targets from SHM
    /// Run one Night Phase Sprouting cycle:
    /// 1. Signal daemon (`night_start`) via JSON NightPhaseRequest
    /// 2. Daemon mutates SHM directly
    /// 3. Wait for `night_done` (or `error`) with timeout
    pub fn run_night(
        &mut self,
        handovers: &[genesis_core::ipc::AxonHandoverEvent],
        padded_n: usize,
        timeout: Duration,
    ) -> Result<()> {
        // ── 1. Prepare Request ──
        let shm_path = format!("/dev/shm{}", shm_name(self.zone_id));
        let req = genesis_core::ipc::NightPhaseRequest {
            zone_name: format!("zone_{}", self.zone_id), // TODO: get real name
            shm_path,
            padded_n,
            weights_offset: self.header().weights_offset as usize,
            targets_offset: self.header().targets_offset as usize,
            handovers: handovers.to_vec(),
        };

        // ── 2. Connect to daemon socket and send request ──
        let mut stream = UnixStream::connect(&self.socket_path)
            .with_context(|| format!("Cannot connect to baker socket {:?}", self.socket_path))?;
        stream
            .set_read_timeout(Some(timeout))
            .context("set_read_timeout")?;

        let msg = serde_json::to_string(&req)?;
        writeln!(stream, "{}", msg)?;
        stream.flush()?;

        // ── 4. Wait for response ──
        let mut reader = BufReader::new(&stream);
        let mut line = String::new();
        reader.read_line(&mut line).context("Waiting for baker response")?;

        let resp: genesis_core::ipc::NightPhaseResponse = serde_json::from_str(&line)
            .context("Parsing baker response")?;

        if resp.status == "success" {
            // Reset state → Idle
            self.set_state(ShmState::Idle);
            Ok(())
        } else {
            self.set_state(ShmState::Idle);
            bail!("Baker daemon returned error: {}", resp.status);
        }
    }

    fn header(&self) -> ShmHeader {
        unsafe { std::ptr::read(self.shm_ptr as *const ShmHeader) }
    }

    fn set_state(&mut self, state: ShmState) {
        // state is at byte offset 5 in ShmHeader
        unsafe {
            self.shm_ptr.add(5).write_volatile(state as u8);
        }
    }
}

impl Drop for BakerClient {
    fn drop(&mut self) {
        if !self.shm_ptr.is_null() {
            unsafe { libc::munmap(self.shm_ptr as *mut _, self.shm_len) };
        }
    }
}
