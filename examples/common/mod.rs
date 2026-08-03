use std::time::Duration;

pub mod controller;
pub mod logging;

/// This is a runtime agnostic sleep function, it'll use tokio if inside a tokio runtime, otherwise
/// it'll fall back to asyncio
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


    (flavor = "current_thread", $body:block) => {
        fn main() {
            let mut runtime = tokio::runtime::Builder::new_current_thread();

            #[cfg(not(target_arch = "wasm32"))]
            let runtime = runtime.enable_all();
            let runtime = runtime.build().unwrap();
            runtime.block_on(async $body);
        }
    };

    (flavor = "local", $body:block) => {
        fn main() {
            let mut runtime = tokio::runtime::Builder::new_current_thread();

            #[cfg(not(target_arch = "wasm32"))]
            let runtime = runtime.enable_all();
            let runtime = runtime.build().unwrap();
            let local = tokio::task::LocalSet::new();
            local.block_on(&runtime, async $body);
        }
    };

    ($body:block) => {
        $crate::beacn_main!(flavor = "current_thread", $body);
    };
}