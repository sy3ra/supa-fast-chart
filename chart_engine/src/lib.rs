use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub enum MaType {
    SMA,
    EMA,
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub enum MaSource {
    Close,
    Open,
    High,
    Low,
    HL2,
    HLC3,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MASettings {
    pub active: bool,
    pub visible: bool,
    pub period: usize,
    pub source: String,
    pub method: String,
    pub color: String,
    pub line_width: f32,
    pub offset: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MovingAverage {
    pub id: u32,
    pub settings: MASettings,
    #[serde(skip)]
    pub cached_values: Vec<f64>,
    #[serde(skip)]
    pub render_buffer: Vec<f32>,
}

impl MovingAverage {
    pub fn new(id: u32, settings: MASettings) -> Self {
        MovingAverage {
            id,
            settings,
            cached_values: Vec::new(),
            render_buffer: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BBSettings {
    pub active: bool,
    pub visible: bool,
    pub period: usize,
    pub multiplier: f64,
    pub source: String,
    pub color: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BollingerBand {
    pub id: u32,
    pub settings: BBSettings,
    #[serde(skip)]
    pub middle: Vec<f64>,
    #[serde(skip)]
    pub upper: Vec<f64>,
    #[serde(skip)]
    pub lower: Vec<f64>,
    #[serde(skip)]
    pub render_buffer: Vec<f32>,
}

impl BollingerBand {
    pub fn new(id: u32, settings: BBSettings) -> Self {
        Self {
            id,
            settings,
            middle: Vec::new(),
            upper: Vec::new(),
            lower: Vec::new(),
            render_buffer: Vec::new(),
        }
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum DrawingType {
    Segment = 0,
    Line = 1,
    Ray = 2,
    Horizontal = 3,
    Vertical = 4,
}

impl DrawingType {
    fn from_u8(v: u8) -> DrawingType {
        match v {
            0 => DrawingType::Segment,
            1 => DrawingType::Line,
            2 => DrawingType::Ray,
            3 => DrawingType::Horizontal,
            4 => DrawingType::Vertical,
            _ => DrawingType::Segment,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Point {
    pub x: f64, // Virtual Index
    pub y: f64, // Price
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DrawingObject {
    pub shape: DrawingType,
    pub p1: Point,
    pub p2: Point,
    pub p1_time: f64,
    pub p2_time: f64,
    pub active: bool,
    pub color: String,
    pub width: f32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DrawingTimeObject {
    pub shape: DrawingType,
    pub p1_time: f64,
    pub p1_price: f64,
    pub p2_time: f64,
    pub p2_price: f64,
    pub active: bool,
    pub color: String,
    pub width: f32,
}

#[derive(Serialize, Deserialize)]
pub struct DrawingProps {
    pub color: String,
    pub line_width: f32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Candle {
    pub time: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Serialize, Deserialize)]
pub struct ChartContext {
    pub id: u32,
    pub symbol: String,
    pub interval: String,

    pub width: f32,
    pub height: f32,
    pub offset_x: f32,
    pub scale_x: f32,

    pub visible_min_price: f64,
    pub visible_max_price: f64,
    pub visible_start_index: usize,
    pub visible_end_index: usize,

    pub candles: Vec<Candle>,
    pub render_buffer: Vec<f32>,

    // Drawing State
    pub drawings: Vec<DrawingObject>,
    pub current_drawing: Option<DrawingObject>,
    pub selected_drawing: Option<usize>,
    pub drag_active_anchor: Option<u8>,
    pub drag_offset_p1: Point,
    pub drag_offset_p2: Point,

    pub undo_stack: Vec<Vec<DrawingTimeObject>>,
    pub redo_stack: Vec<Vec<DrawingTimeObject>>,

    pub drawing_buffer: Vec<f32>,

    // Free Scale State
    pub is_auto_scale: bool,
    pub manual_min_price: f64,
    pub manual_max_price: f64,

    pub ma_lines: Vec<MovingAverage>,
    pub next_ma_id: u32,

    pub bb_bands: Vec<BollingerBand>,
    pub next_bb_id: u32,
}

#[wasm_bindgen]
pub struct ChartEngine {
    charts: HashMap<u32, ChartContext>,
    active_chart_id: Option<u32>,
    next_chart_id: u32,

    active_color: String,
    active_width: f32,
}

#[wasm_bindgen]
impl ChartEngine {
    fn ctx(&self) -> &ChartContext {
        self.charts
            .get(&self.active_chart_id.expect("No active chart"))
            .expect("Active chart key not found")
    }

    fn ctx_mut(&mut self) -> &mut ChartContext {
        self.charts
            .get_mut(&self.active_chart_id.expect("No active chart"))
            .expect("Active chart key not found")
    }

    // --- Multi-Chart Management ---

    pub fn create_tab(&mut self, symbol: String, interval: String) -> u32 {
        let id = self.next_chart_id;
        self.next_chart_id += 1;

        // Clone active chart props for seamless transition if possible, or default
        // For now, simpler: Defaults.
        // We'll reuse the active ctx width/height if available, else default.
        let (w, h) = if let Some(active_id) = self.active_chart_id {
            if let Some(ctx) = self.charts.get(&active_id) {
                (ctx.width, ctx.height)
            } else {
                (800.0, 600.0)
            }
        } else {
            (800.0, 600.0)
        };

        let new_ctx = ChartContext {
            id,
            symbol,
            interval,
            width: w,
            height: h,
            offset_x: 0.0,
            scale_x: 10.0,
            visible_min_price: 0.0,
            visible_max_price: 100.0,
            visible_start_index: 0,
            visible_end_index: 0,
            candles: Vec::new(),
            render_buffer: Vec::new(),
            drawings: Vec::new(),
            current_drawing: None,
            selected_drawing: None,
            drag_active_anchor: None,
            drag_offset_p1: Point { x: 0.0, y: 0.0 },
            drag_offset_p2: Point { x: 0.0, y: 0.0 },
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            drawing_buffer: Vec::new(),
            is_auto_scale: true,
            manual_min_price: 0.0,
            manual_max_price: 100.0,
            ma_lines: Vec::new(),
            next_ma_id: 1,
            bb_bands: vec![BollingerBand::new(
                1,
                BBSettings {
                    active: true,
                    period: 20,
                    multiplier: 2.0,
                    source: "Close".to_string(), // Fixed: String not Enum
                    color: "#00ccff".to_string(),
                    visible: true,
                },
            )],
            next_bb_id: 2,
        };

        self.charts.insert(id, new_ctx);
        self.active_chart_id = Some(id); // Set as active
        self.update_view(); // Render emptiness or whatever is there
        id
    }

    pub fn switch_tab(&mut self, id: u32) -> bool {
        if self.charts.contains_key(&id) {
            self.active_chart_id = Some(id);
            self.update_view(); // Simply re-render current state
            return true;
        }
        false
    }

    pub fn close_tab(&mut self, id: u32) {
        if self.charts.len() <= 1 {
            // Don't remove the last chart to avoid "No active chart" panic on next call
            return;
        }
        self.charts.remove(&id);

        // If we removed the active chart, switch to another
        if self.active_chart_id == Some(id) {
            // Pick any other ID
            if let Some(&first_id) = self.charts.keys().next() {
                self.active_chart_id = Some(first_id);
                self.update_view();
            } else {
                self.active_chart_id = None; // Should catch by Guard above
            }
        }
    }

    pub fn set_tab_interval(&mut self, id: u32, interval: String) {
        if let Some(ctx) = self.charts.get_mut(&id) {
            ctx.interval = interval;
        }
    }

    pub fn get_tabs_info(&self) -> JsValue {
        #[derive(Serialize)]
        struct TabInfo {
            id: u32,
            symbol: String,
            interval: String,
            is_active: bool,
        }

        let mut tabs = Vec::new();
        // Return sorted by ID for stability in UI
        let mut ids: Vec<&u32> = self.charts.keys().collect();
        ids.sort();

        for id in ids {
            if let Some(ctx) = self.charts.get(id) {
                tabs.push(TabInfo {
                    id: *id,
                    symbol: ctx.symbol.clone(),
                    interval: ctx.interval.clone(),
                    is_active: self.active_chart_id == Some(*id),
                });
            }
        }

        serde_wasm_bindgen::to_value(&tabs).unwrap_or(JsValue::NULL)
    }

    #[wasm_bindgen(constructor)]
    pub fn new(canvas_width: f64, canvas_height: f64) -> ChartEngine {
        log("Initializing ChartEngine... (v12 Multi-MA)");
        let mut charts = HashMap::new();

        // Create Default Context
        let default_ctx = ChartContext {
            id: 1,
            symbol: "BTCUSDT".to_string(),
            interval: "1d".to_string(),
            width: canvas_width as f32,
            height: canvas_height as f32,
            offset_x: 0.0,
            scale_x: 10.0, // pixels per candle
            visible_min_price: 0.0,
            visible_max_price: 100.0,
            visible_start_index: 0,
            visible_end_index: 0,
            candles: Vec::new(),
            render_buffer: Vec::new(),
            drawings: Vec::new(),
            current_drawing: None,
            selected_drawing: None,
            drag_active_anchor: None,
            drag_offset_p1: Point { x: 0.0, y: 0.0 },
            drag_offset_p2: Point { x: 0.0, y: 0.0 },
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            drawing_buffer: Vec::new(),
            is_auto_scale: true,
            manual_min_price: 0.0,
            manual_max_price: 100.0,
            ma_lines: Vec::new(),
            next_ma_id: 1,
            bb_bands: vec![BollingerBand::new(
                1,
                BBSettings {
                    active: true,
                    period: 20,
                    multiplier: 2.0,
                    source: "Close".to_string(), // Fixed: String not Enum
                    color: "#00ccff".to_string(), // Cyan
                    visible: true,
                },
            )],
            next_bb_id: 2,
        };

        charts.insert(1, default_ctx);

        ChartEngine {
            charts,
            active_chart_id: Some(1),
            next_chart_id: 2,
            active_color: "#00FFFF".to_string(), // Default Cyan
            active_width: 2.0,                   // Default Width
        }
    }

    pub fn set_candles(&mut self, data: &[f64]) {
        // 1. Migration Snapshot: Save Drawings with Time
        let snapshot = self.create_snapshot();
        let mut center_time_snapshot = 0.0;
        let mut anchor_pixel_snapshot = 0.0;

        {
            let ctx = self.ctx();
            let has_data = !ctx.candles.is_empty();

            if has_data {
                // Snapshot View Right Edge
                let right_px = ctx.width;
                let right_idx = (right_px - ctx.offset_x) / ctx.scale_x;
                center_time_snapshot = self.get_time_at_index(right_idx as f64);
                anchor_pixel_snapshot = right_px;
            }
        }

        let ctx = self.ctx_mut();
        ctx.candles.clear();
        for chunk in data.chunks(6) {
            if chunk.len() == 6 {
                ctx.candles.push(Candle {
                    time: chunk[0] as i64,
                    open: chunk[1],
                    high: chunk[2],
                    low: chunk[3],
                    close: chunk[4],
                    volume: chunk[5],
                });
            }
        }

        // Handle Restore View Right Edge after dropping mut borrow
        if !self.ctx().candles.is_empty() && center_time_snapshot != 0.0 {
            let new_center_idx = self.get_index_for_time(center_time_snapshot);
            let ctx = self.ctx_mut();
            ctx.offset_x = anchor_pixel_snapshot - ((new_center_idx as f32) * ctx.scale_x);
        }

        // 2. Restore Drawings
        self.ctx_mut().drawings = self.restore_snapshot(snapshot);

        // Recalculate - use split borrows logic or strictly non-overlapping
        let ctx = self.ctx_mut();

        // MAs
        // We need to pass &ctx.candles to calculate.
        // ma borrows ctx.ma_lines (mut).
        // ctx.candles is disjoint.
        // This is fine because `ma` is a mutable reference to an element in `ctx.ma_lines`,
        // and `ctx.candles` is a separate field.
        for ma in &mut ctx.ma_lines {
            ma.calculate(&ctx.candles);
        }
        // BBs
        for bb in &mut ctx.bb_bands {
            bb.calculate(&ctx.candles);
        }

        self.update_view();
    }

    pub fn replace_candles(&mut self, chart_id: u32, data: &[f64]) {
        // 1. Migration Snapshot: Save Drawings with Time (Only if targeting active chart? Or logic should be generic)
        // For now, simpler: targeting specific chart_id.
        // If ID not found, strict error or ignore? Ignore for now to avoid crashes.
        if !self.charts.contains_key(&chart_id) {
            return;
        }

        // 1. Migration Snapshot: Save Drawings with Time
        let snapshot = self.create_snapshot_for(chart_id);

        // 1b. View Snapshot: Save Time at rightmost of Screen
        let mut anchor_time: Option<i64> = None;
        let mut anchor_pixel: f32 = 0.0;
        if let Some(ctx) = self.charts.get(&chart_id) {
            if !ctx.candles.is_empty() {
                let right_pixel = ctx.width;
                let right_idx = ((right_pixel - ctx.offset_x) / ctx.scale_x).round() as isize;
                let safe_idx = if right_idx < 0 {
                    0
                } else if right_idx >= ctx.candles.len() as isize {
                    ctx.candles.len() - 1
                } else {
                    right_idx as usize
                };
                anchor_time = Some(ctx.candles[safe_idx].time);
                anchor_pixel = right_pixel;
            }
        }

        log(&format!(
            "Migrating {} drawings for Chart {}",
            snapshot.len(),
            chart_id
        ));

        {
            let ctx = self.charts.get_mut(&chart_id).unwrap();
            ctx.candles.clear();
            for chunk in data.chunks(6) {
                if chunk.len() == 6 {
                    ctx.candles.push(Candle {
                        time: chunk[0] as i64,
                        open: chunk[1],
                        high: chunk[2],
                        low: chunk[3],
                        close: chunk[4],
                        volume: chunk[5],
                    });
                }
            }
        }

        // 3. Restore Drawings at new Indices
        let new_drawings = self.restore_snapshot_for(chart_id, snapshot);

        if let Some(ctx) = self.charts.get_mut(&chart_id) {
            ctx.drawings = new_drawings;
        }

        // Recalculate indicators
        self.recalc_indicators(chart_id);

        // 4. Time-Based View Restoration
        let mut restored_view = false;
        if let Some(target_time) = anchor_time {
            if let Some(ctx) = self.charts.get_mut(&chart_id) {
                match ctx.candles.binary_search_by(|c| c.time.cmp(&target_time)) {
                    Ok(idx) => {
                        ctx.offset_x = anchor_pixel - (idx as f32 * ctx.scale_x);
                        restored_view = true;
                        log("View Restored via Exact Time Sync");
                    }
                    Err(idx) => {
                        let bound_idx = if idx >= ctx.candles.len() {
                            ctx.candles.len().saturating_sub(1)
                        } else {
                            idx
                        };
                        ctx.offset_x = anchor_pixel - (bound_idx as f32 * ctx.scale_x);
                        restored_view = true;
                        log("View Restored via Approximate Time Sync");
                    }
                }
            }
        }

        // 5. Smart View Reset (Fallback)
        let mut need_reset = false;
        if !restored_view {
            if let Some(ctx) = self.charts.get(&chart_id) {
                let len = ctx.candles.len();
                if len > 0 {
                    let start_index = ((-ctx.offset_x) / ctx.scale_x).floor() as isize;
                    // If we are looking at index 5000 but we only have 1000 candles...
                    if start_index > len as isize {
                        need_reset = true;
                    }
                } else {
                    need_reset = true;
                }
            }
        }

        if need_reset {
            self.reset_view(chart_id);
        } else {
            // 6. Force Render (if active)
            if self.active_chart_id == Some(chart_id) {
                self.update_view();
            }
        }
    }

    // 3. Restore Drawings at new Indices
    // restore_snapshot calls internal methods but doesn't conflict if we dropped ctx
    // This block was part of set_candles, but the instruction implies it's a separate section.
    // I'm assuming the instruction meant to modify the set_candles function's end.
    // The original set_candles function ends with:
    // self.ctx_mut().drawings = self.restore_snapshot(snapshot);
    // self.recalc_indicators(self.active_chart_id.unwrap()); // Assuming set_candles always acts on active chart
    // self.reset_view();
    // I will place the new functions after the set_candles function.

    fn recalc_indicators(&mut self, chart_id: u32) {
        if let Some(ctx) = self.charts.get_mut(&chart_id) {
            let candles_clone = ctx.candles.clone(); // Clone to avoid borrow conflict

            for ma in &mut ctx.ma_lines {
                ma.calculate(&candles_clone);
            }
            for bb in &mut ctx.bb_bands {
                bb.calculate(&candles_clone);
            }
        }
    }

    pub fn update_last_candle(&mut self, chart_id: u32, price: f64) {
        if let Some(ctx) = self.charts.get_mut(&chart_id) {
            if let Some(last) = ctx.candles.last_mut() {
                last.close = price;
                if price > last.high {
                    last.high = price;
                }
                if price < last.low {
                    last.low = price;
                }
                // Optional: handle new candle time check? For now just price update.
            }
            // Trigger recalc? maybe optimization needed.
        }
        // If visible, update view
        if self.active_chart_id == Some(chart_id) {
            self.update_view(); // Active only
        }
    }

    pub fn update_live_candle(
        &mut self,
        chart_id: u32,
        time: f64,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
        is_closed: bool,
    ) {
        let mut new_candle_added = false;

        if let Some(ctx) = self.charts.get_mut(&chart_id) {
            let target_time = time as i64;

            if let Some(last) = ctx.candles.last_mut() {
                if last.time == target_time {
                    // Update current candle
                    last.open = open;
                    last.high = high;
                    last.low = low;
                    last.close = close;
                    last.volume = volume;
                } else if target_time > last.time {
                    // New candle formed
                    ctx.candles.push(Candle {
                        time: target_time,
                        open,
                        high,
                        low,
                        close,
                        volume,
                    });
                    new_candle_added = true;
                }
            } else {
                // Empty chart, just push
                ctx.candles.push(Candle {
                    time: target_time,
                    open,
                    high,
                    low,
                    close,
                    volume,
                });
                new_candle_added = true;
            }
        }

        if new_candle_added {
            // Auto-scroll if camera was at the very right
            if let Some(ctx) = self.charts.get_mut(&chart_id) {
                // simple scroll right by 1 candle width if it's following the latest price
                let right_edge_px = ctx.offset_x + (ctx.candles.len() as f32) * ctx.scale_x;
                if right_edge_px > ctx.width {
                    ctx.offset_x -= ctx.scale_x;
                }
            }
        }

        // Recalculate MA/BB
        self.recalc_indicators(chart_id);

        // Render if active
        if self.active_chart_id == Some(chart_id) {
            self.update_view();
        }
    }

    fn interval_to_ms(interval: &str) -> f64 {
        match interval {
            "1s" => 1000.0,
            "1m" => 60_000.0,
            "3m" => 180_000.0,
            "5m" => 300_000.0,
            "15m" => 900_000.0,
            "30m" => 1_800_000.0,
            "1h" => 3_600_000.0,
            "2h" => 7_200_000.0,
            "4h" => 14_400_000.0,
            "6h" => 21_600_000.0,
            "8h" => 28_800_000.0,
            "12h" => 43_200_000.0,
            "1d" => 86_400_000.0,
            "3d" => 259_200_000.0,
            "1w" => 604_800_000.0,
            "1M" => 2_592_000_000.0,
            _ => 60_000.0, // default 1m
        }
    }
    fn get_time_at_index(&self, idx: f64) -> f64 {
        if self.ctx().candles.is_empty() {
            return 0.0;
        }
        if self.ctx().candles.len() == 1 {
            return self.ctx().candles[0].time as f64;
        }

        let len = self.ctx().candles.len();
        let first_time = self.ctx().candles[0].time as f64;
        let last_time = self.ctx().candles[len - 1].time as f64;

        // Use strict static interval
        let avg_interval = Self::interval_to_ms(&self.ctx().interval);

        if avg_interval <= 0.0 {
            return first_time;
        }

        let float_idx = idx; // idx is float

        // Extrapolate
        if float_idx < 0.0 {
            return first_time + (float_idx * avg_interval);
        }
        if float_idx > (len - 1) as f64 {
            let overflow = float_idx - (len - 1) as f64;
            return last_time + (overflow * avg_interval);
        }

        // Interpolate within range
        let i_floor = float_idx.floor() as usize;
        let i_ceil = float_idx.ceil() as usize;

        if i_floor == i_ceil {
            return self.ctx().candles[i_floor].time as f64;
        }

        // Linear interpolate between actual candle times for better precision
        let t1 = self.ctx().candles[i_floor].time as f64;
        let t2 = self.ctx().candles[i_ceil].time as f64;
        let ratio = float_idx - i_floor as f64;

        t1 + (t2 - t1) * ratio
    }

    fn get_index_for_time(&self, target_time: f64) -> f64 {
        if self.ctx().candles.is_empty() {
            return 0.0;
        }
        if self.ctx().candles.len() == 1 {
            return 0.0;
        }

        let len = self.ctx().candles.len();
        let first_time = self.ctx().candles[0].time as f64;
        let last_time = self.ctx().candles[len - 1].time as f64;

        // Use strict static interval
        let avg_interval = Self::interval_to_ms(&self.ctx().interval);

        if avg_interval <= 0.0 {
            return 0.0;
        }

        // Extrapolate
        if target_time < first_time {
            let diff = target_time - first_time;
            return diff / avg_interval; // Will be negative
        }
        if target_time > last_time {
            let diff = target_time - last_time;
            return (len - 1) as f64 + (diff / avg_interval);
        }

        // Binary search for closest candle
        let result = self.ctx().candles.binary_search_by(|c| {
            (c.time as f64)
                .partial_cmp(&target_time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        match result {
            Ok(idx) => idx as f64,
            Err(idx) => {
                // idx is insertion point.
                if idx == 0 {
                    return 0.0;
                }
                if idx >= len {
                    return (len - 1) as f64;
                }

                // Interpolate
                let t_prev = self.ctx().candles[idx - 1].time as f64;
                let t_next = self.ctx().candles[idx].time as f64;
                let range = t_next - t_prev;

                if range == 0.0 {
                    return idx as f64;
                }

                let ratio = (target_time - t_prev) / range;
                (idx - 1) as f64 + ratio
            }
        }
    }

    pub fn prepend_candles(&mut self, chart_id: u32, data: &[f64]) {
        if !self.charts.contains_key(&chart_id) {
            return;
        }

        let mut new_candles = Vec::new();
        for chunk in data.chunks(6) {
            if chunk.len() == 6 {
                new_candles.push(Candle {
                    time: chunk[0] as i64,
                    open: chunk[1],
                    high: chunk[2],
                    low: chunk[3],
                    close: chunk[4],
                    volume: chunk[5],
                });
            }
        }

        if new_candles.is_empty() {
            return;
        }

        let count = new_candles.len();

        {
            let ctx = self.charts.get_mut(&chart_id).unwrap();
            ctx.candles.splice(0..0, new_candles);

            let shift_pixels = count as f32 * ctx.scale_x;
            ctx.offset_x -= shift_pixels;

            // Remove manual index shifting since sync_drawing_indices will recalculate perfectly
        }

        self.sync_drawing_indices(chart_id);
        self.update_view(); // Active check is inside update_view ideally, but if it doesn't hurt it's fine

        self.recalc_indicators(chart_id);

        if self.active_chart_id == Some(chart_id) {
            self.update_view();
        }
    }

    // --- Drawing System API ---

    pub fn start_drawing(&mut self, shape_type: u8, sx: f32, sy: f32) {
        let p = self.screen_to_world(sx, sy);
        let time = {
            let chart_id = self.active_chart_id.unwrap();
            let ctx = self.charts.get(&chart_id).unwrap();
            Self::get_time_at_index_ctx(&ctx.candles, &ctx.interval, p.x)
        };
        let shape = DrawingType::from_u8(shape_type);
        self.ctx_mut().current_drawing = Some(DrawingObject {
            shape,
            p1: p,
            p2: p, // Initially p2 = p1
            p1_time: time,
            p2_time: time,
            active: true,
            color: self.active_color.clone(),
            width: self.active_width,
        });
        self.render_drawings(); // Update visual immediate
    }

    pub fn update_drawing(&mut self, sx: f32, sy: f32) {
        let p = self.screen_to_world(sx, sy);
        let time = {
            let chart_id = self.active_chart_id.unwrap();
            let ctx = self.charts.get(&chart_id).unwrap();
            Self::get_time_at_index_ctx(&ctx.candles, &ctx.interval, p.x)
        };
        if let Some(drawing) = &mut self.ctx_mut().current_drawing {
            drawing.p2 = p;
            drawing.p2_time = time;
        }
        self.render_drawings();
    }

    pub fn complete_drawing(&mut self) {
        if let Some(mut drawing) = self.ctx_mut().current_drawing.take() {
            self.save_state(); // Save before adding new drawing
            drawing.active = false;
            self.ctx_mut().drawings.push(drawing);
            self.render_drawings();
        }
    }

    // --- Helpers ---

    fn screen_to_world(&self, sx: f32, sy: f32) -> Point {
        // X: index = (sx - offset) / scale
        let x_index = (sx - self.ctx().offset_x) as f64 / self.ctx().scale_x as f64;

        // Y: Reverse the update_view logic
        // y_screen = height - padding - norm * avail
        // norm * avail = height - padding - y_screen
        // norm = (height - padding - y_screen) / avail
        // price = min + norm * range

        let chart_area_height = self.ctx().height * 0.7; // Top 70%
        let padding_price = 20.0;
        let avail_price_height = chart_area_height as f64 - (padding_price * 2.0);

        let mut price = 0.0;
        if avail_price_height > 0.0 {
            let norm =
                (chart_area_height as f64 - padding_price as f64 - sy as f64) / avail_price_height;
            let price_range = self.ctx().visible_max_price - self.ctx().visible_min_price;
            price = self.ctx().visible_min_price + (norm * price_range);
        }

        Point {
            x: x_index,
            y: price,
        }
    }

    // --- Undo/Redo ---

    // --- Undo/Redo (Time-Based) ---

    fn create_snapshot(&self) -> Vec<DrawingTimeObject> {
        let mut snapshot = Vec::new();
        // If no data, cannot map to time effectively, but should we save anyway?
        // If we save indexes as 0, it's bad.
        // Assuming we always have data when drawing.
        if self.ctx().candles.is_empty() {
            return snapshot;
        }

        for d in &self.ctx().drawings {
            snapshot.push(DrawingTimeObject {
                shape: d.shape,
                p1_time: d.p1_time,
                p1_price: d.p1.y,
                p2_time: d.p2_time,
                p2_price: d.p2.y,
                active: d.active,
                color: d.color.clone(),
                width: d.width,
            });
        }
        snapshot
    }

    fn sync_drawing_indices(&mut self, chart_id: u32) {
        if let Some(ctx) = self.charts.get_mut(&chart_id) {
            let len = ctx.candles.len();
            if len == 0 {
                return;
            }
            for d in &mut ctx.drawings {
                d.p1.x = Self::get_index_for_time_ctx(&ctx.candles, &ctx.interval, d.p1_time);
                d.p2.x = Self::get_index_for_time_ctx(&ctx.candles, &ctx.interval, d.p2_time);
            }
            if let Some(d) = &mut ctx.current_drawing {
                d.p1.x = Self::get_index_for_time_ctx(&ctx.candles, &ctx.interval, d.p1_time);
                d.p2.x = Self::get_index_for_time_ctx(&ctx.candles, &ctx.interval, d.p2_time);
            }
        }
    }

    fn create_snapshot_for(&self, chart_id: u32) -> Vec<DrawingTimeObject> {
        let mut snapshot = Vec::new();
        if let Some(ctx) = self.charts.get(&chart_id) {
            for d in &ctx.drawings {
                snapshot.push(DrawingTimeObject {
                    shape: d.shape,
                    p1_time: d.p1_time,
                    p1_price: d.p1.y,
                    p2_time: d.p2_time,
                    p2_price: d.p2.y,
                    active: d.active,
                    color: d.color.clone(),
                    width: d.width,
                });
            }
        }
        snapshot
    }

    fn restore_snapshot_for(
        &self,
        chart_id: u32,
        snapshot: Vec<DrawingTimeObject>,
    ) -> Vec<DrawingObject> {
        let mut drawings = Vec::new();
        if let Some(ctx) = self.charts.get(&chart_id) {
            if ctx.candles.is_empty() {
                return drawings;
            }
            for s in snapshot {
                drawings.push(DrawingObject {
                    shape: s.shape,
                    p1: Point {
                        x: Self::get_index_for_time_ctx(&ctx.candles, &ctx.interval, s.p1_time),
                        y: s.p1_price,
                    },
                    p2: Point {
                        x: Self::get_index_for_time_ctx(&ctx.candles, &ctx.interval, s.p2_time),
                        y: s.p2_price,
                    },
                    p1_time: s.p1_time,
                    p2_time: s.p2_time,
                    active: s.active,
                    color: s.color.clone(),
                    width: s.width,
                });
            }
        }
        drawings
    }

    fn get_time_at_index_ctx(candles: &[Candle], interval: &str, idx: f64) -> f64 {
        if candles.is_empty() {
            return 0.0;
        }
        if candles.len() == 1 {
            return candles[0].time as f64;
        }

        let len = candles.len();
        let first_time = candles[0].time as f64;
        let last_time = candles[len - 1].time as f64;

        // Use strict static interval
        let avg_interval = Self::interval_to_ms(interval);

        if avg_interval <= 0.0 {
            return first_time;
        }

        if idx < 0.0 {
            return first_time + (idx * avg_interval);
        }
        if idx > (len - 1) as f64 {
            let overflow = idx - (len - 1) as f64;
            return last_time + (overflow * avg_interval);
        }

        let i_floor = idx.floor() as usize;
        let i_ceil = idx.ceil() as usize;

        if i_floor >= len {
            return last_time;
        }

        let t1 = candles[i_floor].time as f64;
        let t2 = if i_ceil < len {
            candles[i_ceil].time as f64
        } else {
            t1
        };

        let ratio = idx - i_floor as f64;
        t1 + (t2 - t1) * ratio
    }

    fn get_index_for_time_ctx(candles: &[Candle], interval: &str, target_time: f64) -> f64 {
        if candles.is_empty() {
            return 0.0;
        }
        let len = candles.len();
        let first_time = candles[0].time as f64;
        let last_time = candles[len - 1].time as f64;

        // Use strict static interval
        let avg_interval = Self::interval_to_ms(interval);

        if avg_interval <= 0.0 {
            return 0.0;
        }

        if target_time < first_time {
            let diff = target_time - first_time;
            return diff / avg_interval;
        }
        if target_time > last_time {
            let diff = target_time - last_time;
            return (len - 1) as f64 + (diff / avg_interval);
        }

        let result = candles.binary_search_by(|c| {
            (c.time as f64)
                .partial_cmp(&target_time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        match result {
            Ok(idx) => idx as f64,
            Err(idx) => {
                if idx == 0 {
                    return 0.0;
                }
                if idx >= len {
                    return (len - 1) as f64;
                }
                let t_prev = candles[idx - 1].time as f64;
                let t_next = candles[idx].time as f64;
                let range = t_next - t_prev;
                if range == 0.0 {
                    return idx as f64;
                }
                let ratio = (target_time - t_prev) / range;
                (idx - 1) as f64 + ratio
            }
        }
    }

    fn restore_snapshot(&self, snapshot: Vec<DrawingTimeObject>) -> Vec<DrawingObject> {
        let mut drawings = Vec::new();
        if self.ctx().candles.is_empty() {
            return drawings;
        }

        for s in snapshot {
            drawings.push(DrawingObject {
                shape: s.shape,
                p1: Point {
                    x: self.get_index_for_time(s.p1_time),
                    y: s.p1_price,
                },
                p2: Point {
                    x: self.get_index_for_time(s.p2_time),
                    y: s.p2_price,
                },
                p1_time: s.p1_time,
                p2_time: s.p2_time,
                active: s.active,
                color: s.color.clone(),
                width: s.width,
            });
        }
        drawings
    }

    pub fn save_state(&mut self) {
        // Create Time-Based Snapshot
        let snapshot = self.create_snapshot();
        self.ctx_mut().undo_stack.push(snapshot);

        // Clear redo stack on new action
        self.ctx_mut().redo_stack.clear();

        // Safety limit (optional)
        if self.ctx_mut().undo_stack.len() > 50 {
            self.ctx_mut().undo_stack.remove(0);
        }
    }

    pub fn undo(&mut self) {
        if let Some(prev_snapshot) = self.ctx_mut().undo_stack.pop() {
            // Save current to redo (as snapshot)
            let current_snapshot = self.create_snapshot();
            self.ctx_mut().redo_stack.push(current_snapshot);

            // Restore
            self.ctx_mut().drawings = self.restore_snapshot(prev_snapshot);
            self.ctx_mut().selected_drawing = None; // Reset selection to avoid invalid index
            self.render_drawings();
        }
    }

    pub fn redo(&mut self) {
        if let Some(next_snapshot) = self.ctx_mut().redo_stack.pop() {
            // Save current to undo
            let current_snapshot = self.create_snapshot();
            self.ctx_mut().undo_stack.push(current_snapshot);

            // Restore
            self.ctx_mut().drawings = self.restore_snapshot(next_snapshot);
            self.ctx_mut().selected_drawing = None;
            self.render_drawings();
        }
    }

    // --- Anchor Editing ---
    pub fn start_drag_anchor(&mut self, sx: f32, sy: f32) -> bool {
        let tolerance = 10.0;
        let mut hit_anchor = None; // (anchor_id)
        let mut hit_body = None; // (p1_offset, p2_offset)

        if let Some(idx) = self.ctx().selected_drawing {
            let d = &self.ctx().drawings[idx];
            let (sx1, sy1) = self.world_to_screen_static(d.p1);
            let (sx2, sy2) = self.world_to_screen_static(d.p2);

            // 1. Check Anchors first (Priority)
            let check_anchors = match d.shape {
                DrawingType::Horizontal | DrawingType::Vertical => false,
                _ => true,
            };

            if check_anchors {
                // Check P1
                if ((sx - sx1).powi(2) + (sy - sy1).powi(2)).sqrt() < tolerance {
                    hit_anchor = Some(0);
                }
                // Check P2
                else if ((sx - sx2).powi(2) + (sy - sy2).powi(2)).sqrt() < tolerance {
                    hit_anchor = Some(1);
                }
            }

            // 2. Check Body (Move Mode)
            if hit_anchor.is_none() {
                let dist = match d.shape {
                    DrawingType::Segment => {
                        ChartEngine::dist_point_segment(sx, sy, sx1, sy1, sx2, sy2)
                    }
                    DrawingType::Line => {
                        ChartEngine::dist_point_infinite_line(sx, sy, sx1, sy1, sx2, sy2)
                    }
                    DrawingType::Ray => ChartEngine::dist_point_ray(sx, sy, sx1, sy1, sx2, sy2),
                    DrawingType::Horizontal => (sy - sy2).abs(),
                    DrawingType::Vertical => (sx - sx2).abs(),
                };

                if dist < tolerance {
                    let p_mouse = self.screen_to_world(sx, sy);
                    let off1 = Point {
                        x: d.p1.x - p_mouse.x,
                        y: d.p1.y - p_mouse.y,
                    };
                    let off2 = Point {
                        x: d.p2.x - p_mouse.x,
                        y: d.p2.y - p_mouse.y,
                    };
                    hit_body = Some((off1, off2));
                }
            }
        }

        // Apply changes (mutable borrow)
        if let Some(anchor) = hit_anchor {
            self.save_state();
            self.ctx_mut().drag_active_anchor = Some(anchor);
            return true;
        }

        if let Some((off1, off2)) = hit_body {
            self.save_state();
            self.ctx_mut().drag_active_anchor = Some(2);
            self.ctx_mut().drag_offset_p1 = off1;
            self.ctx_mut().drag_offset_p2 = off2;
            return true;
        }

        false
    }

    pub fn update_drag_anchor(&mut self, sx: f32, sy: f32) {
        // Collect IDs first to avoid borrowing self ctx
        let ids = {
            let ctx = self.ctx();
            (ctx.selected_drawing, ctx.drag_active_anchor)
        };

        if let (Some(idx), Some(mode)) = ids {
            let p_mouse = self.screen_to_world(sx, sy);

            {
                let ctx = self.ctx_mut();
                if mode == 2 {
                    // Move Body: Apply offsets
                    if let Some(d) = ctx.drawings.get_mut(idx) {
                        d.p1 = Point {
                            x: p_mouse.x + ctx.drag_offset_p1.x,
                            y: p_mouse.y + ctx.drag_offset_p1.y,
                        };
                        d.p2 = Point {
                            x: p_mouse.x + ctx.drag_offset_p2.x,
                            y: p_mouse.y + ctx.drag_offset_p2.y,
                        };
                    }
                } else {
                    // Resize Anchor
                    if let Some(d) = ctx.drawings.get_mut(idx) {
                        match d.shape {
                            DrawingType::Horizontal | DrawingType::Vertical => {
                                // No-op
                            }
                            _ => {
                                if mode == 0 {
                                    d.p1 = p_mouse;
                                    d.p1_time = Self::get_time_at_index_ctx(
                                        &ctx.candles,
                                        &ctx.interval,
                                        p_mouse.x,
                                    );
                                } else {
                                    d.p2 = p_mouse;
                                    d.p2_time = Self::get_time_at_index_ctx(
                                        &ctx.candles,
                                        &ctx.interval,
                                        p_mouse.x,
                                    );
                                }
                            }
                        }

                        // If moving the whole body, update both times
                        if mode == 2 {
                            d.p1_time =
                                Self::get_time_at_index_ctx(&ctx.candles, &ctx.interval, d.p1.x);
                            d.p2_time =
                                Self::get_time_at_index_ctx(&ctx.candles, &ctx.interval, d.p2.x);
                        }
                    }
                }
            } // Drop ctx borrow

            self.render_drawings();
        }
    }

    pub fn end_drag_anchor(&mut self) {
        self.ctx_mut().drag_active_anchor = None;
    }

    // --- Hit Testing ---

    pub fn deselect_drawing(&mut self) {
        self.ctx_mut().selected_drawing = None;
        self.ctx_mut().drag_active_anchor = None;
        self.render_drawings();
    }

    pub fn check_click(&mut self, sx: f32, sy: f32) -> bool {
        let tolerance = 10.0; // 10px hit radius
        let mut found = None;

        // Iterate backwards to select "topmost" / most recently drawn
        for (i, d) in self.ctx().drawings.iter().enumerate().rev() {
            let (sx1, sy1) = self.world_to_screen_static(d.p1);
            let (sx2, sy2) = self.world_to_screen_static(d.p2);

            let dist = match d.shape {
                DrawingType::Segment => ChartEngine::dist_point_segment(sx, sy, sx1, sy1, sx2, sy2),
                DrawingType::Line => {
                    ChartEngine::dist_point_infinite_line(sx, sy, sx1, sy1, sx2, sy2)
                }
                DrawingType::Ray => ChartEngine::dist_point_ray(sx, sy, sx1, sy1, sx2, sy2),
                DrawingType::Horizontal => (sy - sy2).abs(), // Horizontal line at sy2
                DrawingType::Vertical => (sx - sx2).abs(),   // Vertical line at sx2
            };

            if dist < tolerance {
                found = Some(i);
                break;
            }
        }

        self.ctx_mut().selected_drawing = found;
        self.render_drawings(); // Re-render to show/hide anchors
        found.is_some()
    }

    // --- Math Helpers ---

    // Cohen-Sutherland Line Clipping
    fn clip_line_segment(
        width: f32,
        height: f32,
        mut x0: f32,
        mut y0: f32,
        mut x1: f32,
        mut y1: f32,
    ) -> Option<(f32, f32, f32, f32)> {
        let xmin = 0.0;
        let ymin = 0.0;
        let xmax = width;
        let ymax = height;

        let compute_out_code = |x: f32, y: f32| -> u8 {
            let mut code = 0;
            if x < xmin {
                code |= 1;
            }
            // Left
            else if x > xmax {
                code |= 2;
            } // Right
            if y < ymin {
                code |= 4;
            }
            // Bottom
            else if y > ymax {
                code |= 8;
            } // Top
            code
        };

        let mut outcode0 = compute_out_code(x0, y0);
        let mut outcode1 = compute_out_code(x1, y1);
        let mut accept = false;

        loop {
            if (outcode0 | outcode1) == 0 {
                accept = true;
                break;
            } else if (outcode0 & outcode1) != 0 {
                break;
            } else {
                let outcode_out = if outcode0 != 0 { outcode0 } else { outcode1 };
                let x;
                let y;

                if (outcode_out & 8) != 0 {
                    x = x0 + (x1 - x0) * (ymax - y0) / (y1 - y0);
                    y = ymax;
                } else if (outcode_out & 4) != 0 {
                    x = x0 + (x1 - x0) * (ymin - y0) / (y1 - y0);
                    y = ymin;
                } else if (outcode_out & 2) != 0 {
                    y = y0 + (y1 - y0) * (xmax - x0) / (x1 - x0);
                    x = xmax;
                } else {
                    y = y0 + (y1 - y0) * (xmin - x0) / (x1 - x0);
                    x = xmin;
                }

                if outcode_out == outcode0 {
                    x0 = x;
                    y0 = y;
                    outcode0 = compute_out_code(x0, y0);
                } else {
                    x1 = x;
                    y1 = y;
                    outcode1 = compute_out_code(x1, y1);
                }
            }
        }

        if accept {
            Some((x0, y0, x1, y1))
        } else {
            None
        }
    }

    fn dist_point_segment(px: f32, py: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
        let l2 = (x1 - x2).powi(2) + (y1 - y2).powi(2);
        if l2 == 0.0 {
            return ((px - x1).powi(2) + (py - y1).powi(2)).sqrt();
        }

        let t = ((px - x1) * (x2 - x1) + (py - y1) * (y2 - y1)) / l2;
        let t = t.max(0.0).min(1.0);

        let proj_x = x1 + t * (x2 - x1);
        let proj_y = y1 + t * (y2 - y1);

        ((px - proj_x).powi(2) + (py - proj_y).powi(2)).sqrt()
    }

    fn dist_point_infinite_line(px: f32, py: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
        let l2 = (x1 - x2).powi(2) + (y1 - y2).powi(2);
        if l2 == 0.0 {
            return ((px - x1).powi(2) + (py - y1).powi(2)).sqrt();
        }

        // t is not clamped
        let t = ((px - x1) * (x2 - x1) + (py - y1) * (y2 - y1)) / l2;

        let proj_x = x1 + t * (x2 - x1);
        let proj_y = y1 + t * (y2 - y1);

        ((px - proj_x).powi(2) + (py - proj_y).powi(2)).sqrt()
    }

    fn dist_point_ray(px: f32, py: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
        let l2 = (x1 - x2).powi(2) + (y1 - y2).powi(2);
        if l2 == 0.0 {
            return ((px - x1).powi(2) + (py - y1).powi(2)).sqrt();
        }

        // t >= 0
        let t = ((px - x1) * (x2 - x1) + (py - y1) * (y2 - y1)) / l2;
        let t = t.max(0.0);

        let proj_x = x1 + t * (x2 - x1);
        let proj_y = y1 + t * (y2 - y1);

        ((px - proj_x).powi(2) + (py - proj_y).powi(2)).sqrt()
    }

    fn world_to_screen_static(&self, p: Point) -> (f32, f32) {
        let sx = (p.x as f32 * self.ctx().scale_x) + self.ctx().offset_x;
        let chart_area_height = self.ctx().height * 0.7;
        let padding_price = 20.0;
        let avail_price_height = chart_area_height - (padding_price * 2.0);
        let price_range = self.ctx().visible_max_price - self.ctx().visible_min_price;
        let range = if price_range == 0.0 { 1.0 } else { price_range };
        let norm = (p.y - self.ctx().visible_min_price) / range;
        let sy = chart_area_height - padding_price - (norm as f32 * avail_price_height);
        (sx, sy)
    }

    pub fn render_drawings(&mut self) {
        let ctx = self.ctx_mut();
        ctx.drawing_buffer.clear();

        let cmd_move = 1.0;
        let cmd_line = 2.0;
        let cmd_circle = 3.0;

        // Copy View State to locals to avoid borrowing
        let width = ctx.width;
        let height = ctx.height;
        let offset_x = ctx.offset_x;
        let scale_x = ctx.scale_x;
        let min_price = ctx.visible_min_price;
        let max_price = ctx.visible_max_price;

        // Layout constants
        let chart_area_height = height * 0.7;
        let padding_price = 20.0;
        let avail_price_height = chart_area_height - (padding_price * 2.0);
        let price_range = max_price - min_price;
        let range = if price_range == 0.0 { 1.0 } else { price_range };

        // Helper Closure
        let world_to_screen_local = |p: Point| -> (f32, f32) {
            let sx = (p.x as f32 * scale_x) + offset_x;
            let norm = (p.y - min_price) / range;
            let sy = chart_area_height - padding_price - (norm as f32 * avail_price_height);
            (sx, sy)
        };

        // 1. Process stored drawings
        for (i, d) in ctx.drawings.iter().enumerate() {
            let obj = d.clone();
            let (raw_sx1, raw_sy1) = world_to_screen_local(obj.p1);
            let (raw_sx2, raw_sy2) = world_to_screen_local(obj.p2);

            let mut final_sx1 = raw_sx1;
            let mut final_sy1 = raw_sy1;
            let mut final_sx2 = raw_sx2;
            let mut final_sy2 = raw_sy2;
            let mut should_draw = true;

            // Apply Clipping for Segments
            if obj.shape == DrawingType::Segment {
                if let Some((cx1, cy1, cx2, cy2)) = ChartEngine::clip_line_segment(
                    width, height, raw_sx1, raw_sy1, raw_sx2, raw_sy2,
                ) {
                    final_sx1 = cx1;
                    final_sy1 = cy1;
                    final_sx2 = cx2;
                    final_sy2 = cy2;
                } else {
                    should_draw = false; // Completely off-screen
                }
            }

            if should_draw {
                // Push Style
                let (r, g, b) = ChartEngine::parse_hex_color(&obj.color);
                ChartEngine::push_style_command(&mut ctx.drawing_buffer, obj.width, r, g, b);

                ChartEngine::push_drawing_commands(
                    &mut ctx.drawing_buffer,
                    obj.shape,
                    final_sx1,
                    final_sy1,
                    final_sx2,
                    final_sy2,
                    width as f32, // Pass width and height for clipping
                    height as f32,
                    cmd_move,
                    cmd_line,
                );
            }

            // Check selection
            if Some(i) == ctx.selected_drawing {
                let (r, g, b) = ChartEngine::parse_hex_color(&obj.color);
                ChartEngine::push_anchor_commands(
                    &mut ctx.drawing_buffer,
                    obj.shape,
                    raw_sx1,
                    raw_sy1,
                    raw_sx2,
                    raw_sy2,
                    cmd_circle, // Pass cmd_circle
                    r,
                    g,
                    b,
                );
            }
        }

        // 2. Process current drawing
        if let Some(obj) = &ctx.current_drawing {
            let (raw_sx1, raw_sy1) = world_to_screen_local(obj.p1);
            let (raw_sx2, raw_sy2) = world_to_screen_local(obj.p2);

            let mut final_sx1 = raw_sx1;
            let mut final_sy1 = raw_sy1;
            let mut final_sx2 = raw_sx2;
            let mut final_sy2 = raw_sy2;

            // Clamp infinite lines logic (dup)
            if obj.shape == DrawingType::Horizontal {
                final_sx1 = 0.0;
                final_sx2 = width;
                final_sy2 = final_sy1;
            } else if obj.shape == DrawingType::Vertical {
                final_sy1 = 0.0;
                final_sy2 = chart_area_height;
                final_sx2 = final_sx1;
            } else if obj.shape == DrawingType::Ray {
                let dx = raw_sx2 - raw_sx1;
                let dy = raw_sy2 - raw_sy1;
                final_sx2 = raw_sx1 + (dx * 1000.0);
                final_sy2 = raw_sy1 + (dy * 1000.0);
            }

            let (r, g, b) = ChartEngine::parse_hex_color(&obj.color);
            ChartEngine::push_style_command(&mut ctx.drawing_buffer, obj.width, r, g, b);
            ChartEngine::push_drawing_commands(
                &mut ctx.drawing_buffer,
                obj.shape,
                final_sx1,
                final_sy1,
                final_sx2,
                final_sy2,
                width, // Pass width and height for clipping
                height,
                cmd_move,
                cmd_line,
            );
            ChartEngine::push_anchor_commands(
                &mut ctx.drawing_buffer,
                obj.shape,
                raw_sx1,
                raw_sy1,
                raw_sx2,
                raw_sy2,
                cmd_circle, // Pass cmd_circle
                r,
                g,
                b,
            );
        }
    }

    pub fn remove_selected_drawing(&mut self) {
        if let Some(index) = self.ctx().selected_drawing {
            if index < self.ctx().drawings.len() {
                self.save_state(); // Undoable
                {
                    let ctx = self.ctx_mut();
                    ctx.drawings.remove(index);
                    ctx.selected_drawing = None;
                    ctx.drag_active_anchor = None;
                }
                self.render_drawings();
            }
        }
    }

    // --- Drawing Properties API ---

    pub fn set_tool_color(&mut self, color: String) {
        self.active_color = color;
    }

    pub fn set_tool_width(&mut self, width: f32) {
        self.active_width = width;
    }

    pub fn get_selected_props(&self) -> JsValue {
        if let Some(idx) = self.ctx().selected_drawing {
            if let Some(d) = self.ctx().drawings.get(idx) {
                let props = DrawingProps {
                    color: d.color.clone(),
                    line_width: d.width,
                };
                return serde_wasm_bindgen::to_value(&props).unwrap_or(JsValue::NULL);
            }
        }
        JsValue::NULL
    }

    pub fn update_selected_props(&mut self, val: JsValue) {
        if let Some(idx) = self.ctx().selected_drawing {
            if let Ok(props) = serde_wasm_bindgen::from_value::<DrawingProps>(val) {
                self.save_state(); // Undoable
                if let Some(d) = self.ctx_mut().drawings.get_mut(idx) {
                    d.color = props.color;
                    d.width = props.line_width;
                }
                self.render_drawings();
            }
        }
    }

    // Helpers
    fn parse_hex_color(hex: &str) -> (f32, f32, f32) {
        let hex = hex.trim_start_matches('#');
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f32;
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f32;
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f32;
            (r, g, b)
        } else {
            (0.0, 255.0, 255.0) // Default Cyan-ish on error
        }
    }

    fn push_style_command(buffer: &mut Vec<f32>, width: f32, r: f32, g: f32, b: f32) {
        buffer.push(4.0); // CMD_STYLE
        buffer.push(width);
        buffer.push(r);
        buffer.push(g);
        buffer.push(b);
    }

    fn push_anchor_commands(
        buffer: &mut Vec<f32>,
        shape: DrawingType,
        sx1: f32,
        sy1: f32,
        sx2: f32,
        sy2: f32,
        cmd_circle: f32, // 3.0
        r: f32,
        g: f32,
        b: f32,
    ) {
        match shape {
            DrawingType::Horizontal | DrawingType::Vertical => {
                // Do not draw anchors for infinite lines (or maybe we do? current logic says returns)
                // Wait, logic in file (from memory) might be empty?
                // Let's verify content first.
                return;
            }
            _ => {
                buffer.push(cmd_circle);
                buffer.push(sx1);
                buffer.push(sy1);
                buffer.push(r);
                buffer.push(g);
                buffer.push(b);

                buffer.push(cmd_circle);
                buffer.push(sx2);
                buffer.push(sy2);
                buffer.push(r);
                buffer.push(g);
                buffer.push(b);
            }
        }
    }

    // Static helper to avoid self borrow issues
    fn push_drawing_commands(
        buffer: &mut Vec<f32>,
        shape: DrawingType,
        sx1: f32,
        sy1: f32,
        sx2: f32,
        sy2: f32,
        width: f32,
        height: f32,
        cmd_move: f32,
        cmd_line: f32,
    ) {
        match shape {
            DrawingType::Segment => {
                buffer.push(cmd_move);
                buffer.push(sx1);
                buffer.push(sy1);
                buffer.push(cmd_line);
                buffer.push(sx2);
                buffer.push(sy2);
            }
            DrawingType::Horizontal => {
                buffer.push(cmd_move);
                buffer.push(0.0);
                buffer.push(sy2); // Use sy2 (follow mouse/end point)
                buffer.push(cmd_line);
                buffer.push(width);
                buffer.push(sy2);
            }
            DrawingType::Vertical => {
                buffer.push(cmd_move);
                buffer.push(sx2); // Use sx2 (follow mouse/end point)
                buffer.push(0.0);
                buffer.push(cmd_line);
                buffer.push(sx2);
                buffer.push(height);
            }
            DrawingType::Line | DrawingType::Ray => {
                let dx = sx2 - sx1;
                let dy = sy2 - sy1;

                if dx.abs() < 0.001 {
                    buffer.push(cmd_move);
                    buffer.push(sx1);
                    buffer.push(0.0);
                    buffer.push(cmd_line);
                    buffer.push(sx1);
                    buffer.push(height);
                } else {
                    let m = dy / dx;
                    let b = sy1 - (m * sx1);

                    let y_start = b;
                    let y_end = (m * width) + b;

                    if shape == DrawingType::Line {
                        buffer.push(cmd_move);
                        buffer.push(0.0);
                        buffer.push(y_start);
                        buffer.push(cmd_line);
                        buffer.push(width);
                        buffer.push(y_end);
                    } else {
                        // Ray
                        buffer.push(cmd_move);
                        buffer.push(sx1);
                        buffer.push(sy1);
                        buffer.push(cmd_line);

                        if sx2 >= sx1 {
                            buffer.push(width);
                            buffer.push(y_end);
                        } else {
                            buffer.push(0.0);
                            buffer.push(y_start);
                        }
                    }
                }
            }
        }
    }

    // --- Free Scale API ---
    pub fn set_auto_scale(&mut self, auto: bool) {
        self.ctx_mut().is_auto_scale = auto;
        if auto {
            self.update_view();
        }
    }

    pub fn get_auto_scale(&self) -> bool {
        self.ctx().is_auto_scale
    }

    pub fn pan_y(&mut self, dy_pixels: f32) {
        // dy is pixels. Positive dy (drag down) should move view UP (price increases at top?)
        // Wait, normally dragging down moves the content down (view moves up).
        // Logic:
        // Canvas: 0 at top. Drag Down -> dy > 0.
        // If I drag down, I want to see higher prices? No, if I drag the "Paper" down, I see what's above.
        // So higher prices shift down. Map moves with Mouse.
        // Let's check X pan: offset_x += dx.
        // offset_x pushes the world right.

        // For Y:
        // We aren't using an offset_y, we modify min/max directly.
        // Range = max - min.
        // Height = chart_height (avail).
        // scale_y = height / Range.
        // delta_price = dy / scale_y.
        // delta_price = dy * (Range / Height).

        {
            let ctx = self.ctx_mut();
            let chart_area_height = ctx.height * 0.7 - 40.0; // approx avail height (minus padding)
            let price_range = ctx.visible_max_price - ctx.visible_min_price;

            if chart_area_height <= 0.0 || price_range <= 0.0 {
                return;
            }

            let delta_price = (dy_pixels as f64) * (price_range / chart_area_height as f64);

            // So adding delta_price (positive) should INCREASE the view range?
            // Coordinate system: Y=0 at Top. Price Max at Top.
            // Drag Down (dy > 0) -> moving top pixels down.
            // So we want the view to shift UP (prices increase).
            // Wait, if I drag the chart DOWN, I expect to see Higher prices coming into view from top?
            // No, in TradingView: Dragging Scale Down -> View moves Up (Prices Increase).
            // Let's try: manual_min += delta, manual_max += delta.
            // If dy > 0 (down), prices increase.

            ctx.manual_min_price += delta_price;
            ctx.manual_max_price += delta_price;

            // Force manual mode if panning
            ctx.is_auto_scale = false;
        } // End scope
        self.update_view();
    }

    pub fn zoom_y(&mut self, factor: f32) {
        // Zoom around center
        {
            let ctx = self.ctx_mut();
            let center = (ctx.manual_min_price + ctx.manual_max_price) / 2.0;
            let range = ctx.manual_max_price - ctx.manual_min_price;
            let new_range = range * factor as f64;

            let half = new_range / 2.0;
            ctx.manual_min_price = center - half;
            ctx.manual_max_price = center + half;

            // Force manual mode
            ctx.is_auto_scale = false;
        }
        self.update_view();
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        {
            let ctx = self.ctx_mut();
            ctx.width = width;
            ctx.height = height;
        }
        self.update_view();
    }

    pub fn pan(&mut self, dx: f32) {
        self.ctx_mut().offset_x += dx;
        self.update_view();
    }

    pub fn zoom(&mut self, factor: f32, center_x: f32) {
        // center_x is mouse position in pixels
        // world_x = (center_x - offset_x) / scale_x
        // new_scale = scale_x * factor
        // new_offset = center_x - (world_x * new_scale)

        {
            let ctx = self.ctx_mut();
            let new_scale = (ctx.scale_x * factor).max(1.0).min(200.0);

            // Pivot math
            let world_x = (center_x - ctx.offset_x) / ctx.scale_x;
            ctx.offset_x = center_x - (world_x * new_scale);
            ctx.scale_x = new_scale;
        }
        self.update_view();
    }

    pub fn scroll_to_end(&mut self) {
        if self.ctx().candles.is_empty() {
            {
                let ctx = self.ctx_mut();
                ctx.offset_x = 0.0;
                ctx.scale_x = 10.0; // Default scale
            }
            return;
        }

        {
            let ctx = self.ctx_mut();
            let total_content_width = ctx.candles.len() as f32 * ctx.scale_x;
            // Align end of content to info view width, minus some padding (e.g. 5 candles)
            let padding_right = ctx.scale_x * 5.0;

            ctx.offset_x = ctx.width - total_content_width - padding_right;
        }
        self.update_view();
    }

    pub fn reset_view(&mut self, chart_id: u32) {
        if let Some(ctx) = self.charts.get_mut(&chart_id) {
            let max_candles = (ctx.width / ctx.scale_x) as usize;
            let total = ctx.candles.len();

            let _start = if total > max_candles {
                total - max_candles
            } else {
                0
            };

            // Calculate pixels for start
            // index = -offset / scale
            // offset = -index * scale
            // For Right Align: we want last candle at width - padding.
            // But simpler reset: Just put end at width.

            let total_px = total as f32 * ctx.scale_x;
            if total_px < ctx.width {
                ctx.offset_x = 0.0;
            } else {
                ctx.offset_x = ctx.width - total_px - (ctx.scale_x * 5.0); // 5 bars padding right
            }
            ctx.is_auto_scale = true;
        }

        if self.active_chart_id == Some(chart_id) {
            self.update_view();
        }
    }

    pub fn update_view(&mut self) {
        // Always operate on active context for view rendering
        if let Some(id) = self.active_chart_id {
            if let Some(ctx) = self.charts.get_mut(&id) {
                // ... Existing Logic for ctx ...
                ctx.render_buffer.clear();

                if ctx.candles.is_empty() {
                    ctx.visible_min_price = 0.0;
                    ctx.visible_max_price = 1.0;
                    return;
                }

                let len = ctx.candles.len();

                // Windowing Math
                let start_index = ((-ctx.offset_x) / ctx.scale_x).floor() as isize;
                let end_index = ((ctx.width - ctx.offset_x) / ctx.scale_x).ceil() as isize;

                // Clamp to valid range
                let start = start_index.max(0) as usize;
                let end = end_index.min(len as isize).max(0) as usize;

                // Store visible state
                ctx.visible_start_index = start;
                ctx.visible_end_index = end;

                // Guard against invalid range
                if start >= end || start >= len {
                    if ctx.visible_max_price == ctx.visible_min_price {
                        ctx.visible_min_price = 0.0;
                        ctx.visible_max_price = 1.0;
                    }
                    return;
                }

                // Calculate Scale using a slice
                // We cannot iterate slice while holding ctx mutably if we want to modify ctx.
                // But we are only iterating slice to calc min/max.
                // We can do that, extract min/max, then use them.

                let mut auto_min = f64::MAX;
                let mut auto_max = f64::MIN;
                let mut max_vol = f64::MIN;

                for i in start..end {
                    let c = &ctx.candles[i];
                    if c.low < auto_min {
                        auto_min = c.low;
                    }
                    if c.high > auto_max {
                        auto_max = c.high;
                    }
                    if c.volume > max_vol {
                        max_vol = c.volume;
                    }
                }

                if ctx.is_auto_scale {
                    if auto_max == auto_min {
                        auto_min -= 1.0;
                        auto_max += 1.0;
                    }
                    ctx.visible_min_price = auto_min;
                    ctx.visible_max_price = auto_max;
                    ctx.manual_min_price = auto_min;
                    ctx.manual_max_price = auto_max;
                } else {
                    ctx.visible_min_price = ctx.manual_min_price;
                    ctx.visible_max_price = ctx.manual_max_price;
                }

                let min_price = ctx.visible_min_price;
                let max_price = ctx.visible_max_price;
                let price_range = max_price - min_price;
                let price_range = if price_range == 0.0 { 1.0 } else { price_range };

                let vol_range = if max_vol == 0.0 { 1.0 } else { max_vol };

                // Layout
                let chart_area_height = ctx.height * 0.7;
                let padding_price = 20.0;
                let avail_price_height = chart_area_height - (padding_price * 2.0);
                let vol_area_height = ctx.height * 0.3;
                let padding_vol = 10.0;
                let avail_vol_height = vol_area_height - padding_vol;
                let height = ctx.height;

                ctx.render_buffer.reserve((end - start) * 6);

                // We iteration using index to make borrow checker happy with disjoint fields
                for i in start..end {
                    let c = &ctx.candles[i];
                    let real_idx = i;
                    let x = (real_idx as f32 * ctx.scale_x) + ctx.offset_x;

                    let norm_h = (c.high - min_price) / price_range;
                    let y_high =
                        chart_area_height - padding_price - (norm_h as f32 * avail_price_height);

                    let norm_l = (c.low - min_price) / price_range;
                    let y_low =
                        chart_area_height - padding_price - (norm_l as f32 * avail_price_height);

                    let norm_o = (c.open - min_price) / price_range;
                    let y_open =
                        chart_area_height - padding_price - (norm_o as f32 * avail_price_height);

                    let norm_c = (c.close - min_price) / price_range;
                    let y_close =
                        chart_area_height - padding_price - (norm_c as f32 * avail_price_height);

                    let norm_vol = c.volume / vol_range;
                    let vol_bar_height = norm_vol as f32 * avail_vol_height;
                    let y_vol_top = height - vol_bar_height;

                    ctx.render_buffer.push(x);
                    ctx.render_buffer.push(y_high);
                    ctx.render_buffer.push(y_low);
                    ctx.render_buffer.push(y_open);
                    ctx.render_buffer.push(y_close);
                    ctx.render_buffer.push(y_vol_top);
                }
            } // End of scope for mutable borrow of ctx

            self.render_drawings();
            self.render_ma_lines();
            self.render_bb_bands();
        }
    }

    pub fn get_view_state(&self) -> Vec<f64> {
        vec![
            self.ctx().visible_min_price,
            self.ctx().visible_max_price,
            self.ctx().visible_start_index as f64,
            self.ctx().visible_end_index as f64,
        ]
    }

    pub fn get_render_buffer_ptr(&self) -> *const f32 {
        self.ctx().render_buffer.as_ptr()
    }

    pub fn get_render_buffer_len(&self) -> usize {
        self.ctx().render_buffer.len()
    }

    pub fn get_drawing_buffer_ptr(&self) -> *const f32 {
        self.ctx().drawing_buffer.as_ptr()
    }

    pub fn get_drawing_buffer_len(&self) -> usize {
        self.ctx().drawing_buffer.len()
    }

    // --- Moving Average System ---

    // --- Moving Average System ---

    pub fn add_ma(&mut self) -> u32 {
        let id = self.ctx_mut().next_ma_id;
        self.ctx_mut().next_ma_id += 1;

        // Default Settings with Color Cycling
        let colors = [
            "#FFA500", // Orange
            "#00FF00", // Lime
            "#FF00FF", // Magenta
            "#00FFFF", // Cyan
            "#FFFF00", // Yellow
            "#FF0000", // Red
            "#0000FF", // Blue
            "#FFFFFF", // White
        ];

        // Find the first color not currently used by any active MA
        let mut chosen_color = colors[0];
        let mut found_unused = false;

        for &c in &colors {
            let is_used = self
                .ctx_mut()
                .ma_lines
                .iter()
                .any(|ma| ma.settings.color == c);
            if !is_used {
                chosen_color = c;
                found_unused = true;
                break;
            }
        }

        // Fallback: If all palette colors are used, use modulo logic to cycle
        if !found_unused {
            let idx = self.ctx_mut().ma_lines.len() % colors.len();
            chosen_color = colors[idx];
        }

        let default_settings = MASettings {
            period: 20,
            source: "close".to_string(),
            method: "SMA".to_string(),
            offset: 0,
            color: chosen_color.to_string(),
            line_width: 2.0,
            visible: true,
            active: true,
        };

        let mut ma = MovingAverage::new(id, default_settings);
        // Calculate immediately with current data
        if !self.ctx_mut().candles.is_empty() {
            ma.calculate(&self.ctx_mut().candles);
        }

        self.ctx_mut().ma_lines.push(ma);
        self.update_view();

        id
    }

    pub fn remove_ma(&mut self, id: u32) {
        if let Some(pos) = self.ctx_mut().ma_lines.iter().position(|x| x.id == id) {
            self.ctx_mut().ma_lines.remove(pos);
            self.update_view();
        }
    }

    pub fn get_clicked_ma(&self, x: f32, y: f32) -> Option<u32> {
        let chart_area_height = self.ctx().height * 0.7; // Match render_ma_lines
        let padding_price = 20.0;
        let avail_price_height = chart_area_height - (padding_price * 2.0);
        let min_price = self.ctx().visible_min_price;
        let max_price = self.ctx().visible_max_price;
        let range = max_price - min_price;

        if range <= 0.0 {
            return None;
        }

        // Convert screen X to data index
        // x = (index * scale) + offset
        // index = (x - offset) / scale
        let float_index = (x - self.ctx().offset_x) / self.ctx().scale_x;
        let index = float_index.round() as isize;

        if index < 0 || index >= self.ctx().candles.len() as isize {
            return None;
        }

        let idx = index as usize;
        let threshold = 5.0; // 5 pixels tolerance

        // Iterate MAs
        for ma in &self.ctx().ma_lines {
            if !ma.settings.visible {
                continue;
            }
            // Get value at index (handle offset)
            let offset = ma.settings.offset;
            let src_idx = idx as isize - offset as isize;

            if src_idx >= 0 && src_idx < ma.cached_values.len() as isize {
                let val = ma.cached_values[src_idx as usize];
                if !val.is_nan() {
                    // Calculate Screen Y
                    let norm = (val - min_price) / range;
                    let ma_y =
                        chart_area_height - padding_price - (norm as f32 * avail_price_height);

                    if (ma_y - y).abs() < threshold {
                        return Some(ma.id);
                    }
                }
            }
        }

        None
    }

    // Better implementation of update_ma:
    pub fn update_ma(&mut self, id: u32, val: JsValue) {
        if let Ok(new_settings) = serde_wasm_bindgen::from_value::<MASettings>(val) {
            let mut target_index = None;
            let mut needs_recalc = false;

            // 1. Find index and check if recalc needed
            if let Some(pos) = self.ctx_mut().ma_lines.iter().position(|x| x.id == id) {
                target_index = Some(pos);
                let old = &self.ctx_mut().ma_lines[pos].settings;
                needs_recalc = old.period != new_settings.period
                    || old.source != new_settings.source
                    || old.method != new_settings.method;
            }

            // 2. Apply updates
            if let Some(idx) = target_index {
                self.ctx_mut().ma_lines[idx].settings = new_settings;

                if needs_recalc {
                    // 3. Recalculate using full 'self' access safely
                    // (We are not holding borrow of ma_lines anymore)
                    let ctx = self.ctx_mut();
                    ctx.ma_lines[idx].calculate(&ctx.candles);
                }
                self.update_view();
            }
        }
    }

    pub fn get_all_mas(&self) -> JsValue {
        // Return Array of objects: { id, settings }
        #[derive(Serialize)]
        struct MaExport {
            id: u32,
            settings: MASettings,
        }

        let list: Vec<MaExport> = self
            .ctx()
            .ma_lines
            .iter()
            .map(|m| MaExport {
                id: m.id,
                settings: m.settings.clone(),
            })
            .collect();

        serde_wasm_bindgen::to_value(&list).unwrap_or(JsValue::NULL)
    }

    pub fn render_ma_lines(&mut self) {
        let ctx = self.ctx_mut();
        let start = ctx.visible_start_index;
        let end = ctx.visible_end_index;

        let min_price = ctx.visible_min_price;
        let max_price = ctx.visible_max_price;
        let price_range = max_price - min_price;
        let range = if price_range == 0.0 { 1.0 } else { price_range };

        let chart_area_height = ctx.height * 0.7;
        let padding_price = 20.0;
        let avail_price_height = chart_area_height - (padding_price * 2.0);
        let scale_x = ctx.scale_x;
        let offset_x = ctx.offset_x;

        for ma in &mut ctx.ma_lines {
            ma.render_buffer.clear();
            if !ma.settings.visible {
                continue;
            }

            let offset = ma.settings.offset;
            let mut is_drawing = false;

            for i in start..end {
                let src_idx = i as isize - offset as isize;

                if src_idx < 0 || src_idx >= ma.cached_values.len() as isize {
                    is_drawing = false;
                    continue;
                }

                let val = ma.cached_values[src_idx as usize];
                if val.is_nan() {
                    is_drawing = false;
                    continue;
                }

                let x = (i as f32 * scale_x) + offset_x;
                let norm = (val - min_price) / range;
                let y = chart_area_height - padding_price - (norm as f32 * avail_price_height);

                if !is_drawing {
                    ma.render_buffer.push(1.0); // MoveTo
                    ma.render_buffer.push(x);
                    ma.render_buffer.push(y);
                    is_drawing = true;
                } else {
                    ma.render_buffer.push(2.0); // LineTo
                    ma.render_buffer.push(x);
                    ma.render_buffer.push(y);
                }
            }
        }
    }

    pub fn get_ma_count(&self) -> usize {
        self.ctx().ma_lines.len()
    }

    pub fn get_ma_buffer_ptr(&self, index: usize) -> *const f32 {
        self.ctx().ma_lines[index].render_buffer.as_ptr()
    }

    pub fn get_ma_buffer_len(&self, index: usize) -> usize {
        self.ctx().ma_lines[index].render_buffer.len()
    }

    pub fn get_ma_color(&self, index: usize) -> String {
        self.ctx().ma_lines[index].settings.color.clone()
    }

    pub fn get_ma_width(&self, index: usize) -> f32 {
        self.ctx().ma_lines[index].settings.line_width
    }
}

// Logic for MovingAverage
impl MovingAverage {
    pub fn calculate(&mut self, candles: &[Candle]) {
        self.cached_values.clear();
        let period = self.settings.period;
        let len = candles.len();

        if period == 0 || period > len {
            self.cached_values.resize(len, f64::NAN);
            return;
        }

        self.cached_values.resize(len, f64::NAN);

        let ma_type = match self.settings.method.as_str() {
            "EMA" | "ema" => MaType::EMA,
            _ => MaType::SMA,
        };
        let ma_source = match self.settings.source.as_str() {
            "Open" | "open" => MaSource::Open,
            "High" | "high" => MaSource::High,
            "Low" | "low" => MaSource::Low,
            "HL2" | "hl2" => MaSource::HL2,
            "HLC3" | "hlc3" => MaSource::HLC3,
            _ => MaSource::Close,
        };

        match ma_type {
            MaType::SMA => {
                let mut sum = 0.0;
                for i in 0..len {
                    let price = MovingAverage::get_price(&candles[i], ma_source);
                    sum += price;

                    if i >= period {
                        let out_price = MovingAverage::get_price(&candles[i - period], ma_source);
                        sum -= out_price;
                    }

                    if i >= period - 1 {
                        self.cached_values[i] = sum / period as f64;
                    }
                }
            }
            MaType::EMA => {
                let k = 2.0 / (period as f64 + 1.0);

                // Initialize with SMA
                let mut sum = 0.0;
                for i in 0..period {
                    sum += MovingAverage::get_price(&candles[i], ma_source);
                }

                if period <= len {
                    let mut ema = sum / period as f64;
                    self.cached_values[period - 1] = ema;

                    for i in period..len {
                        let price = MovingAverage::get_price(&candles[i], ma_source);
                        ema = price * k + ema * (1.0 - k);
                        self.cached_values[i] = ema;
                    }
                }
            }
        }
    }

    fn get_price(c: &Candle, source: MaSource) -> f64 {
        match source {
            MaSource::Close => c.close,
            MaSource::Open => c.open,
            MaSource::High => c.high,
            MaSource::Low => c.low,
            MaSource::HL2 => (c.high + c.low) / 2.0,
            MaSource::HLC3 => (c.high + c.low + c.close) / 3.0,
        }
    }
}

// --- Bollinger Bands System ---
#[wasm_bindgen]
impl ChartEngine {
    pub fn update_bb_settings(&mut self, index: usize, val: JsValue) {
        if let Ok(new_settings) = serde_wasm_bindgen::from_value::<BBSettings>(val) {
            while self.ctx_mut().bb_bands.len() <= index {
                let id = self.ctx_mut().next_bb_id;
                self.ctx_mut().next_bb_id += 1;
                self.ctx_mut()
                    .bb_bands
                    .push(BollingerBand::new(id, new_settings.clone()));
            }

            let bb = &mut self.ctx_mut().bb_bands[index];
            let old_settings = &bb.settings;

            let needs_recalc = old_settings.period != new_settings.period
                || old_settings.source != new_settings.source
                || old_settings.multiplier != new_settings.multiplier;

            bb.settings = new_settings;

            if needs_recalc {
                self.recalculate_bb(index);
            }

            self.update_view();
        }
    }

    fn recalculate_bb(&mut self, index: usize) {
        let ctx = self.ctx_mut();
        if index >= ctx.bb_bands.len() {
            return;
        }
        // Disjoint borrow: bb_bands (mut), candles (immut)
        ctx.bb_bands[index].calculate(&ctx.candles);
    }

    pub fn render_bb_bands(&mut self) {
        let ctx = self.ctx_mut();
        let start = ctx.visible_start_index;
        let end = ctx.visible_end_index;

        let min_price = ctx.visible_min_price;
        let max_price = ctx.visible_max_price;
        let price_range = max_price - min_price;
        let range = if price_range == 0.0 { 1.0 } else { price_range };

        let chart_area_height = ctx.height * 0.7;
        let padding_price = 20.0;
        let avail_price_height = chart_area_height - (padding_price * 2.0);
        let scale_x = ctx.scale_x;
        let offset_x = ctx.offset_x;

        for (_, bb) in ctx.bb_bands.iter_mut().enumerate() {
            bb.render_buffer.clear();
            if !bb.settings.visible {
                continue;
            }

            // Check if we have data
            if bb.upper.len() == 0 {
                continue;
            }

            // 1. Upper Band L->R
            let mut points_u = Vec::new(); // Temporary storage to build loop
                                           // 2. Lower Band R->L
            let mut points_l = Vec::new();

            // We use simple iteration.
            // In render buffer, we will store [x, y] sequentially for the polygon.
            // Polygon = Upper(start..end) + Lower(end..start)

            for i in start..end {
                if i >= bb.upper.len() {
                    break;
                }

                let x = (i as f32 * scale_x) + offset_x;

                // Upper
                let val_u = bb.upper[i];
                if !val_u.is_nan() {
                    let norm_u = (val_u - min_price) / range;
                    let y_u =
                        chart_area_height - padding_price - (norm_u as f32 * avail_price_height);
                    points_u.push((x, y_u));
                }

                // Lower
                let val_l = bb.lower[i];
                if !val_l.is_nan() {
                    let norm_l = (val_l - min_price) / range;
                    let y_l =
                        chart_area_height - padding_price - (norm_l as f32 * avail_price_height);
                    points_l.push((x, y_l));
                }
            }

            // Construct Render Buffer: x, y, x, y ...
            // Forward Upper
            for (x, y) in &points_u {
                bb.render_buffer.push(*x);
                bb.render_buffer.push(*y);
            }
            // Backward Lower
            for (x, y) in points_l.iter().rev() {
                bb.render_buffer.push(*x);
                bb.render_buffer.push(*y);
            }
        }
    }

    pub fn get_bb_count(&self) -> usize {
        self.ctx().bb_bands.len()
    }

    pub fn get_bb_buffer_ptr(&self, index: usize) -> *const f32 {
        self.ctx().bb_bands[index].render_buffer.as_ptr()
    }

    pub fn get_bb_buffer_len(&self, index: usize) -> usize {
        self.ctx().bb_bands[index].render_buffer.len()
    }

    pub fn get_active_bb_settings(&self) -> JsValue {
        if let Some(bb) = self.ctx().bb_bands.first() {
            serde_wasm_bindgen::to_value(&bb.settings).unwrap_or(JsValue::NULL)
        } else {
            JsValue::NULL
        }
    }

    pub fn get_bb_color(&self, index: usize) -> String {
        self.ctx().bb_bands[index].settings.color.clone()
    }

    pub fn get_active_chart_status(&self) -> JsValue {
        #[derive(Serialize)]
        struct ChartStatus {
            is_auto_scale: bool,
            offset_x: f32,
            scale_x: f32,
            manual_min_price: f64,
            manual_max_price: f64,
        }

        let ctx = self.ctx();
        let status = ChartStatus {
            is_auto_scale: ctx.is_auto_scale,
            offset_x: ctx.offset_x,
            scale_x: ctx.scale_x,
            manual_min_price: ctx.manual_min_price,
            manual_max_price: ctx.manual_max_price,
        };
        serde_wasm_bindgen::to_value(&status).unwrap_or(JsValue::NULL)
    }
}

// Logic for Bollinger Band
impl BollingerBand {
    pub fn calculate(&mut self, candles: &[Candle]) {
        self.middle.clear();
        self.upper.clear();
        self.lower.clear();

        let period = self.settings.period;
        let len = candles.len();

        // Resize
        self.middle.resize(len, f64::NAN);
        self.upper.resize(len, f64::NAN);
        self.lower.resize(len, f64::NAN);

        if period == 0 || period > len {
            return;
        }

        // Welford's or Sum of Squares sliding window
        // Sum of Squares approach is good for sliding window.
        // Var = (SumSq / N) - (Mean)^2
        // Std = Sqrt(Var)

        let mut sum = 0.0;
        let mut sum_sq = 0.0;

        let ma_source = match self.settings.source.as_str() {
            "Open" | "open" => MaSource::Open,
            "High" | "high" => MaSource::High,
            "Low" | "low" => MaSource::Low,
            "HL2" | "hl2" => MaSource::HL2,
            "HLC3" | "hlc3" => MaSource::HLC3,
            _ => MaSource::Close,
        };

        for i in 0..len {
            let price = MovingAverage::get_price(&candles[i], ma_source);
            sum += price;
            sum_sq += price * price;

            if i >= period {
                let out_price = MovingAverage::get_price(&candles[i - period], ma_source);
                sum -= out_price;
                sum_sq -= out_price * out_price;
            }

            if i >= period - 1 {
                let mean = sum / period as f64;
                let variance = (sum_sq / period as f64) - (mean * mean);
                // Clamp variance to 0.0 to avoid sqrt(negative) due to float precision
                let std_dev = variance.max(0.0).sqrt();

                self.middle[i] = mean;
                self.upper[i] = mean + (std_dev * self.settings.multiplier);
                self.lower[i] = mean - (std_dev * self.settings.multiplier);
            }
        }
    }
}
