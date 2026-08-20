use std::{
    collections::{BTreeMap, VecDeque},
    path::PathBuf,
    sync::{
        LazyLock, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use plotly::{Configuration, Layout, Plot, Trace, common::Title, layout::Axis};
use serde::Serialize;

static ACTIVE: AtomicBool = AtomicBool::new(false);
static NEXT_FRAME_ID: AtomicU64 = AtomicU64::new(1);
static PROFILE: LazyLock<Mutex<ProfileData>> = LazyLock::new(|| Mutex::new(ProfileData::default()));

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    BufferProduced,
    TextureBuildStarted,
    TextureBuilt,
    Snapshot,
}

impl Stage {
    fn index(self) -> usize {
        match self {
            Self::BufferProduced => 0,
            Self::TextureBuildStarted => 1,
            Self::TextureBuilt => 2,
            Self::Snapshot => 3,
        }
    }
}

struct Sample {
    frame_id: u64,
    elapsed_ms: f64,
    unix_timestamp_us: u128,
    stage: Stage,
}

struct FpsSample {
    elapsed_ms: f64,
    fps: f64,
}

#[derive(Default)]
struct ProfileData {
    started_at: Option<Instant>,
    samples: Vec<Sample>,
    fps_samples: Vec<FpsSample>,
    recent_buffers: VecDeque<Instant>,
}

impl ProfileData {
    fn reset(&mut self) {
        *self = Self {
            started_at: Some(Instant::now()),
            ..Self::default()
        };
    }

    fn timestamp(&self, now: Instant) -> f64 {
        self.started_at.map_or(0.0, |started_at| {
            now.duration_since(started_at).as_secs_f64() * 1_000.0
        })
    }

    fn record(&mut self, frame_id: u64, stage: Stage) {
        let now = Instant::now();
        let elapsed_ms = self.timestamp(now);
        let unix_timestamp_us = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_micros());

        self.samples.push(Sample {
            frame_id,
            elapsed_ms,
            unix_timestamp_us,
            stage,
        });

        if stage == Stage::BufferProduced {
            self.recent_buffers.push_back(now);
            while self
                .recent_buffers
                .front()
                .is_some_and(|first| now.duration_since(*first).as_secs_f64() > 1.0)
            {
                self.recent_buffers.pop_front();
            }

            let fps = match (self.recent_buffers.front(), self.recent_buffers.back()) {
                (Some(first), Some(last)) if self.recent_buffers.len() > 1 => {
                    (self.recent_buffers.len() - 1) as f64
                        / last.duration_since(*first).as_secs_f64()
                }
                _ => 0.0,
            };
            self.fps_samples.push(FpsSample { elapsed_ms, fps });
        }
    }
}

/// Flushes the proxy profiling report when dropped.
///
/// Keep this guard alive for the duration of the application. The report path defaults to
/// `mutsumi-proxy-profile.html` and can be overridden with `MUTSUMI_PROFILE_PATH`.
#[must_use = "keep the guard alive until profiling should be written"]
pub struct ProxyProfilingGuard {
    output_path: PathBuf,
}

impl Drop for ProxyProfilingGuard {
    fn drop(&mut self) {
        ACTIVE.store(false, Ordering::Release);
        if let Err(error) = write_report(&self.output_path) {
            tracing::error!("failed to write proxy profiling report: {error}");
        } else {
            tracing::info!(path = %self.output_path.display(), "wrote proxy profiling report");
        }
    }
}

