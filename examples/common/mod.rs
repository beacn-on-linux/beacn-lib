use std::time::Duration;

pub mod controller;
pub mod logging;

// Clippy doesn't understand that there's more than one example, and flags these unused :D

/// This is a runtime agnostic sleep function, it'll use tokio if inside a tokio runtime, otherwise
/// it'll fall back to asyncio
#[allow(unused)]
pub(crate) async fn sleep(duration: Duration) {
    #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
    {
        if tokio::runtime::Handle::try_current().is_ok() {
            tokio::time::sleep(duration).await;
            return;
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        async_io::Timer::after(duration).await;
    }

    #[cfg(target_arch = "wasm32")]
    {
        gloo_timers::future::sleep(duration).await;
    }
}

// This is kinda silly, but I don't wanna make a proc macro for this, and it's here to ensure
// that wasm environments are built correctly without time.
#[macro_export]
macro_rules! beacn_main {

    // To address the elephant in the room, we don't support flavor = "multi_thread" as:
    // * If rt-multi-thread is disabled, this can't compile
    // * Tokio will not compile under WASM with rt-multi-thread enabled.
    // * We cannot check the feature set of tokio.
    //
    // Given that all our examples are designed to be single threaded anyway as they're example
    // and don't require a massive threaded runtime, we won't support flavor = "muliti_thread".


    (@wasm $body:block) => {{
        wasm_bindgen_futures::spawn_local(async move {
            $body

            if let Some(window) = web_sys::window() {
                if let Ok(event) = web_sys::Event::new("wasm-finished") {
                    let _ = window.dispatch_event(&event);
                }
            }
        });
    }};

    (flavor = "current_thread", $body:block) => {
        fn main() {
            #[cfg(target_arch = "wasm32")]
            {
                $crate::beacn_main!(@wasm $body);
            }

            #[cfg(not(target_arch = "wasm32"))]
            {
                let mut runtime = tokio::runtime::Builder::new_current_thread();

                let runtime = runtime.enable_all();
                let runtime = runtime.build().unwrap();

                runtime.block_on(async $body);
            }
        }
    };

    (flavor = "local", $body:block) => {
        fn main() {
            #[cfg(target_arch = "wasm32")]
            {
                $crate::beacn_main!(@wasm $body);
            }

            #[cfg(not(target_arch = "wasm32"))]
            {
                let mut runtime = tokio::runtime::Builder::new_current_thread();

                let runtime = runtime.enable_all();
                let runtime = runtime.build().unwrap();

                let local = tokio::task::LocalSet::new();
                local.block_on(&runtime, async $body);
            }
        }
    };

    ($body:block) => {
        $crate::beacn_main!(flavor = "current_thread", $body);
    };
}

#[allow(unused)]
pub fn spawn_local<F>(future: F) -> TaskHandle
where
    F: Future<Output = ()> + 'static,
{
    #[cfg(not(target_arch = "wasm32"))]
    {
        TaskHandle {
            inner: tokio::task::spawn_local(future),
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        let (tx, rx) = flume::unbounded();

        wasm_bindgen_futures::spawn_local(async move {
            future.await;
            let _ = tx.send(());
        });

        TaskHandle { done: rx }
    }
}

#[allow(unused)]
pub struct TaskHandle {
    #[cfg(not(target_arch = "wasm32"))]
    inner: tokio::task::JoinHandle<()>,

    #[cfg(target_arch = "wasm32")]
    done: flume::Receiver<()>,
}

impl TaskHandle {
    #[allow(unused)]
    pub async fn join(self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = self.inner.await;
        }

        #[cfg(target_arch = "wasm32")]
        {
            let _ = self.done.recv_async().await;
        }
    }
}

#[allow(unused)]
pub mod interval {
    use futures_timer::Delay;
    use web_time::Duration;

    pub struct Interval {
        period: Duration,
        delay: Delay,
    }

    impl Interval {
        pub fn new(period: Duration) -> Self {
            Self {
                period,
                delay: Delay::new(period),
            }
        }

        pub async fn tick(&mut self) {
            (&mut self.delay).await;
            self.delay = Delay::new(self.period);
        }
    }
}
