//! Windows-style GUI (Slint, Fluent style). Runs without admin: edits the shared config,
//! prepares media into the data dir, and shows what the service reports.

slint::include_modules!();

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use image::imageops::FilterType;
use image::{Rgba, RgbaImage};
use slint::{ComponentHandle, Image, Model, ModelRc, Rgba8Pixel, SharedPixelBuffer, SharedString, Timer, TimerMode, VecModel};
use windows_service::service::ServiceState;

use crate::config::{self, ClockStats, Config, Design, FontChoice, Position, StatValues, Status};
use crate::media::{self, Fit, IMAGE_EXTENSIONS, SCREEN_SIZE, VIDEO_EXTENSIONS};
use crate::sensors::{Reading, Stat};
use crate::{ffmpeg, icon, overlay, service};

const STATUS_STALE_MS: u128 = 5_000;
const SWATCHES: [[u8; 3]; 8] = [
    [255, 255, 255],
    [203, 213, 225],
    [56, 189, 248],
    [74, 222, 128],
    [251, 191, 36],
    [244, 114, 182],
    [244, 63, 94],
    [167, 139, 250],
];
const FITS: [Fit; 3] = [Fit::Cover, Fit::Contain, Fit::Stretch];
const POSITIONS: [Position; 3] = [Position::Top, Position::Center, Position::Bottom];
const CLOCK_STATS: [ClockStats; 3] = [ClockStats::Temps, ClockStats::Loads, ClockStats::None];

enum Job {
    Poster(Result<RgbaImage, String>),
    Media(Result<(String, bool, RgbaImage), String>, String),
    ServiceRestarted(Result<(), String>),
    FfmpegFound(Option<PathBuf>),
    FfmpegDownloaded(Result<(), String>),
}

struct State {
    cfg: Config,
    readings: Vec<Reading>,
    live: bool,
    preview_bg: RgbaImage,
    jobs: (mpsc::Sender<Job>, mpsc::Receiver<Job>),
    last_poll: Instant,
    last_process_check: Instant,
    last_fit: i32,
    rows: Rc<VecModel<StatRow>>,
    row_keys: Vec<Vec<String>>,
    rows_sig: String,
    ffmpeg: Option<PathBuf>,
    ffmpeg_progress_file: Option<PathBuf>,
}

impl State {
    fn spawn(&self, f: impl FnOnce() -> Job + Send + 'static) {
        let tx = self.jobs.0.clone();
        std::thread::spawn(move || {
            let _ = tx.send(f());
        });
    }

    fn values(&self) -> StatValues {
        if self.live {
            self.cfg.stats.resolve(&self.readings)
        } else {
            StatValues::sample()
        }
    }
}