/// Starts collecting proxy frame timing data.
///
/// # Panics
///
/// Panics if another proxy profiling guard is still active.
pub fn start_proxy_profiling() -> ProxyProfilingGuard {
    assert!(
        !ACTIVE.swap(true, Ordering::AcqRel),
        "proxy profiling is already active"
    );
    NEXT_FRAME_ID.store(1, Ordering::Relaxed);
    PROFILE.lock().unwrap().reset();

    ProxyProfilingGuard {
        output_path: std::env::var_os("MUTSUMI_PROFILE_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("mutsumi-proxy-profile.html")),
    }
}

pub fn begin_frame() -> Option<u64> {
    if !ACTIVE.load(Ordering::Acquire) {
        return None;
    }

    let frame_id = NEXT_FRAME_ID.fetch_add(1, Ordering::Relaxed);
    PROFILE
        .lock()
        .unwrap()
        .record(frame_id, Stage::BufferProduced);
    Some(frame_id)
}

pub fn mark(frame_id: Option<u64>, stage: Stage) {
    if let Some(frame_id) = frame_id.filter(|_| ACTIVE.load(Ordering::Acquire)) {
        PROFILE.lock().unwrap().record(frame_id, stage);
    }
}

struct FrameTiming {
    frame_id: u64,
    stages: [Option<f64>; 4],
    unix_timestamps_us: [Option<u128>; 4],
}

impl FrameTiming {
    fn new(frame_id: u64) -> Self {
        Self {
            frame_id,
            stages: [None; 4],
            unix_timestamps_us: [None; 4],
        }
    }

    fn record(&mut self, sample: &Sample) {
        let index = sample.stage.index();
        self.stages[index] = Some(sample.elapsed_ms);
        self.unix_timestamps_us[index] = Some(sample.unix_timestamp_us);
    }
}

#[derive(Serialize)]
struct CanvasFrame {
    #[serde(rename = "i")]
    frame_id: u64,
    #[serde(rename = "b")]
    buffer_elapsed_ms: f64,
    #[serde(rename = "u")]
    buffer_unix_timestamp_us: u128,
    #[serde(rename = "t")]
    relative_stages_ms: [Option<f64>; 4],
}

fn canvas_frames(data: &ProfileData) -> Vec<CanvasFrame> {
    let mut frames = BTreeMap::new();
    for sample in &data.samples {
        frames
            .entry(sample.frame_id)
            .or_insert_with(|| FrameTiming::new(sample.frame_id))
            .record(sample);
    }

    frames
        .into_values()
        .filter_map(|frame| {
            let buffer_elapsed_ms = frame.stages[Stage::BufferProduced.index()]?;
            let relative_stages_ms = frame
                .stages
                .map(|timestamp| timestamp.map(|timestamp| timestamp - buffer_elapsed_ms));
            Some(CanvasFrame {
                frame_id: frame.frame_id,
                buffer_elapsed_ms,
                buffer_unix_timestamp_us: frame.unix_timestamps_us[Stage::BufferProduced.index()]
                    .unwrap_or(0),
                relative_stages_ms,
            })
        })
        .collect()
}

#[derive(Clone, Serialize)]
struct WebGlLine {
    color: &'static str,
    width: f64,
}

#[derive(Clone, Serialize)]
struct FpsTrace {
    #[serde(rename = "type")]
    type_: &'static str,
    x: Vec<f64>,
    y: Vec<f64>,
    name: &'static str,
    mode: &'static str,
    line: WebGlLine,
}

impl Trace for FpsTrace {
    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

fn fps_trace(data: &ProfileData) -> Box<FpsTrace> {
    Box::new(FpsTrace {
        type_: "scattergl",
        x: data
            .fps_samples
            .iter()
            .map(|sample| sample.elapsed_ms)
            .collect(),
        y: data.fps_samples.iter().map(|sample| sample.fps).collect(),
        name: "buffer FPS (1s rolling)",
        mode: "lines",
        line: WebGlLine {
            color: "#b279a2",
            width: 2.0,
        },
    })
}

fn average_fps(data: &ProfileData) -> f64 {
    if data.fps_samples.len() <= 1 {
        return 0.0;
    }

    let first = data.fps_samples.first().unwrap().elapsed_ms;
    let last = data.fps_samples.last().unwrap().elapsed_ms;
    if last > first {
        (data.fps_samples.len() - 1) as f64 * 1_000.0 / (last - first)
    } else {
        0.0
    }
}

fn render_report_html(plot: &Plot, frames: &[CanvasFrame], average_fps: f64) -> String {
    const TIMELINE: &str = r#"
<style>
  #timeline-panel { height: max(560px, calc(100vh - 320px)); min-height: 560px; font-family: sans-serif; }
  #timeline-toolbar { display: flex; align-items: center; gap: 10px; padding: 8px 14px; flex-wrap: wrap; }
  #timeline-toolbar h2 { margin: 0 14px 0 0; font-size: 18px; }
  #timeline-toolbar button { padding: 4px 10px; cursor: pointer; }
  #timeline-toolbar .legend { display: inline-flex; align-items: center; gap: 5px; }
  #timeline-toolbar .swatch { width: 14px; height: 10px; display: inline-block; }
  #timeline-toolbar .hint { color: #666; margin-left: auto; }
  #timeline-wrap { position: relative; height: calc(100% - 48px); width: 100%; overflow: hidden; }
  #timeline-canvas { display: block; width: 100%; height: 100%; cursor: grab; }
  #timeline-canvas.dragging { cursor: grabbing; }
  #timeline-tooltip { position: absolute; display: none; pointer-events: none; padding: 7px 9px;
    background: rgba(30, 30, 30, 0.92); color: white; border-radius: 4px; font: 12px monospace;
    white-space: nowrap; z-index: 2; }
</style>
<section id="timeline-panel">
  <div id="timeline-toolbar">
    <h2>Mutsumi frame timing — average buffer FPS: __AVERAGE_FPS__</h2>
    <button id="timeline-reset">Reset</button>
    <button id="timeline-fit-x">Fit time</button>
    <button id="timeline-all-frames">All frames</button>
    <label>Time mode
      <select id="timeline-mode">
        <option value="relative">Aligned frame latency</option>
        <option value="absolute">Absolute timeline</option>
      </select>
    </label>
    <span class="legend"><i class="swatch" style="background:#4c78a8"></i>queue</span>
    <span class="legend"><i class="swatch" style="background:#f58518"></i>texture build</span>
    <span class="legend"><i class="swatch" style="background:#54a24b"></i>built → snapshot</span>
    <span class="hint">Wheel: horizontal zoom · Shift+wheel: vertical zoom · Drag: pan</span>
  </div>
  <div id="timeline-wrap">
    <canvas id="timeline-canvas"></canvas>
    <div id="timeline-tooltip"></div>
  </div>
</section>
<script>
(() => {
  const frames = __FRAME_DATA__;
  const canvas = document.getElementById('timeline-canvas');
  const wrap = document.getElementById('timeline-wrap');
  const tooltip = document.getElementById('timeline-tooltip');
  const ctx = canvas.getContext('2d', {alpha: false});
  const colors = ['#4c78a8', '#f58518', '#54a24b'];
  const initialRows = Math.min(frames.length, 120);
  const totals = frames.map(f => f.t[3] ?? f.t[2] ?? f.t[1] ?? 0).sort((a, b) => a - b);
  const percentile = totals.length ? totals[Math.min(Math.floor(totals.length * 0.99), totals.length - 1)] : 1;
  const fitX = Math.max(percentile * 1.15, 1);
  const absoluteMin = frames.length ? frames[0].b : 0;
  const absoluteMax = frames.reduce((max, frame) => {
    const end = frame.t[3] ?? frame.t[2] ?? frame.t[1] ?? 0;
    return Math.max(max, frame.b + end);
  }, absoluteMin + 1);
  const state = {mode: 'relative', x0: 0, x1: fitX, y0: 0, y1: Math.max(initialRows, 1)};
  const margin = {left: 72, right: 18, top: 10, bottom: 38};
  let dragging = false;
  let dragStart = null;

  const clampY = () => {
    const span = Math.min(Math.max(state.y1 - state.y0, 1), Math.max(frames.length, 1));
    state.y0 = Math.min(Math.max(state.y0, 0), Math.max(frames.length - span, 0));
    state.y1 = state.y0 + span;
  };
  const fitTime = () => {
    if (state.mode === 'absolute') {
      state.x0 = absoluteMin;
      state.x1 = Math.max(absoluteMax, absoluteMin + 1);
    } else {
      state.x0 = 0;
      state.x1 = fitX;
    }
  };
  const stageTime = (frame, relativeTime) =>
    relativeTime == null ? null : state.mode === 'absolute' ? frame.b + relativeTime : relativeTime;
  const resize = () => {
    const dpr = window.devicePixelRatio || 1;
    const rect = wrap.getBoundingClientRect();
    canvas.width = Math.max(Math.floor(rect.width * dpr), 1);
    canvas.height = Math.max(Math.floor(rect.height * dpr), 1);
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    draw();
  };
  const metrics = () => {
    const width = canvas.width / (window.devicePixelRatio || 1);
    const height = canvas.height / (window.devicePixelRatio || 1);
    return {width, height, plotW: width - margin.left - margin.right, plotH: height - margin.top - margin.bottom};
  };
  const draw = () => {
    const m = metrics();
    ctx.fillStyle = '#ffffff';
    ctx.fillRect(0, 0, m.width, m.height);
    if (!frames.length || m.plotW <= 0 || m.plotH <= 0) return;

    const xToPx = value => margin.left + (value - state.x0) / (state.x1 - state.x0) * m.plotW;
    const rowH = m.plotH / (state.y1 - state.y0);
    const first = Math.max(Math.floor(state.y0), 0);
    const last = Math.min(Math.ceil(state.y1), frames.length);

    ctx.save();
    ctx.beginPath();
    ctx.rect(margin.left, margin.top, m.plotW, m.plotH);
    ctx.clip();
    for (let index = first; index < last; index++) {
      const frame = frames[index];
      const y = margin.top + (index - state.y0) * rowH;
      const t = frame.t;
      const segments = [
        [stageTime(frame, t[0]), stageTime(frame, t[1])],
        [stageTime(frame, t[1]), stageTime(frame, t[2])],
        [stageTime(frame, t[2]), stageTime(frame, t[3])],
      ];
      for (let phase = 0; phase < segments.length; phase++) {
        const [start, end] = segments[phase];
        if (start == null || end == null) continue;
        const left = xToPx(start);
        const right = xToPx(end);
        ctx.fillStyle = colors[phase];
        ctx.fillRect(left, y, Math.max(right - left, 1), rowH + 0.75);
      }
      ctx.strokeStyle = 'rgba(20,20,20,0.75)';
      ctx.lineWidth = 1;
      for (const relativeStage of t) {
        const stage = stageTime(frame, relativeStage);
        if (stage == null) continue;
        const x = Math.round(xToPx(stage)) + 0.5;
        ctx.beginPath();
        ctx.moveTo(x, y);
        ctx.lineTo(x, y + rowH);
        ctx.stroke();
      }
    }
    ctx.restore();

    ctx.strokeStyle = '#dddddd';
    ctx.fillStyle = '#555555';
    ctx.font = '12px sans-serif';
    ctx.textAlign = 'center';
    const xTicks = 10;
    for (let tick = 0; tick <= xTicks; tick++) {
      const px = margin.left + tick / xTicks * m.plotW;
      const value = state.x0 + tick / xTicks * (state.x1 - state.x0);
      ctx.beginPath();
      ctx.moveTo(px, margin.top);
      ctx.lineTo(px, margin.top + m.plotH);
      ctx.stroke();
      ctx.fillText(`${value.toFixed(value < 10 ? 2 : 1)} ms`, px, m.height - 12);
    }

    ctx.textAlign = 'right';
    const labelStep = Math.max(Math.ceil(18 / rowH), 1);
    for (let index = first; index < last; index += labelStep) {
      const y = margin.top + (index + 0.5 - state.y0) * rowH + 4;
      ctx.fillText(String(frames[index].i), margin.left - 8, y);
    }
  };

  const frameAt = event => {
    const rect = canvas.getBoundingClientRect();
    const m = metrics();
    const localY = event.clientY - rect.top - margin.top;
    const index = Math.floor(state.y0 + localY / m.plotH * (state.y1 - state.y0));
    return index >= 0 && index < frames.length ? frames[index] : null;
  };
  canvas.addEventListener('mousemove', event => {
    if (dragging) {
      const m = metrics();
      const dx = event.clientX - dragStart.x;
      const dy = event.clientY - dragStart.y;
      const xSpan = state.x1 - state.x0;
      const ySpan = state.y1 - state.y0;
      state.x0 = dragStart.x0 - dx / m.plotW * xSpan;
      state.x1 = dragStart.x1 - dx / m.plotW * xSpan;
      state.y0 = dragStart.y0 - dy / m.plotH * ySpan;
      state.y1 = state.y0 + ySpan;
      clampY();
      tooltip.style.display = 'none';
      draw();
      return;
    }
    const frame = frameAt(event);
    if (!frame) { tooltip.style.display = 'none'; return; }
    const t = frame.t;
    const duration = (a, b) => a == null || b == null ? 'n/a' : `${(b - a).toFixed(3)} ms`;
    tooltip.innerHTML = `frame=${frame.i}<br>mode=${state.mode}<br>total=${duration(t[0], t[3])}<br>queue=${duration(t[0], t[1])}<br>texture build=${duration(t[1], t[2])}<br>snapshot wait=${duration(t[2], t[3])}<br>buffer at=${frame.b.toFixed(3)} ms<br>unix=${frame.u} us`;
    tooltip.style.display = 'block';
    tooltip.style.left = `${Math.min(event.offsetX + 14, wrap.clientWidth - tooltip.offsetWidth - 8)}px`;
    tooltip.style.top = `${Math.min(event.offsetY + 14, wrap.clientHeight - tooltip.offsetHeight - 8)}px`;
  });
  canvas.addEventListener('mouseleave', () => { tooltip.style.display = 'none'; });
  canvas.addEventListener('pointerdown', event => {
    dragging = true;
    canvas.classList.add('dragging');
    canvas.setPointerCapture(event.pointerId);
    dragStart = {x: event.clientX, y: event.clientY, x0: state.x0, x1: state.x1, y0: state.y0, y1: state.y1};
  });
  canvas.addEventListener('pointerup', event => {
    dragging = false;
    canvas.classList.remove('dragging');
    canvas.releasePointerCapture(event.pointerId);
  });
  canvas.addEventListener('wheel', event => {
    event.preventDefault();
    const rect = canvas.getBoundingClientRect();
    const m = metrics();
    const factor = Math.exp(event.deltaY * 0.0015);
    if (event.shiftKey) {
      const anchor = state.y0 + (event.clientY - rect.top - margin.top) / m.plotH * (state.y1 - state.y0);
      const span = Math.min(Math.max((state.y1 - state.y0) * factor, 1), Math.max(frames.length, 1));
      const ratio = (anchor - state.y0) / (state.y1 - state.y0);
      state.y0 = anchor - span * ratio;
      state.y1 = state.y0 + span;
      clampY();
    } else {
      const anchor = state.x0 + (event.clientX - rect.left - margin.left) / m.plotW * (state.x1 - state.x0);
      const span = Math.max((state.x1 - state.x0) * factor, 0.001);
      const ratio = (anchor - state.x0) / (state.x1 - state.x0);
      state.x0 = anchor - span * ratio;
      state.x1 = anchor + span * (1 - ratio);
    }
    draw();
  }, {passive: false});

  document.getElementById('timeline-reset').onclick = () => {
    state.y0 = 0; state.y1 = Math.max(initialRows, 1); fitTime(); draw();
  };
  document.getElementById('timeline-fit-x').onclick = () => { fitTime(); draw(); };
  document.getElementById('timeline-all-frames').onclick = () => {
    state.y0 = 0; state.y1 = Math.max(frames.length, 1); draw();
  };
  document.getElementById('timeline-mode').onchange = event => {
    state.mode = event.target.value;
    fitTime();
    draw();
  };
  new ResizeObserver(resize).observe(wrap);
  resize();
})();
</script>
"#;

    let frame_data = serde_json::to_string(frames).unwrap();
    let timeline = TIMELINE
        .replace("__FRAME_DATA__", &frame_data)
        .replace("__AVERAGE_FPS__", &format!("{average_fps:.2}"));
    plot.to_html()
        .replacen("<body>", &format!("<body>{timeline}"), 1)
}

fn write_report(path: &PathBuf) -> std::io::Result<()> {
    let data = PROFILE.lock().unwrap();
    let frames = canvas_frames(&data);
    let average_fps = average_fps(&data);
    let mut plot = Plot::new();
    plot.set_configuration(Configuration::new().responsive(true).scroll_zoom(true));
    plot.add_trace(fps_trace(&data));
    plot.set_layout(
        Layout::new()
            .title(Title::with_text("Buffer FPS (1 second rolling window)"))
            .height(280)
            .x_axis(Axis::new().title(Title::with_text("Time since profiling start (ms)")))
            .y_axis(Axis::new().title(Title::with_text("Buffer FPS"))),
    );

    std::fs::write(path, render_report_html(&plot, &frames, average_fps))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_large_virtualized_html_report() {
        let path = std::env::temp_dir().join(format!(
            "mutsumi-proxy-profile-test-{}.html",
            std::process::id()
        ));

        {
            let mut data = PROFILE.lock().unwrap();
            data.reset();
            for frame_id in 1..=8_000 {
                let buffer = (frame_id - 1) as f64 * 16.667;
                for (stage, elapsed_ms) in [
                    (Stage::BufferProduced, buffer),
                    (Stage::TextureBuildStarted, buffer + 0.2),
                    (Stage::TextureBuilt, buffer + 0.8),
                    (Stage::Snapshot, buffer + 1.4),
                ] {
                    data.samples.push(Sample {
                        frame_id,
                        elapsed_ms,
                        unix_timestamp_us: 1_000_000 + (elapsed_ms * 1_000.0) as u128,
                        stage,
                    });
                }
                data.fps_samples.push(FpsSample {
                    elapsed_ms: buffer,
                    fps: 60.0,
                });
            }
        }

        write_report(&path).unwrap();
        let html = std::fs::read_to_string(&path).unwrap();
        assert!(html.contains("Mutsumi frame timing"));
        assert!(html.contains("timeline-canvas"));
        assert!(html.contains("Shift+wheel: vertical zoom"));
        assert!(html.contains("Absolute timeline"));
        assert!(html.contains("stageTime(frame, t[3])"));
        assert!(html.contains("rowH + 0.75"));
        assert!(html.contains("buffer FPS (1s rolling)"));
        assert!(html.contains("\"type\":\"scattergl\""));
        assert!(html.len() < 15 * 1024 * 1024);
        std::fs::remove_file(path).unwrap();
    }
}
