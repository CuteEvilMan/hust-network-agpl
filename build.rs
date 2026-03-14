use std::env;
use std::fs;
use std::path::PathBuf;

const FALLBACK_CONSTANTS: &str = r#"
pub const EMBEDDED_CONSTANTS_AVAILABLE: bool = false;
pub const DEFAULT_TEST_URL: &str = "http://connectivitycheck.platform.hicloud.com/generate_204";
pub const DEFAULT_PROBE_TIMEOUT_SECS: u64 = 10;
pub const DEFAULT_LOGIN_TIMEOUT_SECS: u64 = 10;
pub const DEFAULT_RETRY_DELAY_SECS: u64 = 1;
pub const DEFAULT_OK_SLEEP_SECS: u64 = 15;
pub const DEFAULT_EXPECT_204_RESPONSE: bool = true;
pub const USE_EMBEDDED_CONFIG: bool = false;
pub const EMBEDDED_USERNAME: &str = "";
pub const EMBEDDED_PASSWORD: &str = "";
"#;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("missing manifest dir"));
    let constants_path = manifest_dir.join("src").join("constants.rs");
    let out_path =
        PathBuf::from(env::var("OUT_DIR").expect("missing OUT_DIR")).join("constants_generated.rs");

    println!("cargo:rerun-if-changed={}", constants_path.display());

    let generated = if constants_path.exists() {
        let mut content =
            fs::read_to_string(&constants_path).expect("failed to read src/constants.rs");

        let mut header = String::from("pub const EMBEDDED_CONSTANTS_AVAILABLE: bool = true;\n");

        if !content.contains("DEFAULT_EXPECT_204_RESPONSE") {
            header.push_str("pub const DEFAULT_EXPECT_204_RESPONSE: bool = true;\n");
        }

        header.push('\n');
        header.push_str(&content);
        content = header;
        content
    } else {
        println!(
            "cargo:warning=src/constants.rs not found, falling back to runtime config file mode"
        );
        FALLBACK_CONSTANTS.to_owned()
    };

    fs::write(out_path, generated).expect("failed to write generated constants module");
}
