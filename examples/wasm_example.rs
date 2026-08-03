use std::{env, fs, path::PathBuf, process::Command};

use tiny_http::{Header, Response, Server, StatusCode};
use wasm_bindgen_cli_support::Bindgen;

fn run_wasm_bindgen(wasm_path: &PathBuf, pkg_dir: &std::path::Path, name: &str) {
    println!("Running wasm-bindgen");

    fs::create_dir_all(pkg_dir).expect("failed to create pkg directory");

    let mut bindgen = Bindgen::new();

    bindgen.input_path(wasm_path);
    bindgen.web(true).expect("failed to set web flag");

    bindgen
        .generate(pkg_dir)
        .expect("failed to generate wasm-bindgen output");
}

fn main() {
    let example = env::args()
        .nth(1)
        .expect("usage: cargo run --example wasm-example -- <example-name>");

    // #[cfg(not(feature = "tokio"))]
    // compile_error!("The wasm-example example requires the tokio feature to be enabled");

    println!("Building WASM example: {example}");

    let status = Command::new("cargo")
        .args([
            "build",
            "--example",
            &example,
            "--features",
            "smol",
            "--target",
            "wasm32-unknown-unknown",
        ])
        .status()
        .expect("failed to run cargo");

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    let wasm_path = PathBuf::from(format!(
        "target/wasm32-unknown-unknown/debug/examples/{example}.wasm"
    ));

    if !wasm_path.exists() {
        panic!("WASM file not found: {}", wasm_path.display());
    }

    let temp = tempfile::tempdir().unwrap();

    let pkg_dir = temp.path().join("pkg");

    run_wasm_bindgen(&wasm_path, &pkg_dir, &example);

    let index = format!(
        r#"
<!doctype html>
<html>
<head>
<meta charset="utf-8">
<title>Rust WASM Example</title>
</head>
<body>
<pre id="output"></pre>

<script type="module">
import init from "./pkg/{example}.js";

const output = document.getElementById("output");

try {{
    await init("./pkg/{example}_bg.wasm");

    output.textContent = "WASM loaded.";
}} catch (err) {{
    console.error(err);
    output.textContent =
        "WASM failed to load:\n\n" +
        (err.stack || err);
}}
</script>

</body>
</html>
"#
    );

    fs::write(temp.path().join("index.html"), index).unwrap();

    env::set_current_dir(temp.path()).unwrap();

    let server = Server::http("127.0.0.1:8000").expect("failed to start server");

    println!("Serving:");
    println!("  http://127.0.0.1:8000");

    //webbrowser::open("http://127.0.0.1:8000").ok();

    for request in server.incoming_requests() {
        let path = request.url().trim_start_matches('/');

        let filename = if path.is_empty() { "index.html" } else { path };

        let path = PathBuf::from(filename);
        if !path.exists() {
            let response = Response::empty(StatusCode(404));
            request.respond(response).unwrap();
            continue;
        }

        let data = fs::read(filename).expect("failed to read file");

        let content_type = match filename {
            "index.html" => "text/html; charset=utf-8",
            x if x.ends_with(".js") => "application/javascript",
            x if x.ends_with(".wasm") => "application/wasm",
            _ => "application/octet-stream",
        };

        let response = Response::from_data(data).with_header(
            Header::from_bytes(&b"Content-Type"[..], content_type.as_bytes()).unwrap(),
        );

        request.respond(response).unwrap();
    }
}