pub fn run() -> Result<(), String> {
    // Skia renders text and shapes with proper anti-aliasing; fall back if it can't start.
    if slint::BackendSelector::new().renderer_name("skia".into()).select().is_err() {
        let _ = slint::BackendSelector::new().select();
    }
    let ui = AppWindow::new().map_err(|e| e.to_string())?;
    let state = Rc::new(RefCell::new(State {
        preview_bg: default_background(),
        cfg: Config::load(),
        readings: Vec::new(),
        live: false,
        jobs: mpsc::channel(),
        last_poll: Instant::now() - Duration::from_secs(10),
        last_process_check: Instant::now() - Duration::from_secs(10),
        last_fit: 0,
        rows: Rc::new(VecModel::default()),
        row_keys: Vec::new(),
        rows_sig: String::new(),
        ffmpeg: None,
        ffmpeg_progress_file: None,
    }));

    init_ui(&ui, &state.borrow());
    {
        let s = state.borrow();
        load_current_poster(&s);
        s.spawn(|| Job::FfmpegFound(ffmpeg::find()));
    }
    wire_callbacks(&ui, &state);

    let timer = Timer::default();
    let (weak, st) = (ui.as_weak(), state.clone());
    timer.start(TimerMode::Repeated, Duration::from_millis(250), move || {
        if let Some(ui) = weak.upgrade() {
            tick(&ui, &st);
        }
    });
    ui.run().map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------------------
// Images

fn to_image(img: &RgbaImage) -> Image {
    Image::from_rgba8(SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(img.as_raw(), img.width(), img.height()))
}

/// Shown when no background media is set: a deep blue gradient.
fn default_background() -> RgbaImage {
    RgbaImage::from_fn(SCREEN_SIZE, SCREEN_SIZE, |x, y| {
        let t = (x + y) as f32 / (2 * SCREEN_SIZE) as f32;
        let mix = |a: f32, b: f32| (a + (b - a) * t) as u8;
        Rgba([mix(30.0, 8.0), mix(41.0, 12.0), mix(75.0, 28.0), 255])
    })
}

fn compose(bg: &RgbaImage, cfg: &config::StatsConfig, values: &StatValues) -> RgbaImage {
    let mut img = bg.clone();
    image::imageops::overlay(&mut img, &overlay::render(cfg, values, chrono::Local::now()), 0, 0);
    img
}

fn design_thumbnails(state: &State) -> Vec<Image> {
    let bg = default_background();
    Design::ALL
        .iter()
        .map(|&design| {
            let mut stats = state.cfg.stats.clone();
            stats.enabled = true;
            stats.design = design;
            let img = compose(&bg, &stats, &StatValues::sample());
            to_image(&image::imageops::resize(&img, 224, 224, FilterType::Lanczos3))
        })
        .collect()
}

fn refresh_preview(ui: &AppWindow, state: &State) {
    ui.set_preview(to_image(&compose(&state.preview_bg, &state.cfg.stats, &state.values())));
    ui.set_preview_live(state.live);
}

fn load_current_poster(state: &State) {
    let Some(path) = state.cfg.media_file.as_ref().map(|f| config::data_dir().join(f)).filter(|p| p.exists()) else {
        return;
    };
    let is_video = state.cfg.media_is_video;
    state.spawn(move || {
        Job::Poster(if is_video { media::video_poster(&path, Fit::Cover) } else { media::prepare_image(&path, Fit::Cover) })
    });
}

fn poster_for(path: &Path, fit: Fit) -> Result<RgbaImage, String> {
    if media::is_video(path) {
        media::video_poster(path, fit)
    } else {
        media::prepare_image(path, fit)
    }
}

// ---------------------------------------------------------------------------------------
// Config <-> UI

fn swatch_index(rgb: [u8; 3]) -> i32 {
    SWATCHES.iter().position(|s| *s == rgb).map_or(-1, |i| i as i32)
}

fn swatch(index: i32) -> Option<[u8; 3]> {
    usize::try_from(index).ok().and_then(|i| SWATCHES.get(i)).copied()
}

fn index_of<T: PartialEq>(all: &[T], v: &T) -> i32 {
    all.iter().position(|x| x == v).unwrap_or(0) as i32
}

fn media_label(cfg: &Config) -> String {
    match (&cfg.media_source, &cfg.media_file) {
        (Some(src), _) => format!("{src}{}", if cfg.media_is_video { " (video)" } else { "" }),
        (None, Some(f)) => f.clone(),
        (None, None) => "none".into(),
    }
}

fn init_ui(ui: &AppWindow, state: &State) {
    ui.set_app_icon(to_image(&icon::render(64)));
    ui.set_version(env!("CARGO_PKG_VERSION").into());
    let names: Vec<SharedString> = Design::ALL.iter().map(|d| d.name().into()).collect();
    ui.set_design_names(ModelRc::new(VecModel::from(names)));
    ui.set_design_thumbs(ModelRc::new(VecModel::from(design_thumbnails(state))));
    let swatches: Vec<slint::Color> = SWATCHES.iter().map(|c| slint::Color::from_rgb_u8(c[0], c[1], c[2])).collect();
    ui.set_swatches(ModelRc::new(VecModel::from(swatches)));
    ui.set_stat_rows(ModelRc::from(state.rows.clone()));
    ui.set_media_name(media_label(&state.cfg).into());
    ui.set_lhm_port(state.cfg.effective_lhm_port().to_string().into());
    ui.set_ffmpeg_state("Checking…".into());
    load_design_into_ui(ui, &state.cfg);
    refresh_preview(ui, state);
}

/// Show the selected design's look and every design's options in the controls.
fn load_design_into_ui(ui: &AppWindow, cfg: &Config) {
    let s = &cfg.stats;
    ui.set_design(s.design.index() as i32);
    ui.set_stats_enabled(s.enabled);

    let look = s.look();
    ui.set_text_color_index(swatch_index(look.text));
    ui.set_accent_index(swatch_index(look.accent));
    ui.set_temp_colors(look.temp_colors);
    ui.set_dim(look.dim as f32);
    ui.set_font_index(index_of(&FontChoice::ALL, &look.font));
    ui.set_text_scale(look.text_scale as f32);
    ui.set_outline(look.outline);

    let g = &s.gauges;
    ui.set_g_load_rings(g.load_rings);
    ui.set_g_labels(g.labels);
    ui.set_g_thickness(g.thickness as f32);

    let t = &s.tiles;
    ui.set_t_cpu_temp(t.show.contains(&Stat::CpuTemp));
    ui.set_t_cpu_load(t.show.contains(&Stat::CpuLoad));
    ui.set_t_gpu_temp(t.show.contains(&Stat::GpuTemp));
    ui.set_t_gpu_load(t.show.contains(&Stat::GpuLoad));
    ui.set_t_columns(if t.columns >= 2 { 1 } else { 0 });
    ui.set_t_position(index_of(&POSITIONS, &t.position));
    ui.set_t_panel(t.panel);
    ui.set_t_panel_opacity(t.panel_opacity as f32);

    let b = &s.big_stat;
    ui.set_b_stat(index_of(&Stat::ALL, &b.stat));
    ui.set_b_ring(b.ring);
    ui.set_b_ticks(b.ticks);
    ui.set_b_secondary(b.secondary);

    let c = &s.clock;
    ui.set_c_h24(c.h24);
    ui.set_c_seconds(c.seconds_ring);
    ui.set_c_date(c.date);
    ui.set_c_stats(index_of(&CLOCK_STATS, &c.stats));
}

/// Copy the controls back into the config (the look goes to the selected design).
fn read_settings(ui: &AppWindow, cfg: &mut Config) {
    let s = &mut cfg.stats;
    s.enabled = ui.get_stats_enabled();

    let look = s.look_mut();
    if let Some(c) = swatch(ui.get_text_color_index()) {
        look.text = c;
    }
    if let Some(c) = swatch(ui.get_accent_index()) {
        look.accent = c;
    }
    look.temp_colors = ui.get_temp_colors();
    look.dim = ui.get_dim().round().clamp(0.0, 80.0) as u8;
    look.font = FontChoice::ALL[ui.get_font_index().clamp(0, 3) as usize];
    look.text_scale = ui.get_text_scale().round().clamp(70.0, 130.0) as u8;
    look.outline = ui.get_outline();

    s.gauges.load_rings = ui.get_g_load_rings();
    s.gauges.labels = ui.get_g_labels();
    s.gauges.thickness = ui.get_g_thickness().round().clamp(8.0, 36.0) as u8;

    let flags = [ui.get_t_cpu_temp(), ui.get_t_cpu_load(), ui.get_t_gpu_temp(), ui.get_t_gpu_load()];
    s.tiles.show = Stat::ALL.into_iter().zip(flags).filter(|(_, on)| *on).map(|(st, _)| st).collect();
    s.tiles.columns = if ui.get_t_columns() == 1 { 2 } else { 1 };
    s.tiles.position = POSITIONS[ui.get_t_position().clamp(0, 2) as usize];
    s.tiles.panel = ui.get_t_panel();
    s.tiles.panel_opacity = ui.get_t_panel_opacity().round().clamp(10.0, 100.0) as u8;

    s.big_stat.stat = Stat::ALL[ui.get_b_stat().clamp(0, 3) as usize];
    s.big_stat.ring = ui.get_b_ring();
    s.big_stat.ticks = ui.get_b_ticks();
    s.big_stat.secondary = ui.get_b_secondary();

    s.clock.h24 = ui.get_c_h24();
    s.clock.seconds_ring = ui.get_c_seconds();
    s.clock.date = ui.get_c_date();
    s.clock.stats = CLOCK_STATS[ui.get_c_stats().clamp(0, 2) as usize];
}

fn show_message(ui: &AppWindow, ok: bool, msg: impl Into<SharedString>) {
    ui.set_message(msg.into());
    ui.set_message_ok(ok);
}

fn save(ui: &AppWindow, state: &State) {
    if let Err(e) = state.cfg.save() {
        show_message(ui, false, format!("Could not save settings: {e}"));
    }
}

fn update_ffmpeg_ui(ui: &AppWindow, state: &State) {
    ui.set_ffmpeg_ok(state.ffmpeg.is_some());
    ui.set_ffmpeg_state(match &state.ffmpeg {
        Some(p) if p.as_os_str() == "ffmpeg" => "Found on PATH".into(),
        Some(p) => format!("Found: {}", p.display()).into(),
        None => "Not installed".into(),
    });
}

// ---------------------------------------------------------------------------------------
// Callbacks

fn wire_callbacks(ui: &AppWindow, state: &Rc<RefCell<State>>) {
    let weak = ui.as_weak();
    let st = state.clone();
    ui.on_settings_changed(move || {
        let Some(ui) = weak.upgrade() else { return };
        let mut s = st.borrow_mut();
        read_settings(&ui, &mut s.cfg);
        save(&ui, &s);
        refresh_preview(&ui, &s);
        ui.set_design_thumbs(ModelRc::new(VecModel::from(design_thumbnails(&s))));
    });

    let weak = ui.as_weak();
    let st = state.clone();
    ui.on_design_selected(move |i| {
        let Some(ui) = weak.upgrade() else { return };
        let mut s = st.borrow_mut();
        s.cfg.stats.design = Design::ALL[i.clamp(0, 3) as usize];
        s.cfg.stats.enabled = true;
        save(&ui, &s);
        load_design_into_ui(&ui, &s.cfg);
        refresh_preview(&ui, &s);
    });

    let weak = ui.as_weak();
    let st = state.clone();
    ui.on_browse(move || {
        let Some(ui) = weak.upgrade() else { return };
        let exts: Vec<&str> = IMAGE_EXTENSIONS.iter().chain(VIDEO_EXTENSIONS).copied().collect();
        let Some(p) = rfd::FileDialog::new().add_filter("Images and videos", &exts).pick_file() else { return };
        ui.set_source_path(p.display().to_string().into());
        ui.set_source_is_video(media::is_video(&p));
        let s = st.borrow();
        if media::is_video(&p) && s.ffmpeg.is_none() {
            return; // the Background card offers the ffmpeg download
        }
        let fit = FITS[ui.get_fit().clamp(0, 2) as usize];
        s.spawn(move || Job::Poster(poster_for(&p, fit)));
    });

    let weak = ui.as_weak();
    let st = state.clone();
    ui.on_apply_media(move || {
        let Some(ui) = weak.upgrade() else { return };
        let src = PathBuf::from(ui.get_source_path().trim());
        if !src.exists() {
            show_message(&ui, false, format!("File not found: {}", src.display()));
            return;
        }
        let fit = FITS[ui.get_fit().clamp(0, 2) as usize];
        ui.set_busy(true);
        ui.set_busy_text(if media::is_video(&src) { "Converting video…" } else { "Preparing image…" }.into());
        ui.set_message(SharedString::new());
        let name = src.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        st.borrow().spawn(move || Job::Media(prepare_into_data_dir(&src, fit), name));
    });

    let weak = ui.as_weak();
    let st = state.clone();
    ui.on_source_selected(move |i, idx| {
        let Some(ui) = weak.upgrade() else { return };
        let mut s = st.borrow_mut();
        let i = i.clamp(0, 3) as usize;
        let key = usize::try_from(idx - 1).ok().and_then(|k| s.row_keys.get(i)?.get(k).cloned());
        s.cfg.stats.set_source(Stat::ALL[i], key);
        save(&ui, &s);
        s.rows_sig.clear();
        refresh_preview(&ui, &s);
    });

    let weak = ui.as_weak();
    let st = state.clone();
    ui.on_repair_service(move || {
        let Some(ui) = weak.upgrade() else { return };
        ui.set_busy(true);
        ui.set_busy_text("Restarting service…".into());
        show_message(&ui, true, "Approve the admin prompt to restart the background service.");
        st.borrow().spawn(|| Job::ServiceRestarted(elevate("install", &result_file("install"))));
    });

    let weak = ui.as_weak();
    let st = state.clone();
    ui.on_download_ffmpeg(move || {
        let Some(ui) = weak.upgrade() else { return };
        let mut s = st.borrow_mut();
        let file = result_file("fetch-ffmpeg");
        s.ffmpeg_progress_file = Some(PathBuf::from(format!("{}.progress", file.display())));
        ui.set_ffmpeg_busy(true);
        ui.set_ffmpeg_progress(0.0);
        ui.set_ffmpeg_state("Downloading… (approve the admin prompt)".into());
        s.spawn(move || Job::FfmpegDownloaded(elevate("fetch-ffmpeg", &file)));
    });

    let weak = ui.as_weak();
    let st = state.clone();
    ui.on_lhm_port_edited(move |text| {
        let Some(ui) = weak.upgrade() else { return };
        if let Ok(port) = text.trim().parse::<u16>() {
            let mut s = st.borrow_mut();
            s.cfg.lhm_port = port;
            save(&ui, &s);
        }
    });

    ui.on_open_data_folder(|| {
        let _ = Command::new("explorer.exe").arg(config::data_dir()).spawn();
    });
    ui.on_open_log(|| {
        let _ = Command::new("notepad.exe").arg(config::data_dir().join("service.log")).spawn();
    });
    ui.on_open_url(|url| open_url(&url));
}

// ---------------------------------------------------------------------------------------
// Periodic updates

fn tick(ui: &AppWindow, state: &Rc<RefCell<State>>) {
    let mut s = state.borrow_mut();

    while let Ok(job) = s.jobs.1.try_recv() {
        match job {
            Job::Poster(Ok(img)) => {
                s.preview_bg = img;
                refresh_preview(ui, &s);
            }
            Job::Poster(Err(e)) => show_message(ui, false, e),
            Job::Media(result, source_name) => {
                ui.set_busy(false);
                match result {
                    Ok((file, is_video, poster)) => {
                        let old = s.cfg.media_file.replace(file);
                        s.cfg.media_is_video = is_video;
                        s.cfg.media_source = Some(source_name);
                        s.cfg.media_version += 1;
                        match s.cfg.save() {
                            Ok(()) => {
                                if let Some(old) = old.filter(|o| Some(o) != s.cfg.media_file.as_ref()) {
                                    let _ = std::fs::remove_file(config::data_dir().join(old));
                                }
                                show_message(ui, true, "Sent to the cooler. It updates within a few seconds.");
                            }
                            Err(e) => show_message(ui, false, format!("Could not save settings: {e}")),
                        }
                        ui.set_media_name(media_label(&s.cfg).into());
                        s.preview_bg = poster;
                        refresh_preview(ui, &s);
                    }
                    Err(e) => show_message(ui, false, e),
                }
            }
            Job::ServiceRestarted(r) => {
                ui.set_busy(false);
                match r {
                    Ok(()) => show_message(ui, true, "Background service restarted."),
                    Err(e) => show_message(ui, false, e),
                }
                s.last_poll = Instant::now() - Duration::from_secs(10);
            }
            Job::FfmpegFound(path) => {
                s.ffmpeg = path;
                update_ffmpeg_ui(ui, &s);
            }
            Job::FfmpegDownloaded(r) => {
                ui.set_ffmpeg_busy(false);
                s.ffmpeg_progress_file = None;
                if let Err(e) = r {
                    show_message(ui, false, format!("ffmpeg download failed: {e}"));
                }
                s.spawn(|| Job::FfmpegFound(ffmpeg::find()));
            }
        }
    }

    if let Some(p) = &s.ffmpeg_progress_file {
        if let Ok(text) = std::fs::read_to_string(p) {
            let mut it = text.split_whitespace().filter_map(|x| x.parse::<f64>().ok());
            if let (Some(done), Some(total)) = (it.next(), it.next()) {
                if total > 0.0 {
                    ui.set_ffmpeg_progress((done / total) as f32);
                    ui.set_ffmpeg_state(format!("Downloading… {:.0} / {:.0} MB", done / 1e6, total / 1e6).into());
                }
            }
        }
    }

    // Re-render the preview background when the fit changes for a chosen file.
    if ui.get_fit() != s.last_fit {
        s.last_fit = ui.get_fit();
        let src = PathBuf::from(ui.get_source_path().trim());
        if src.exists() && (!media::is_video(&src) || s.ffmpeg.is_some()) {
            let fit = FITS[s.last_fit.clamp(0, 2) as usize];
            s.spawn(move || Job::Poster(poster_for(&src, fit)));
        }
    }

    if s.last_poll.elapsed() < Duration::from_secs(1) {
        return;
    }
    s.last_poll = Instant::now();

    let status = Status::load().filter(|st| media::now_ms().saturating_sub(st.updated_ms) < STATUS_STALE_MS);
    let state_now = service::query_state();
    s.live = status.as_ref().is_some_and(|st| !st.readings.is_empty());
    s.readings = status.as_ref().map(|st| st.readings.clone()).unwrap_or_default();

    let (svc_text, running) = match state_now {
        None => ("Not installed", false),
        Some(ServiceState::Running) => ("Running", true),
        Some(ServiceState::Stopped) => ("Stopped", false),
        Some(ServiceState::StartPending) => ("Starting…", false),
        Some(ServiceState::StopPending) => ("Stopping…", false),
        Some(_) => ("Busy", false),
    };
    ui.set_service_state(svc_text.into());
    ui.set_service_running(running);
    match &status {
        Some(st) => {
            ui.set_cooler_ok(st.device_connected);
            ui.set_cooler_state(if st.device_connected { "Cooler connected" } else { "Cooler not found" }.into());
            ui.set_sensors_state(st.sensors.clone().into());
            ui.set_sensors_ok(!st.readings.is_empty());
            ui.set_last_error(st.last_error.clone().unwrap_or_default().into());
        }
        None => {
            ui.set_cooler_ok(false);
            ui.set_cooler_state(if running { "Waiting for the service…" } else { "Service not running" }.into());
            ui.set_sensors_state("No data from the service yet.".into());
            ui.set_sensors_ok(true);
            ui.set_last_error(SharedString::new());
        }
    }

    if s.last_process_check.elapsed() >= Duration::from_secs(3) {
        s.last_process_check = Instant::now();
        ui.set_official_running(official_app_running());
    }

    update_stat_rows(&mut s);
    refresh_preview(ui, &s);
}

fn fmt_value(stat: Stat, v: f64) -> String {
    if stat.is_temp() {
        format!("{v:.0}°")
    } else {
        format!("{v:.0}%")
    }
}

/// Rebuild the source pickers only when the available readings change (so an open
/// drop-down isn't reset every second); otherwise just refresh the values.
fn update_stat_rows(s: &mut State) {
    let mut keys = Vec::new();
    let mut sig = String::new();
    for stat in Stat::ALL {
        let k: Vec<String> = s.readings.iter().filter(|r| r.kind == stat.reading_kind()).map(|r| r.key.clone()).collect();
        sig.push_str(&k.join("|"));
        sig.push_str(s.cfg.stats.source(stat).map_or("", |x| x));
        sig.push(';');
        keys.push(k);
    }
    let values = s.values();
    let rebuild = sig != s.rows_sig || s.rows.row_count() != Stat::ALL.len();
    for (i, stat) in Stat::ALL.into_iter().enumerate() {
        let value: SharedString = values.get(stat).map(|v| fmt_value(stat, v)).unwrap_or_else(|| "--".into()).into();
        if !rebuild {
            if let Some(mut row) = s.rows.row_data(i) {
                row.value = value;
                s.rows.set_row_data(i, row);
            }
            continue;
        }
        let auto = stat.auto_pick(&s.readings).map_or_else(|| "Auto".to_string(), |r| format!("Auto ({})", r.key));
        let mut choices: Vec<SharedString> = vec![auto.into()];
        choices.extend(keys[i].iter().map(|k| SharedString::from(k.as_str())));
        let index = s.cfg.stats.source(stat).and_then(|src| keys[i].iter().position(|k| k == src)).map_or(0, |p| p as i32 + 1);
        let row = StatRow { title: stat.title().into(), choices: ModelRc::new(VecModel::from(choices)), index, value };
        if i < s.rows.row_count() {
            s.rows.set_row_data(i, row);
        } else {
            s.rows.push(row);
        }
    }
    if rebuild {
        s.row_keys = keys;
        s.rows_sig = sig;
    }
}

// ---------------------------------------------------------------------------------------
// Helpers

/// Convert the user's file and store it in the data dir. Returns (file name, is_video, poster).
fn prepare_into_data_dir(src: &Path, fit: Fit) -> Result<(String, bool, RgbaImage), String> {
    let dir = config::data_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    let stamp = media::now_ms();
    if media::is_video(src) {
        let name = format!("media-{stamp}.mp4");
        media::prepare_video(src, fit, &dir.join(&name))?;
        let poster = media::video_poster(&dir.join(&name), Fit::Cover)?;
        Ok((name, true, poster))
    } else {
        let img = media::prepare_image(src, fit)?;
        let name = format!("media-{stamp}.png");
        std::fs::write(dir.join(&name), media::encode_png(&img)?).map_err(|e| format!("cannot save media: {e}"))?;
        Ok((name, false, img))
    }
}

fn wide(s: &std::ffi::OsStr) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    s.encode_wide().chain(Some(0)).collect()
}

