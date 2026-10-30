//! Shared state between the GUI (writes config, prepares media) and the service
//! (reads config, writes status). Lives in %ProgramData%\aio-screen.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::sensors::{Reading, Stat};

pub const SERVICE_NAME: &str = "AioScreen";
pub const SERVICE_DISPLAY_NAME: &str = "AIO Cooler Screen";

pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("ProgramData").map_or_else(|| PathBuf::from(r"C:\ProgramData"), PathBuf::from);
    base.join("aio-screen")
}

/// Where `install` copies the executable the service runs from (admin-only location).
pub fn install_dir() -> PathBuf {
    let base = std::env::var_os("ProgramFiles").map_or_else(|| PathBuf::from(r"C:\Program Files"), PathBuf::from);
    base.join("AIO Screen")
}

fn config_path() -> PathBuf {
    data_dir().join("config.json")
}

fn status_path() -> PathBuf {
    data_dir().join("status.json")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Position {
    Top,
    Center,
    Bottom,
}

/// The built-in screen designs drawn over the background.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Design {
    Gauges,
    Tiles,
    BigStat,
    Clock,
}

impl Design {
    pub const ALL: [Design; 4] = [Design::Gauges, Design::Tiles, Design::BigStat, Design::Clock];

    pub fn name(self) -> &'static str {
        match self {
            Design::Gauges => "Ring gauges",
            Design::Tiles => "Dashboard tiles",
            Design::BigStat => "Big single stat",
            Design::Clock => "Clock + stats",
        }
    }

    pub fn index(self) -> usize {
        Design::ALL.iter().position(|d| *d == self).unwrap_or(0)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StatsConfig {
    pub enabled: bool,
    pub design: Design,
    pub gauges: GaugesOptions,
    pub tiles: TilesOptions,
    pub big_stat: BigStatOptions,
    pub clock: ClockOptions,
    /// Per-stat reading key chosen by the user; missing means automatic.
    pub sources: BTreeMap<String, String>,
}

impl Default for StatsConfig {
    fn default() -> Self {
        StatsConfig {
            enabled: true,
            design: Design::Gauges,
            gauges: GaugesOptions::default(),
            tiles: TilesOptions::default(),
            big_stat: BigStatOptions::default(),
            clock: ClockOptions::default(),
            sources: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FontChoice {
    SegoeUi,
    Bahnschrift,
    Arial,
    Consolas,
}

impl FontChoice {
    pub const ALL: [FontChoice; 4] = [FontChoice::SegoeUi, FontChoice::Bahnschrift, FontChoice::Arial, FontChoice::Consolas];
}

/// Colours and text styling; every design keeps its own.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Look {
    pub text: [u8; 3],
    /// Colour of gauges, rings and highlights.
    pub accent: [u8; 3],
    /// Colour temperatures from blue (cool) to red (hot) instead of the accent.
    pub temp_colors: bool,
    /// Darken the background by this many percent so stats stay readable.
    pub dim: u8,
    pub font: FontChoice,
    /// Text size in percent of the design's default.
    pub text_scale: u8,
    /// Dark outline around text.
    pub outline: bool,
}

impl Default for Look {
    fn default() -> Self {
        Look {
            text: [255, 255, 255],
            accent: [56, 189, 248],
            temp_colors: true,
            dim: 35,
            font: FontChoice::SegoeUi,
            text_scale: 100,
            outline: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GaugesOptions {
    pub look: Look,
    pub load_rings: bool,
    pub labels: bool,
    /// Temperature ring thickness in pixels.
    pub thickness: u8,
}

impl Default for GaugesOptions {
    fn default() -> Self {
        GaugesOptions { look: Look::default(), load_rings: true, labels: true, thickness: 20 }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TilesOptions {
    pub look: Look,
    pub show: Vec<Stat>,
    pub position: Position,
    pub columns: u8,
    pub panel: bool,
    /// Panel opacity in percent.
    pub panel_opacity: u8,
}

impl Default for TilesOptions {
    fn default() -> Self {
        TilesOptions {
            look: Look { dim: 0, ..Look::default() },
            show: Stat::ALL.to_vec(),
            position: Position::Center,
            columns: 2,
            panel: true,
            panel_opacity: 65,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BigStatOptions {
    pub look: Look,
    pub stat: Stat,
    pub ring: bool,
    pub ticks: bool,
    /// Show the related stat (e.g. load under temperature) underneath.
    pub secondary: bool,
}

impl Default for BigStatOptions {
    fn default() -> Self {
        BigStatOptions { look: Look::default(), stat: Stat::CpuTemp, ring: true, ticks: true, secondary: true }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClockStats {
    Temps,
    Loads,
    None,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ClockOptions {
    pub look: Look,
    pub h24: bool,
    pub seconds_ring: bool,
    pub date: bool,
    pub stats: ClockStats,
}

impl Default for ClockOptions {
    fn default() -> Self {
        ClockOptions { look: Look::default(), h24: true, seconds_ring: true, date: true, stats: ClockStats::Temps }
    }
}

/// Current value of every stat (None when no reading is available).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StatValues(pub [Option<f64>; 4]);

impl StatValues {
    pub fn get(&self, stat: Stat) -> Option<f64> {
        self.0[Stat::ALL.iter().position(|s| *s == stat).unwrap_or(0)]
    }

    /// Plausible values for previews before the service reports real ones.
    pub fn sample() -> Self {
        StatValues([Some(58.0), Some(23.0), Some(47.0), Some(12.0)])
    }
}

impl StatsConfig {
    /// The look of the selected design.
    pub fn look(&self) -> &Look {
        match self.design {
            Design::Gauges => &self.gauges.look,
            Design::Tiles => &self.tiles.look,
            Design::BigStat => &self.big_stat.look,
            Design::Clock => &self.clock.look,
        }
    }

    pub fn look_mut(&mut self) -> &mut Look {
        match self.design {
            Design::Gauges => &mut self.gauges.look,
            Design::Tiles => &mut self.tiles.look,
            Design::BigStat => &mut self.big_stat.look,
            Design::Clock => &mut self.clock.look,
        }
    }

    pub fn source(&self, stat: Stat) -> Option<&String> {
        self.sources.get(&format!("{stat:?}"))
    }

    pub fn set_source(&mut self, stat: Stat, key: Option<String>) {
        let k = format!("{stat:?}");
        match key {
            Some(v) => self.sources.insert(k, v),
            None => self.sources.remove(&k),
        };
    }

    /// The reading that feeds `stat`: the user's choice, or the automatic pick.
    pub fn reading<'a>(&self, stat: Stat, readings: &'a [Reading]) -> Option<&'a Reading> {
        match self.source(stat) {
            Some(key) => readings.iter().find(|r| &r.key == key),
            None => stat.auto_pick(readings),
        }
    }

    pub fn resolve(&self, readings: &[Reading]) -> StatValues {
        StatValues(Stat::ALL.map(|s| self.reading(s, readings).map(|r| r.value)))
    }
}

pub const DEFAULT_LHM_PORT: u16 = 8085;

fn default_lhm_port() -> u16 {
    DEFAULT_LHM_PORT
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// LibreHardwareMonitor web server port.
    #[serde(default = "default_lhm_port")]
    pub lhm_port: u16,
    /// File name inside the data dir of the prepared media (480x480 PNG or MP4).
    pub media_file: Option<String>,
    /// Original file name the user picked, for display.
    pub media_source: Option<String>,
    pub media_is_video: bool,
    /// Bumped by the GUI on every change so the service re-uploads.
    pub media_version: u64,
    pub stats: StatsConfig,
}

impl Config {
    pub fn effective_lhm_port(&self) -> u16 {
        if self.lhm_port == 0 {
            DEFAULT_LHM_PORT
        } else {
            self.lhm_port
        }
    }

    pub fn load() -> Config {
        let Ok(text) = std::fs::read_to_string(config_path()) else { return Config::default() };
        serde_json::from_str(&text)
            .ok()
            .or_else(|| serde_json::from_str(&text).ok().map(migrate_v1))
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        write_atomic(config_path(), &serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?)
    }

    pub fn modified() -> Option<std::time::SystemTime> {
        std::fs::metadata(config_path()).and_then(|m| m.modified()).ok()
    }
}

/// Convert a 1.0 config (one shared look, flat design options) to the current layout,
/// keeping the background and as many settings as map across.
fn migrate_v1(v: serde_json::Value) -> Config {
    use serde_json::from_value as from;
    let get = |o: &serde_json::Value, k: &str| o.get(k).cloned().unwrap_or(serde_json::Value::Null);
    let mut cfg = Config {
        lhm_port: from(get(&v, "lhm_port")).unwrap_or(DEFAULT_LHM_PORT),
        media_file: from(get(&v, "media_file")).unwrap_or_default(),
        media_source: from(get(&v, "media_source")).unwrap_or_default(),
        media_is_video: from(get(&v, "media_is_video")).unwrap_or_default(),
        media_version: from(get(&v, "media_version")).unwrap_or_default(),
        stats: StatsConfig::default(),
    };
    let s = get(&v, "stats");
    let st = &mut cfg.stats;
    st.enabled = from(get(&s, "enabled")).unwrap_or(true);
    st.design = from(get(&s, "design")).unwrap_or(Design::Gauges);
    st.sources = from(get(&s, "sources")).unwrap_or_default();
    for look in [&mut st.gauges.look, &mut st.tiles.look, &mut st.big_stat.look, &mut st.clock.look] {
        look.text = from(get(&s, "color")).unwrap_or(look.text);
        look.accent = from(get(&s, "accent")).unwrap_or(look.accent);
        look.temp_colors = from(get(&s, "temp_colors")).unwrap_or(look.temp_colors);
    }
    for look in [&mut st.gauges.look, &mut st.big_stat.look, &mut st.clock.look] {
        look.dim = from(get(&s, "dim")).unwrap_or(look.dim);
    }
    st.tiles.show = from(get(&s, "show")).unwrap_or(st.tiles.show.clone());
    st.tiles.position = from(get(&s, "position")).unwrap_or(st.tiles.position);
    st.tiles.panel = from(get(&s, "panel")).unwrap_or(st.tiles.panel);
    st.big_stat.stat = from(get(&s, "big_stat")).unwrap_or(st.big_stat.stat);
    st.clock.h24 = from(get(&s, "clock_24h")).unwrap_or(st.clock.h24);
    cfg
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Status {
    pub updated_ms: u128,
    pub device_connected: bool,
    pub sensors: String,
    pub media_version_applied: Option<u64>,
    pub last_error: Option<String>,
    /// Temperature and usage readings, for the GUI's source pickers and preview.
    pub readings: Vec<Reading>,
}

impl Status {
    pub fn load() -> Option<Status> {
        std::fs::read_to_string(status_path()).ok().and_then(|s| serde_json::from_str(&s).ok())
    }

    pub fn save(&self) -> Result<(), String> {
        write_atomic(status_path(), &serde_json::to_vec(self).map_err(|e| e.to_string())?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_v1_config_keeping_background_and_settings() {
        let v1 = r#"{"media_file":"media-1.png","media_source":null,"media_is_video":false,"media_version":9,
            "stats":{"enabled":true,"design":"Tiles","show":["CpuTemp","GpuTemp"],"position":"Bottom",
            "color":[255,0,0],"accent":[56,189,248],"temp_colors":false,"panel":true,"dim":35,
            "big_stat":"GpuLoad","clock_24h":false,"sources":{}}}"#;
        assert!(serde_json::from_str::<Config>(v1).is_err(), "v1 should need migrating");
        let cfg = migrate_v1(serde_json::from_str(v1).unwrap());
        assert_eq!(cfg.media_file.as_deref(), Some("media-1.png"));
        assert_eq!(cfg.media_version, 9);
        assert_eq!(cfg.stats.design, Design::Tiles);
        assert_eq!(cfg.stats.tiles.show, vec![Stat::CpuTemp, Stat::GpuTemp]);
        assert_eq!(cfg.stats.tiles.position, Position::Bottom);
        assert_eq!(cfg.stats.tiles.look.text, [255, 0, 0]);
        assert!(!cfg.stats.gauges.look.temp_colors);
        assert_eq!(cfg.stats.big_stat.stat, Stat::GpuLoad);
        assert!(!cfg.stats.clock.h24);
        assert_eq!(cfg.effective_lhm_port(), DEFAULT_LHM_PORT);
    }
}

fn write_atomic(path: PathBuf, bytes: &[u8]) -> Result<(), String> {
    std::fs::create_dir_all(data_dir()).map_err(|e| format!("cannot create {}: {e}", data_dir().display()))?;
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes).map_err(|e| format!("cannot write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("cannot replace {}: {e}", path.display()))
}
