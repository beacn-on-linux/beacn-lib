use crate::common::logging::configure_logging;
use flume::Sender;
use log::{error, info, warn};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{BufRead, BufReader, Read};
use std::process::Stdio;
use std::str::FromStr;
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};
use tiny_http::{Header, Request, Response, Server, StatusCode};
use wasm_bindgen_cli_support::Bindgen;

const INDEX_HTML: &[u8] = include_bytes!("wasm_helpers/index.html");
const IFRAME_HTML: &[u8] = include_bytes!("wasm_helpers/iframe.html");

type HttpResponse = Response<Box<dyn std::io::Read + Send>>;

#[path = "common/mod.rs"]
mod common;

fn main() {
    configure_logging();

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manager = ExampleManager::new(root.clone());

    let server = WebServer::new(manager, root);
    server.run();
}

struct ExampleManager {
    root: PathBuf,
    build_cache: Mutex<HashMap<String, String>>,
}

impl ExampleManager {
    fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            build_cache: Mutex::new(HashMap::new()),
        }
    }
}

struct WebServer {
    manager: Arc<ExampleManager>,
    root: PathBuf,
}

impl WebServer {
    fn new(manager: ExampleManager, root: impl Into<PathBuf>) -> Self {
        Self {
            manager: Arc::new(manager),
            root: root.into(),
        }
    }

    fn run(&self) {
        let server = Server::http("127.0.0.1:8000").expect("failed starting server");

        println!("Serving:");
        println!("  http://127.0.0.1:8000");

        for request in server.incoming_requests() {
            let (request, response) = self.route(request);

            if let Some(request) = request {
                request.respond(response).unwrap();
            }
        }
    }

    fn route(&self, request: Request) -> (Option<Request>, HttpResponse) {
        let url = request.url().to_string();
        let url = url.split('?').next().unwrap_or(&url);

        let parts: Vec<_> = url.trim_matches('/').split('/').collect();

        info!("Parts: {:?}", parts.as_slice());

        match parts.as_slice() {
            ["examples"] => {
                let examples = self.manager.examples();

                let content = serde_json::to_string(&examples).unwrap_or_else(|e| {
                    error!("Failed to serialize examples: {:?}", e);
                    "{}".to_string()
                });

                let content_type = "Content-Type: application/json";
                let content_type = Header::from_str(content_type).unwrap();
                (
                    Some(request),
                    Response::from_string(content)
                        .with_header(content_type)
                        .boxed(),
                )
            }

            ["load", name] => {
                self.stream_build(request, name);
                (None, Response::empty(StatusCode(200)).boxed())
            }

            ["generated", name, file] => {
                let filename = self.root.join("generated").join(name).join(file);
                info!("Loading file: {}", filename.display());

                (Some(request), self.serve_file(filename))
            }

            ["runner.html"] => (Some(request), self.serve_iframe()),

            _ => {
                if url == "/" || url == "/index.html" {
                    return (Some(request), self.serve_index());
                }

                let filename = url.trim_start_matches('/');
                (Some(request), self.serve_file(filename))
            }
        }
    }

    fn serve_file(&self, filename: impl Into<PathBuf>) -> HttpResponse {
        let path = filename.into();

        if !path.exists() {
            return Response::empty(StatusCode(404)).boxed();
        }

        let data = fs::read(&path).expect("failed reading file");

        let content_type = match path.extension().and_then(|e| e.to_str()) {
            Some("html") => "text/html; charset=utf-8",
            Some("js") => "application/javascript",
            Some("wasm") => "application/wasm",
            _ => "application/octet-stream",
        };

        let header = format!("Content-Type: {content_type}");
        let header = Header::from_str(&header).unwrap();
        Response::from_data(data).with_header(header).boxed()
    }

    fn serve_index(&self) -> HttpResponse {
        let header = "Content-Type: text/html; charset=utf-8";
        let header = Header::from_str(header).unwrap();

        let content = INDEX_HTML.to_vec();
        Response::from_data(content).with_header(header).boxed()
    }

    fn serve_iframe(&self) -> HttpResponse {
        let header = "Content-Type: text/html; charset=utf-8";
        let header = Header::from_str(header).unwrap();

        let content = IFRAME_HTML.to_vec();
        Response::from_data(content).with_header(header).boxed()
    }

