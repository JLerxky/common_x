use std::{sync::OnceLock, time::Duration};

use flume::{Receiver, Sender};
use parking_lot::Mutex;

pub fn close_chain() -> &'static Mutex<CloseChain> {
    static CLOSE_CHAIN: OnceLock<Mutex<CloseChain>> = OnceLock::new();
    CLOSE_CHAIN.get_or_init(|| Mutex::new(CloseChain::init()))
}

pub struct CloseHandler {
    close_rv: Receiver<()>,
    _prev_tx: Option<Sender<()>>,
}

impl CloseHandler {
    pub fn handle(&self) {
        self.close_rv.recv().ok();
    }

    pub fn handle_timeout(&self, timeout: u64) {
        self.close_rv
            .recv_timeout(Duration::from_secs(timeout))
            .ok();
    }

    pub async fn handle_async(&self) {
        self.close_rv.recv_async().await.ok();
    }
}

#[derive(Default)]
pub struct CloseChain(Vec<(Sender<()>, Receiver<()>)>);

impl CloseChain {
    pub fn init() -> Self {
        Self(Vec::new())
    }

    pub fn close(&mut self) {
        self.0.clear();
    }

    pub fn handler(&mut self, deep: usize) -> CloseHandler {
        let len = self.0.len();

        if deep >= len {
            for _ in len..=deep {
                self.0.push(flume::bounded(0));
            }
        }

        if deep == 0 {
            CloseHandler {
                close_rv: self.0[deep].1.clone(),
                _prev_tx: None,
            }
        } else {
            CloseHandler {
                close_rv: self.0[deep].1.clone(),
                _prev_tx: Some(self.0[deep - 1].0.clone()),
            }
        }
    }
}
