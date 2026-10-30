//! Wire protocol of the Monster AIO Cooler LCD ("Water Block Screen"),
//! recovered from HID traffic of the official app (v1.0.7).
//!
//! Every HID report is 1024 bytes, prefixed on write by report id 0.
//!
//! Text frame (commands and replies):
//!   0x5A | total_len: u16 BE | payload | checksum: u8 | 0x5A
//!   total_len counts the whole (unescaped) frame including both 0x5A bytes.
//!   checksum = 8-bit sum of total_len bytes + payload bytes.
//!   Between the two 0x5A markers, bytes are escaped: 0x5A -> 5B 01, 0x5B -> 5B 02.
//!   payload  = "POST <cmd> 1\r\nSeqNumber=..\r\nDate=..\r\nContentType=json\r\n
//!               ContentLength=..\r\n\r\n<json>"
//!   replies  = "1 200\r\nAckNumber=..[\r\nContentType=json\r\nContentLength=..]\r\n\r\n<json>"
//!
//! Binary block (file data):
//!   0x5C | len: u16 BE (= 21 + data len) | header[21] | data (<= 1000 bytes)
//!   header = counter: u8 | block_count: u16 BE | block_index: u16 BE | kind: u8 | 15 x 0x00
//!   kind 0 = MP4 video, kind 1 = live overlay (.osd), kind 2 = PNG image.
//!   Videos are sent as plain MP4 files (H.264 High, yuv420p, 480x480, 20 fps, no audio).
//!
//! File upload: "POST transport" {type:"media", fileSize, fileName}
//!   -> reply {state:"success", blockMaxSize:1024}
//!   -> all blocks back to back -> reply "1 200 AckNumber=0"
//!   -> "POST transported" {md5:"todo", fileName} -> reply {state:"success"}
//!
//! Connect handshake (what makes the official app say "device connected"):
//!   "POST conn" (no body) -> reply with device info (firmware, timeout, brightness, ...)
//!   "POST power" {event:"resume"} -> turns the screen back on
//!
//! The device blanks the screen ~30 s after the host stops sending, and stays dark until
//! the handshake is repeated. The official app keeps it on by sending something every
//! second (its overlay, or a "STATE all" stats snapshot); a transparent overlay works too.

pub const VENDOR_ID: u16 = 0x1D6B;
pub const PRODUCT_ID: u16 = 0x0130;
pub const REPORT_LEN: usize = 1024;

const TEXT_MAGIC: u8 = 0x5A;
const ESCAPE: u8 = 0x5B;
const BIN_MAGIC: u8 = 0x5C;
const BLOCK_HEADER_LEN: usize = 21;
pub const BLOCK_DATA_LEN: usize = REPORT_LEN - 3 - BLOCK_HEADER_LEN; // 1000

#[derive(Clone, Copy)]
pub enum MediaKind {
    Video = 0,
    Overlay = 1,
    Image = 2,
}

pub fn text_frame(cmd: &str, seq: u32, date_ms: u128, json: Option<&str>) -> Vec<u8> {
    let mut payload = format!("POST {cmd} 1\r\nSeqNumber={seq}\r\nDate={date_ms}\r\n");
    if let Some(json) = json {
        payload.push_str(&format!("ContentType=json\r\nContentLength={}\r\n", json.len()));
    }
    payload.push_str("\r\n");
    payload.push_str(json.unwrap_or(""));
    frame(payload.as_bytes())
}

/// Wrap a text payload: magic, length, escaped body and checksum.
fn frame(payload: &[u8]) -> Vec<u8> {
    let total = payload.len() + 5;
    let mut inner = Vec::with_capacity(total);
    inner.extend_from_slice(&(total as u16).to_be_bytes());
    inner.extend_from_slice(payload);
    inner.push(inner.iter().fold(0u8, |a, &b| a.wrapping_add(b)));

    let mut f = Vec::with_capacity(total + 8);
    f.push(TEXT_MAGIC);
    for &b in &inner {
        match b {
            TEXT_MAGIC => f.extend_from_slice(&[ESCAPE, 0x01]),
            ESCAPE => f.extend_from_slice(&[ESCAPE, 0x02]),
            _ => f.push(b),
        }
    }
    f.push(TEXT_MAGIC);
    f
}

fn unescape(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut it = bytes.iter();
    while let Some(&b) = it.next() {
        out.push(match (b, it.as_slice().first()) {
            (ESCAPE, Some(0x01)) => TEXT_MAGIC,
            (ESCAPE, Some(0x02)) => ESCAPE,
            (ESCAPE, _) => return None,
            _ => b,
        });
        if b == ESCAPE {
            it.next();
        }
    }
    Some(out)
}

pub fn block_frames(data: &[u8], counter: u8, kind: MediaKind) -> Vec<Vec<u8>> {
    let chunks: Vec<&[u8]> = data.chunks(BLOCK_DATA_LEN).collect();
    let count = chunks.len() as u16;
    chunks
        .iter()
        .enumerate()
        .map(|(i, chunk)| {
            let len = BLOCK_HEADER_LEN + chunk.len();
            let mut f = Vec::with_capacity(3 + len);
            f.push(BIN_MAGIC);
            f.extend_from_slice(&(len as u16).to_be_bytes());
            f.push(counter);
            f.extend_from_slice(&count.to_be_bytes());
            f.extend_from_slice(&(i as u16).to_be_bytes());
            f.push(kind as u8);
            f.extend_from_slice(&[0u8; 15]);
            f.extend_from_slice(chunk);
            f
        })
        .collect()
}

