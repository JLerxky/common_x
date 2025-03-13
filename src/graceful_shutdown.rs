use std::{
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};

use flume::{Receiver, Sender};

pub fn close_chain() -> &'static Arc<CloseChain> {
    static CLOSE_CHAIN: OnceLock<Arc<CloseChain>> = OnceLock::new();
    CLOSE_CHAIN.get_or_init(|| Arc::new(CloseChain::default()))
}

pub struct CloseToken {
    close_rv: Receiver<()>,
    _prev_tx: Option<Sender<()>>,
}

impl CloseToken {
    pub fn closed(&self) {
        self.close_rv.recv().ok();
    }

    pub fn closed_with_timeout(&self, timeout: u64) {
        self.close_rv
            .recv_timeout(Duration::from_secs(timeout))
            .ok();
    }

    pub async fn closed_async(&self) {
        self.close_rv.recv_async().await.ok();
    }
}

#[derive(Default)]
pub struct CloseChain(Mutex<Vec<(Sender<()>, Receiver<()>)>>);

impl CloseChain {
    pub fn close(&self) {
        self.0.lock().unwrap().clear();
    }

    pub fn token(&self, deep: usize) -> CloseToken {
        let mut chain = self.0.lock().unwrap();
        let len = chain.len();

        if deep >= len {
            for _ in len..=deep {
                chain.push(flume::bounded(0));
            }
        }

        if deep == 0 {
            CloseToken {
                close_rv: chain[deep].1.clone(),
                _prev_tx: None,
            }
        } else {
            CloseToken {
                close_rv: chain[deep].1.clone(),
                _prev_tx: Some(chain[deep - 1].0.clone()),
            }
        }
    }
}