fn open_url(url: &str) {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let (verb, file) = (wide("open".as_ref()), wide(url.as_ref()));
    // SAFETY: NUL-terminated wide strings that outlive the call.
    unsafe {
        ShellExecuteW(std::ptr::null_mut(), verb.as_ptr(), file.as_ptr(), std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL);
    }
}

fn official_app_running() -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
    };
    // SAFETY: standard process snapshot walk; the entry struct is sized before use.
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE {
            return false;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut found = false;
        let mut ok = Process32FirstW(snap, &mut entry) != 0;
        while ok {
            let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
            if String::from_utf16_lossy(&entry.szExeFile[..len]).eq_ignore_ascii_case("Monster AIO Cooler.exe") {
                found = true;
                break;
            }
            ok = Process32NextW(snap, &mut entry) != 0;
        }
        CloseHandle(snap);
        found
    }
}

fn result_file(what: &str) -> PathBuf {
    std::env::temp_dir().join(format!("aio-screen-{what}-{}.txt", media::now_ms()))
}

/// Run `aio-screen.exe <what> <result-file>` elevated and wait for it.
fn elevate(what: &str, result_file: &Path) -> Result<(), String> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject, INFINITE};
    use windows_sys::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;

    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let verb = wide("runas".as_ref());
    let file = wide(exe.as_os_str());
    let params = wide(format!("{what} \"{}\"", result_file.display()).as_ref());

    // SAFETY: plain Win32 calls with NUL-terminated wide strings that outlive the call.
    let result = unsafe {
        let mut info: SHELLEXECUTEINFOW = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
        info.fMask = SEE_MASK_NOCLOSEPROCESS;
        info.lpVerb = verb.as_ptr();
        info.lpFile = file.as_ptr();
        info.lpParameters = params.as_ptr();
        info.nShow = SW_HIDE;
        if ShellExecuteExW(&mut info) == 0 || info.hProcess.is_null() {
            return Err("The admin prompt was cancelled.".into());
        }
        WaitForSingleObject(info.hProcess, INFINITE);
        let mut code = 1u32;
        GetExitCodeProcess(info.hProcess, &mut code);
        CloseHandle(info.hProcess);
        let detail = std::fs::read_to_string(result_file).unwrap_or_default();
        if code == 0 {
            Ok(())
        } else {
            Err(if detail.trim().is_empty() { format!("{what} failed (exit code {code})") } else { detail.trim().to_string() })
        }
    };
    let _ = std::fs::remove_file(result_file);
    let _ = std::fs::remove_file(format!("{}.progress", result_file.display()));
    result
}
