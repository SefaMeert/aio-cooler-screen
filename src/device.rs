//! The cooler screen over USB HID: command/reply exchange and file uploads.

use std::time::{Duration, Instant};

use hidapi::{HidApi, HidDevice};

use crate::media::now_ms;
use crate::protocol::{self, MediaKind, Reply, PRODUCT_ID, REPORT_LEN, VENDOR_ID};

pub struct Screen {
    dev: HidDevice,
    seq: u32,
    counter: u8,
}

impl Screen {
    pub fn open(api: &HidApi) -> Result<Self, String> {
        let info = api
            .device_list()
            .find(|d| d.vendor_id() == VENDOR_ID && d.product_id() == PRODUCT_ID)
            .ok_or("cooler screen not found (USB HID 1D6B:0130)")?;
        let dev = api.open_path(info.path()).map_err(|e| format!("failed to open device: {e}"))?;
        let now = now_ms();
        let mut s = Screen { dev, seq: (now % 10_000) as u32, counter: (now % 256) as u8 };
        s.drain();
        s.handshake()?;
        Ok(s)
    }

    /// The official app's connect sequence. Wakes the screen if it went dark after the
    /// host stopped sending.
    pub fn handshake(&mut self) -> Result<(), String> {
        self.command("conn", None, Duration::from_secs(2), |_| true)?;
        self.command("power", Some(r#"{"event":"resume"}"#), Duration::from_secs(2), |_| true)?;
        Ok(())
    }

    fn drain(&self) {
        let mut buf = [0u8; REPORT_LEN];
        while matches!(self.dev.read_timeout(&mut buf, 20), Ok(n) if n > 0) {}
    }

    fn write_frame(&self, frame: &[u8]) -> Result<(), String> {
        let mut report = Vec::with_capacity(frame.len() + 1);
        report.push(0); // report id
        report.extend_from_slice(frame);
        self.dev.write(&report).map_err(|e| format!("HID write failed: {e}"))?;
        Ok(())
    }

    /// Wait for a reply that satisfies `want`, skipping anything else.
    fn wait_reply(&self, timeout: Duration, what: &str, want: impl Fn(&Reply) -> bool) -> Result<Reply, String> {
        let deadline = Instant::now() + timeout;
        let mut buf = [0u8; REPORT_LEN];
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(format!("timed out waiting for {what}"));
            }
            let n = self
                .dev
                .read_timeout(&mut buf, left.as_millis().max(1) as i32)
                .map_err(|e| format!("HID read failed: {e}"))?;
            let Some(reply) = protocol::parse_reply(&buf[..n]) else { continue };
            if !reply.status.ends_with(" 200") {
                return Err(format!("device rejected {what}: {} {}", reply.status, reply.body));
            }
            if want(&reply) {
                return Ok(reply);
            }
        }
    }

    fn command(&mut self, cmd: &str, json: Option<&str>, timeout: Duration, want: impl Fn(&Reply) -> bool) -> Result<Reply, String> {
        self.seq = self.seq.wrapping_add(1);
        self.write_frame(&protocol::text_frame(cmd, self.seq, now_ms(), json))?;
        self.wait_reply(timeout, cmd, want)
    }

    pub fn upload(&mut self, data: &[u8], file_name: &str, kind: MediaKind) -> Result<(), String> {
        let json = format!(r#"{{"type":"media","fileSize":{},"fileName":"{file_name}"}}"#, data.len());
        let r = self.command("transport", Some(&json), Duration::from_secs(2), |r| r.body.contains("blockMaxSize"))?;
        if !r.body.contains("success") {
            return Err(format!("device refused upload: {}", r.body));
        }

        self.counter = self.counter.wrapping_add(1);
        for block in protocol::block_frames(data, self.counter, kind) {
            self.write_frame(&block)?;
        }
        let ack_timeout = Duration::from_secs(5 + (data.len() / 1_000_000) as u64);
        self.wait_reply(ack_timeout, "block acknowledgement", |r| r.headers.contains("AckNumber=0"))?;

        let json = format!(r#"{{"md5":"todo","fileName":"{file_name}"}}"#);
        let r = self.command("transported", Some(&json), Duration::from_secs(10), |r| r.body.contains("state"))?;
        if !r.body.contains("success") {
            return Err(format!("device did not accept the file: {}", r.body));
        }
        Ok(())
    }
}
