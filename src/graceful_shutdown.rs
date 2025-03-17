use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use flume::{Receiver, Sender};
use tracing::debug;

type CloseChain = Arc<Mutex<Vec<(Sender<()>, Receiver<()>)>>>;

#[derive(Clone)]
pub struct CloseToken {
    deep: usize,
    close_chain: CloseChain,
    close_rv: Receiver<()>,
    _prev_tx: Option<Sender<()>>,
}

impl Default for CloseToken {
    fn default() -> Self {
        let token = flume::bounded(0);
        let close_chain = Arc::new(Mutex::new(vec![token.clone()]));
        Self {
            deep: 0,
            close_rv: token.1,
            _prev_tx: None,
            close_chain,
        }
    }
}

impl CloseToken {
    pub fn close(&self) {
        self.close_chain.lock().unwrap().clear();
        self.closed();
    }

    pub fn close_with_timeout(&self, timeout: u64) {
        self.close_chain.lock().unwrap().clear();
        self.closed_with_timeout(timeout);
    }

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

    pub fn child_token(&self) -> CloseToken {
        let mut chain = self.close_chain.lock().unwrap();
        let deep = self.deep + 1;
        if deep >= chain.len() {
            chain.push(flume::bounded(0));
        }
        debug!(
            "child_token[{}] created by close_token[{}] ",
            deep, self.deep
        );
        Self {
            deep,
            close_rv: chain[deep].1.clone(),
            _prev_tx: Some(chain[deep - 1].0.clone()),
            close_chain: self.close_chain.clone(),
        }
    }
}

impl Drop for CloseToken {
    fn drop(&mut self) {
        self.closed();
        debug!("close_token[{}] dropped", self.deep);
    }
}

#[test]
fn test() {
    let token = CloseToken::default();
    let token1 = token.clone();
    let child1 = token.child_token();
    let child2 = child1.child_token();
    let child3 = token.child_token();
    let child4 = child1.clone();
    let child5 = child4.child_token();
    println!("token deep: {}", token.deep);
    println!("child1 deep: {}", child1.deep);
    println!("child2 deep: {}", child2.deep);
    println!("child3 deep: {}", child3.deep);
    println!("child4 deep: {}", child4.deep);
    println!("child5 deep: {}", child5.deep);
    println!("len: {}", token.close_chain.lock().unwrap().len());

    token.close_chain.lock().unwrap().clear();
    println!(
        "child4.close_rv.sender_count: {}",
        child4.close_rv.sender_count()
    );
    drop(child5);
    println!(
        "child4.close_rv.sender_count: {}",
        child4.close_rv.sender_count()
    );
    drop(child2);
    println!(
        "child4.close_rv.sender_count: {}",
        child4.close_rv.sender_count()
    );
    child4.closed();
    child3.closed();
    child1.closed();
    drop(child4);
    drop(child3);
    drop(child1);
    token1.closed();

    println!("exit len: {}", token.close_chain.lock().unwrap().len());
}
