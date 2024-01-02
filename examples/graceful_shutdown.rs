use color_eyre::Result;
use common_x::{
    graceful_shutdown::{CloseChain, CloseHandler},
    signal::shutdown_signal,
};
use tracing::info;

struct A {
    _data: u8,
    close_hander: CloseHandler,
}

impl Drop for A {
    fn drop(&mut self) {
        self.close_hander.handle();
        info!("A drop");
    }
}

struct B {
    _data: u8,
    close_hander: CloseHandler,
}

struct C {
    _data: u8,
    close_hander: CloseHandler,
}

#[tokio::main]
async fn main() -> Result<()> {
    common_x::log::init_log_filter("info");
    let mut close_chain = CloseChain::init();

    let _a = A {
        _data: 1,
        close_hander: close_chain.handler(0),
    };
    let mut c = C {
        _data: 3,
        close_hander: close_chain.handler(2),
    };
    let mut b = B {
        _data: 2,
        close_hander: close_chain.handler(1),
    };
    let mut b1 = B {
        _data: 2,
        close_hander: close_chain.handler(1),
    };
    tokio::spawn(async move {
        b.close_hander.handle();
        info!("B drop");
    });
    tokio::spawn(async move {
        b1.close_hander.handle();
        info!("B1 drop");
    });
    tokio::spawn(async move {
        c.close_hander.handle();
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        info!("C drop");
    });
    shutdown_signal().await;
    drop(close_chain);
    Ok(())
}
