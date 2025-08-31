use perf_event::events::Hardware;
use perf_event::{Builder, Counter};

pub struct PerfCounter {
    counter: Option<Counter>,
}

impl PerfCounter {
    pub fn new() -> Self {
        let counter = Builder::new()
            .kind(Hardware::CPU_CYCLES)
            .build()
            .map_err(|e| {
                eprintln!(
                    "Warning: Failed to open perf counter ({}), will use time-based measurement",
                    e
                );
                e
            })
            .ok();

        PerfCounter { counter }
    }

    pub fn start(&mut self) {
        if let Some(ref mut counter) = self.counter {
            let _ = counter.reset();
            let _ = counter.enable();
        }
    }

    pub fn read(&mut self) -> u64 {
        if let Some(ref mut counter) = self.counter {
            match counter.read() {
                Ok(val) => val,
                Err(e) => {
                    eprintln!("Failed to read counter: {}", e);
                    0
                }
            }
        } else {
            0
        }
    }

    pub fn stop(&mut self) {
        if let Some(ref mut counter) = self.counter {
            let _ = counter.disable();
        }
    }

    pub fn is_valid(&self) -> bool {
        self.counter.is_some()
    }
}
