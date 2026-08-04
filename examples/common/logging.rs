pub(crate) fn configure_logging() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use env_logger::{Builder, Env};
        Builder::from_env(Env::default().default_filter_or("info")).init();
    }

    #[cfg(target_arch = "wasm32")]
    wasm_console_log::init_with_level(log::Level::Info).unwrap();

    std::panic::set_hook(Box::new(|info| {
        // Extract just the raw inner payload string if available (e.g., "TEST")
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            *s
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.as_str()
        } else {
            "Box<dyn Any>"
        };

        // Format a single unified error entry matching your [ERROR] sidebar UI style
        if let Some(l) = info.location() {
            log::error!("Panic in '{}' at line {}: {}", l.file(), l.line(), payload);
        } else {
            log::error!("Panic occurred: {}", payload);
        }

        #[cfg(target_arch = "wasm32")]
        {
            if let Some(window) = web_sys::window() {
                if let Ok(event) = web_sys::Event::new("wasm-finished") {
                    let _ = window.dispatch_event(&event);
                }
            }
        }
    }));
}

// Portions of this code are derived from console_log and are covered by its
// existing copyright and license, presented below.
//
// --- console_log MIT License
// Copyright (c) 2018 Matthew Nicholson
//
// Permission is hereby granted, free of charge, to any person obtaining a copy of
// this software and associated documentation files (the "Software"), to deal in
// the Software without restriction, including without limitation the rights to
// use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies
// of the Software, and to permit persons to whom the Software is furnished to do
// so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
// --- End console_log MIT License
//
// Source: https://github.com/iamcodemaker/console_log

#[cfg(target_arch = "wasm32")]
mod wasm_console_log {
    extern crate alloc;

    use alloc::format;
    use log::{Level, Log, Metadata, Record, SetLoggerError};
    use web_sys::{CustomEvent, console};

    use alloc::string::ToString;
    use serde_json::json;
    use wasm_bindgen::JsValue;

    const STYLE: Style<'static> = Style::default();
    static LOGGER: WebConsoleLogger = WebConsoleLogger {};
    struct WebConsoleLogger {}

    impl Log for WebConsoleLogger {
        #[inline]
        fn enabled(&self, metadata: &Metadata) -> bool {
            metadata.level() <= log::max_level()
        }

        fn log(&self, record: &Record) {
            if !self.enabled(record.metadata()) {
                return;
            }

            log(record);
        }

        fn flush(&self) {}
    }

    pub fn log(record: &Record) {
        // Before we do anything, send this across as a window event..
        if let Some(window) = web_sys::window() {
            let detail = json!({
                "level": record.level().to_string(),
                "message": record.args().to_string(),
            });

            // Convert the JSON to a plain string value
            let js_payload = JsValue::from_str(&detail.to_string());

            let event_init = web_sys::CustomEventInit::new();
            event_init.set_detail(&js_payload);

            // Create the event explicitly as "inner-log"
            if let Ok(event) = CustomEvent::new_with_event_init_dict("inner-log", &event_init) {
                let _ = window.dispatch_event(&event);
            }
        }

        // pick the console.log() variant for the appropriate logging level
        let console_log = match record.level() {
            Level::Error => console::error_4,
            Level::Warn => console::warn_4,
            Level::Info => console::info_4,
            Level::Debug => console::log_4,
            Level::Trace => console::debug_4,
        };

        let message = {
            format!(
                "%c{level}%c {file}:{line} %c\n{text}",
                level = record.level(),
                file = record.file().unwrap_or_else(|| record.target()),
                line = record
                    .line()
                    .map_or_else(|| "[Unknown]".to_string(), |line| line.to_string()),
                text = record.args(),
            )
        };

        let level_style = {
            match record.level() {
                Level::Trace => STYLE.trace,
                Level::Debug => STYLE.debug,
                Level::Info => STYLE.info,
                Level::Warn => STYLE.warn,
                Level::Error => STYLE.error,
            }
        };

        console_log(
            &message.into(),
            &level_style.into(),
            &STYLE.file_line.into(),
            &STYLE.text.into(),
        );
    }

    #[inline]
    pub fn init_with_level(level: Level) -> Result<(), SetLoggerError> {
        log::set_logger(&LOGGER)?;
        log::set_max_level(level.to_level_filter());
        Ok(())
    }

    /// Log message styling.
    ///
    /// Adapted from <https://gitlab.com/limira-rs/wasm-logger/-/blob/0c16227/src/lib.rs#L72-85>
    pub(crate) struct Style<'s> {
        pub trace: &'s str,
        pub debug: &'s str,
        pub info: &'s str,
        pub warn: &'s str,
        pub error: &'s str,
        pub file_line: &'s str,
        pub text: &'s str,
    }

    impl Style<'static> {
        /// Returns default style values.
        pub const fn default() -> Self {
            macro_rules! bg_color {
                ($color:expr) => {
                    concat!("color: white; padding: 0 3px; background: ", $color, ";")
                };
            }

            Style {
                trace: bg_color!("gray"),
                debug: bg_color!("blue"),
                info: bg_color!("green"),
                warn: bg_color!("orange"),
                error: bg_color!("darkred"),
                file_line: "font-weight: bold; color: inherit",
                text: "background: inherit; color: inherit",
            }
        }
    }
}