/// A decoded reply frame from the device.
#[derive(Debug)]
pub struct Reply {
    pub status: String,
    pub headers: String,
    pub body: String,
}

pub fn parse_reply(buf: &[u8]) -> Option<Reply> {
    if buf.len() < 5 || buf[0] != TEXT_MAGIC {
        return None;
    }
    // Escaping guarantees the next 0x5A is the closing marker.
    let end = 1 + buf[1..].iter().position(|&b| b == TEXT_MAGIC)?;
    let inner = unescape(&buf[1..end])?;
    let total = u16::from_be_bytes([*inner.first()?, *inner.get(1)?]) as usize;
    if total < 5 || inner.len() != total - 2 {
        return None;
    }
    let (body, cks) = inner.split_at(inner.len() - 1);
    if body.iter().fold(0u8, |a, &b| a.wrapping_add(b)) != cks[0] {
        return None;
    }
    let text = String::from_utf8_lossy(&body[2..]);
    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    let (status, headers) = head.split_once("\r\n").unwrap_or((head, ""));
    Some(Reply {
        status: status.to_string(),
        headers: headers.replace("\r\n", "; "),
        body: body.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Captured from the official app: "POST transported" for an overlay file.
    #[test]
    fn text_frame_matches_capture() {
        let f = text_frame(
            "transported",
            79,
            1793340187740,
            Some(r#"{"md5":"todo","fileName":"2026-10-30_09-03-07-715.osd"}"#),
        );
        assert_eq!(f.len(), 152);
        assert_eq!(&f[..3], &[0x5A, 0x00, 0x98]);
        assert_eq!(&f[f.len() - 2..], &[0x1E, 0x5A]);
        assert!(parse_reply(&f).is_some());
    }

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }

    // Captured: the official app escaping a checksum of 0x5A as 5B 01.
    #[test]
    fn escapes_checksum_like_capture() {
        let f = text_frame(
            "transported",
            299,
            1793340298190,
            Some(r#"{"md5":"todo","fileName":"2026-10-30_09-04-58-173.osd"}"#),
        );
        assert_eq!(&f[f.len() - 3..], &[0x5B, 0x01, 0x5A]);
        assert_eq!(f.len(), 154);
        assert!(parse_reply(&f).is_some());
    }

    // Shaped like the device's reply to "conn" (serial number replaced), with 'Z' and '['
    // in the JSON body, which the device escapes.
    #[test]
    fn parses_escaped_conn_reply() {
        let body = r#"{"OS":"Linux","version":{"firmware":"V1.0.1"},"sn":"XXZX0000TEST0000","timeout":60,"background":["2026-01-15_17-37-01-923.mp4"]}"#;
        let payload = format!("1 200\r\nAckNumber=134\r\nContentType=json\r\nContentLength={}\r\n\r\n{body}", body.len());
        let mut buf = frame(payload.as_bytes());
        assert!(buf.windows(2).any(|w| w == [0x5B, 0x01]) && buf.windows(2).any(|w| w == [0x5B, 0x02]));
        buf.resize(REPORT_LEN, 0);
        let r = parse_reply(&buf).expect("conn reply should parse");
        assert_eq!(r.status, "1 200");
        assert_eq!(r.body, body);
    }

    // Captured: the official app's connect handshake.
    #[test]
    fn handshake_frames_match_capture() {
        assert_eq!(
            hex(&text_frame("conn", 133, 1793343584956, None)),
            "5a0037504f535420636f6e6e20310d0a5365714e756d6265723d3133330d0a446174653d313739333334333538343935360d0a0d0acc5a"
        );
        assert_eq!(
            hex(&text_frame("power", 134, 1793343584980, Some(r#"{"event":"resume"}"#))),
            "5a006e504f535420706f77657220310d0a5365714e756d6265723d3133340d0a446174653d313739333334333538343938300d0a436f6e74656e74547970653d6a736f6e0d0a436f6e74656e744c656e6774683d31380d0a0d0a7b226576656e74223a22726573756d65227d725a"
        );
    }

    #[test]
    fn block_layout_matches_capture() {
        let data = vec![0xAB; 68092];
        let blocks = block_frames(&data, 0x18, MediaKind::Image);
        assert_eq!(blocks.len(), 69);
        assert_eq!(&blocks[0][..9], &[0x5C, 0x03, 0xFD, 0x18, 0x00, 0x45, 0x00, 0x00, 0x02]);
        let last = blocks.last().unwrap();
        assert_eq!(last.len(), 116);
        assert_eq!(&last[..9], &[0x5C, 0x00, 0x71, 0x18, 0x00, 0x45, 0x00, 0x44, 0x02]);
    }

    // Captured: 605500-byte MP4 upload, 606 blocks of kind 0.
    #[test]
    fn video_block_layout_matches_capture() {
        let data = vec![0u8; 605500];
        let blocks = block_frames(&data, 0x1D, MediaKind::Video);
        assert_eq!(blocks.len(), 606);
        assert_eq!(&blocks[1][..9], &[0x5C, 0x03, 0xFD, 0x1D, 0x02, 0x5E, 0x00, 0x01, 0x00]);
    }
}
