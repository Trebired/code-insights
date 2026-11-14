use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use crate::scan::Phase;

const EMIT_EVERY_MS: u64 = 100;

pub fn emit_line(value: &serde_json::Value) {
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    let _ = writeln!(lock, "{}", value);
    let _ = lock.flush();
}

struct Current {
    phase: Option<Phase>,
    phase_started: Instant,
    total: u64,
}

pub struct Reporter {
    current: Mutex<Current>,
    done: AtomicU64,
    last_emit_ms: AtomicU64,
    last_percent: AtomicU64,
    phases: Vec<Phase>,
    started: Instant,
}

impl Reporter {
    pub fn new(phases: &[Phase]) -> Self {
        Reporter {
            current: Mutex::new(Current { phase: None, phase_started: Instant::now(), total: 0 }),
            done: AtomicU64::new(0),
            last_emit_ms: AtomicU64::new(0),
            last_percent: AtomicU64::new(0),
            phases: phases.to_vec(),
            started: Instant::now(),
        }
    }

    fn weight_total(&self) -> f64 {
        self.phases.iter().map(|phase| phase.weight()).sum::<f64>().max(1.0)
    }

    fn weight_before(&self, phase: Phase) -> f64 {
        self.phases.iter().take_while(|item| **item != phase).map(|item| item.weight()).sum()
    }

    fn overall_percent(&self, phase: Phase, fraction: f64) -> f64 {
        let value = (self.weight_before(phase) + phase.weight() * fraction.clamp(0.0, 1.0)) / self.weight_total();
        (value * 100.0).clamp(0.0, 99.0)
    }

    pub fn start(&self, phase: Phase, total: u64) {
        let mut current = self.current.lock().unwrap();
        current.phase = Some(phase);
        current.phase_started = Instant::now();
        current.total = total;
        self.done.store(0, Ordering::SeqCst);
        drop(current);
        emit_line(&serde_json::json!({ "type": "phase", "phase": phase.name(), "state": "start", "total": total }));
        self.emit(true);
    }

    pub fn tick(&self, amount: u64) {
        self.done.fetch_add(amount, Ordering::Relaxed);
        let now = self.started.elapsed().as_millis() as u64;
        let last = self.last_emit_ms.load(Ordering::Relaxed);
        if now.saturating_sub(last) < EMIT_EVERY_MS {
            return;
        }
        if self.last_emit_ms.compare_exchange(last, now, Ordering::SeqCst, Ordering::Relaxed).is_ok() {
            self.emit(false);
        }
    }

    fn emit(&self, force: bool) {
        let current = match self.current.try_lock() {
            Ok(current) => current,
            Err(_) if !force => return,
            Err(_) => self.current.lock().unwrap(),
        };
        let Some(phase) = current.phase else { return };
        let done = self.done.load(Ordering::Relaxed).min(current.total.max(1));
        let fraction = if current.total == 0 { 0.0 } else { done as f64 / current.total as f64 };
        let percent = self.overall_percent(phase, fraction);
        let rounded = (percent * 10.0).round() as u64;
        if !force && rounded == self.last_percent.load(Ordering::Relaxed) {
            return;
        }
        self.last_percent.store(rounded, Ordering::Relaxed);
        emit_line(&serde_json::json!({
                    "type": "progress",
                    "phase": phase.name(),
                    "done": done,
                    "total": current.total,
                    "percent": rounded as f64 / 10.0,
        }));
    }

    pub fn finish(&self, phase: Phase) {
        let elapsed = self.current.lock().unwrap().phase_started.elapsed().as_millis() as u64;
        let total = self.current.lock().unwrap().total;
        self.done.store(total, Ordering::SeqCst);
        self.emit(true);
        emit_line(&serde_json::json!({ "type": "phase", "phase": phase.name(), "state": "done", "ms": elapsed }));
    }

    pub fn done(&self) {
        emit_line(&serde_json::json!({
                    "type": "done",
                    "percent": 100,
                    "ms": self.started.elapsed().as_millis() as u64,
        }));
    }
}
