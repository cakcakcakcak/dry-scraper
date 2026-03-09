use std::sync::Arc;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

pub trait ProgressReporter: Send {
    fn inc(&self, n: u64);
    fn set_len(&self, len: u64);
    fn set_message(&self, msg: &str);
    fn finish(&self);
}

#[derive(Clone)]
pub enum ProgressReporterMode {
    Noop,
    Indicatif(Arc<MultiProgress>, ProgressStyle),
    // Channel variant added in Phase 2
}

impl ProgressReporterMode {
    pub fn create_reporter(
        &self,
        total: Option<u64>,
        msg: &str,
    ) -> Box<dyn ProgressReporter + Send> {
        match self {
            Self::Noop => Box::new(NoopReporter),
            Self::Indicatif(mp, style) => {
                let pb = mp.add(ProgressBar::new(total.unwrap_or(0)));
                pb.set_style(style.clone());
                pb.set_message(msg.to_string());
                Box::new(IndicatifReporter { pb })
            }
        }
    }
}

pub struct NoopReporter;
impl ProgressReporter for NoopReporter {
    fn inc(&self, _n: u64) {}
    fn set_len(&self, _len: u64) {}
    fn set_message(&self, _msg: &str) {}
    fn finish(&self) {}
}

pub struct IndicatifReporter {
    pb: ProgressBar,
}
impl ProgressReporter for IndicatifReporter {
    fn inc(&self, n: u64) {
        self.pb.inc(n)
    }
    fn set_len(&self, len: u64) {
        self.pb.set_length(len)
    }
    fn set_message(&self, msg: &str) {
        self.pb.set_message(msg.to_string())
    }
    fn finish(&self) {
        self.pb.finish()
    }
}