    fn stream_build(&self, request: Request, name: &str) {
        info!("Stream Build Started..");

        let (sender, receiver) = flume::unbounded();

        let manager = self.manager.clone();
        let name = name.to_string();

        std::thread::spawn(move || {
            manager.build(&name, sender.clone());
        });

        let mut output_stream = OutputStream {
            receiver,
            buffer: Vec::new(),
        };

        let mut writer = request.into_writer();
        write!(
            writer,
            "HTTP/1.1 200 OK\r\n\
             Content-Type: text/plain\r\n\
             Cache-Control: no-cache\r\n\
             Transfer-Encoding: chunked\r\n\
             Connection: close\r\n\
             \r\n"
        )
        .unwrap();
        writer.flush().unwrap();

        let mut buf = [0u8; 8192];

        loop {
            let len = output_stream.read(&mut buf).unwrap();
            if len == 0 {
                break;
            }

            // 2. Format data into an HTTP chunk: [Hex Length]\r\n[Data]\r\n
            if write!(writer, "{:X}\r\n", len).is_err() {
                break;
            }
            if writer.write_all(&buf[..len]).is_err() {
                break;
            }
            if write!(writer, "\r\n").is_err() {
                break;
            }

            // 3. This immediately pushes the chunk to the browser
            if let Err(e) = writer.flush() {
                warn!("Client Disconnected: {}", e);
                break;
            }
        }
        let _ = write!(writer, "0\r\n\r\n");
        let _ = writer.flush();

        drop(writer);

        info!("Stream Build Finished..");
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Builder and Cache

impl ExampleManager {
    fn build(&self, name: &str, log: Sender<Output>) -> PathBuf {
        let current_hash = match self.calculate_file_hash(name) {
            Some(hash) => hash,
            None => {
                let _ = log.send(Output::ExitStatus {
                    success: false,
                    reason: Some(format!(
                        "Could not locate source file for example '{}'",
                        name
                    )),
                });
                return self.root.join("generated").join(name);
            }
        };

        {
            let cache = self.build_cache.lock().unwrap();
            if let Some(cached_hash) = cache.get(name)
                && cached_hash == &current_hash
            {
                let output = self.root.join("generated").join(name);
                let _ = log.send(Output::ExitStatus {
                    success: true,
                    reason: Some(format!("Using cached compilation binary for {}", name)),
                });
                return output;
            }
        }

        let _ = log.send(format!("Building WASM example: {name}").into());

        let mut command = Command::new("cargo");
        command.args([
            "--color",
            "always",
            "build",
            "--example",
            name,
            "--features",
            "async",
            "--target",
            "wasm32-unknown-unknown",
        ]);

        if !cfg!(debug_assertions) {
            command.arg("--release");
        }
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let mut child = command.spawn().expect("failed to execute cargo");

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let stdout_task = forward_output(stdout, log.clone(), OutputKind::Stdout);
        let stderr_task = forward_output(stderr, log.clone(), OutputKind::Stderr);

        let status = child.wait().expect("failed waiting for cargo");

        let _ = stdout_task.join();
        let _ = stderr_task.join();

        if !status.success() {
            let _ = log.send(Output::ExitStatus {
                success: false,
                reason: Some(format!("Cargo build failed for {} ({})", name, status)),
            });
            return self.root.join("generated").join(name);
        }

        let wasm = wasm_generated_dir(&self.root).join(format!("{name}.wasm"));
        if !wasm.exists() {
            let _ = log.send(Output::ExitStatus {
                success: false,
                reason: Some(format!("Target binary missing: {}.wasm", name)),
            });
            return self.root.join("generated").join(name);
        }

        let _ = log.send("Running wasm-bindgen".into());
        let output = self.root.join("generated").join(name);

        // Check the result of the binding process explicitly
        if let Err(e) = self.run_bindgen(&wasm, &output) {
            let _ = log.send(Output::ExitStatus {
                success: false,
                reason: Some(e),
            });
            return self.root.join("generated").join(name);
        }

        {
            let mut cache = self.build_cache.lock().unwrap();
            cache.insert(name.to_string(), current_hash);
        }

        let _ = log.send(Output::ExitStatus {
            success: true,
            reason: Some(format!("Build Complete, preparing to run {}", name)),
        });

        output
    }

    fn run_bindgen(&self, wasm: &Path, output: &Path) -> Result<(), String> {
        info!("Running wasm-bindgen");

        if output.exists()
            && let Err(e) = fs::remove_dir_all(output)
        {
            return Err(format!("Failed to clean stale bindgen directory: {e}"));
        }

        if let Err(e) = fs::create_dir_all(output) {
            return Err(format!("Failed to create output directory: {e}"));
        }

        let mut bindgen = Bindgen::new();
        bindgen.input_path(wasm);

        if let Err(e) = bindgen.web(true) {
            return Err(format!("Failed setting wasm-bindgen web mode: {e:?}"));
        }

        // Catch the generation result instead of using .expect()
        if let Err(e) = bindgen.generate(output) {
            return Err(format!("wasm-bindgen failed to generate bindings: {e:?}"));
        }

        Ok(())
    }

    // A lightweight helper to get a deterministic hash of the example file's source code
    fn calculate_file_hash(&self, name: &str) -> Option<String> {
        // Examples sit inside the standard workspace layout path: examples/name.rs
        let source_path = self.root.join("examples").join(format!("{}.rs", name));

        if let Ok(bytes) = fs::read(source_path) {
            let mut hasher = DefaultHasher::new();
            bytes.hash(&mut hasher);
            Some(format!("{:x}", hasher.finish()))
        } else {
            None
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Example Finder and Handler

#[derive(Debug, Serialize)]
struct Example {
    name: String,
}

impl ExampleManager {
    fn examples(&self) -> Vec<Example> {
        let output = Command::new("cargo")
            .args(["metadata", "--format-version", "1", "--no-deps"])
            .output()
            .expect("failed to run cargo metadata");

        if !output.status.success() {
            error!("Failed to get Cargo Metadata: {}", output.status);
            return vec![];
        }

        let metadata = match serde_json::from_slice::<Value>(&output.stdout) {
            Ok(metadata) => metadata,
            Err(e) => {
                error!("Failed to parse Cargo Metadata: {:?}", e);
                return vec![];
            }
        };

        let mut examples = Vec::new();
        let Some(packages) = metadata["packages"].as_array() else {
            error!("Cargo metadata did not contain packages");
            return examples;
        };

        for package in packages {
            let Some(targets) = package["targets"].as_array() else {
                continue;
            };

            for target in targets {
                let Some(kind) = target["kind"].as_array() else {
                    continue;
                };

                if !kind.contains(&serde_json::json!("example")) {
                    continue;
                }

                let requires_async = target["required-features"]
                    .as_array()
                    .map(|features| {
                        features
                            .iter()
                            .any(|feature| feature.as_str() == Some("async"))
                    })
                    .unwrap_or(false);

                if !requires_async {
                    continue;
                }

                if let Some(name) = target["name"].as_str() {
                    examples.push(Example {
                        name: name.to_string(),
                    });
                }
            }
        }

        examples
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Stream and Message Handler

struct OutputStream {
    receiver: flume::Receiver<Output>,
    buffer: Vec<u8>,
}

impl Read for OutputStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // 1. Only block and wait on the channel if our buffer is completely dry
        if self.buffer.is_empty() {
            match self.receiver.recv() {
                Ok(output) => {
                    if let Ok(json) = serde_json::to_vec(&output) {
                        self.buffer = json;
                        self.buffer.push(0); // Null terminator
                    }
                }
                // Channel closed and empty -> Signal standard EOF
                Err(_) => {
                    return Ok(0);
                }
            }
        }

        // 2. Safely copy what is available right now
        let len = self.buffer.len().min(buf.len());
        buf[..len].copy_from_slice(&self.buffer[..len]);

        // 3. Remove what we copied from our internal tracking vec
        self.buffer.drain(..len);

        // 4. Return immediately so stream_build can flush these bytes down the socket
        Ok(len)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Message Logging and UI Forwarding

#[derive(Debug, Clone, Copy, serde::Serialize)]
enum OutputKind {
    Stdout,
    Stderr,
}

#[derive(serde::Serialize)]
#[serde(tag = "type")]
enum Output {
    Line {
        kind: OutputKind,
        text: String,
    },
    ExitStatus {
        success: bool,
        reason: Option<String>,
    },
}

impl From<&str> for Output {
    fn from(text: &str) -> Self {
        Output::Line {
            kind: OutputKind::Stdout,
            text: text.to_string(),
        }
    }
}

impl From<String> for Output {
    fn from(text: String) -> Self {
        Output::Line {
            kind: OutputKind::Stdout,
            text,
        }
    }
}

fn forward_output<R>(reader: R, output: Sender<Output>, kind: OutputKind) -> JoinHandle<()>
where
    R: Read + Send + 'static,
{
    std::thread::spawn(move || {
        let reader = BufReader::new(reader);
        for line in BufRead::lines(reader) {
            let Ok(line) = line else {
                break;
            };

            match kind {
                OutputKind::Stdout => info!("{line}"),
                OutputKind::Stderr => warn!("{line}"),
            }

            let _ = output.send(Output::Line { kind, text: line });
        }
    })
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Utility

fn wasm_generated_dir(root: &Path) -> PathBuf {
    if cfg!(debug_assertions) {
        root.join("target/wasm32-unknown-unknown/debug/examples")
    } else {
        root.join("target/wasm32-unknown-unknown/release/examples")
    }
}
