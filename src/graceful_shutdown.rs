use std::{sync::Mutex, time::Duration};

use flume::{Receiver, Sender};

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

pub struct CloseChain(Mutex<Vec<(Sender<()>, Receiver<()>)>>);

impl Default for CloseChain {
    fn default() -> Self {
        Self(Mutex::new(vec![flume::bounded(0)]))
    }
}

impl CloseChain {
    pub fn close(&self) {
        self.0.lock().unwrap().clear();
    }

    pub fn token(&self) -> CloseToken {
        let chain = self.0.lock().unwrap();
        let deep = chain.len() - 1;
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

    pub fn child_token(&self) -> CloseToken {
        let mut chain = self.0.lock().unwrap();
        let deep = chain.len();

        chain.push(flume::bounded(0));

        CloseToken {
            close_rv: chain[deep].1.clone(),
            _prev_tx: Some(chain[deep - 1].0.clone()),
        }
    }
}
