mod encrypt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{io, thread};

mod embedded_constants {
    include!(concat!(env!("OUT_DIR"), "/constants_generated.rs"));
}

const MAIN_DEFAULT_TEST_URL: &str = "http://connectivitycheck.platform.hicloud.com/generate_204";
const MAIN_DEFAULT_PROBE_TIMEOUT_SECS: u64 = 10;
const MAIN_DEFAULT_LOGIN_TIMEOUT_SECS: u64 = 10;
const MAIN_DEFAULT_RETRY_DELAY_SECS: u64 = 1;
const MAIN_DEFAULT_OK_SLEEP_SECS: u64 = 15;
const MAIN_DEFAULT_EXPECT_204_RESPONSE: bool = true;
const MAIN_DEFAULT_CONFIG_PATHS: &[&str] = &["hust-network-login.conf", "my.conf", "config.txt"];

fn extract<'a>(text: &'a str, prefix: &'a str, suffix: &'a str) -> io::Result<&'a str> {
    if let Some(l) = text.find(prefix) {
        let start = l + prefix.len();
        if let Some(r) = text[start..].find(suffix) {
            return Ok(&text[start..start + r]);
        }
    }
    Err(io::ErrorKind::InvalidData.into())
}

/// 校园网自动登录函数
///
/// 通过访问华科官网探测网络状态，如果被重定向到认证页面则自动登录
fn login(config: &Config) -> io::Result<()> {
    let username = &config.username;
    let password = &config.password;

    // 第一步：访问华科官网检测是否需要登录
    // 如果未登录，会被重定向到认证门户页面
    let resp = match minreq::get(&config.test_url)
        .with_timeout(config.probe_timeout_secs)
        .send()
    {
        Ok(resp) => resp,
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("https") {
                println!("https redirect detected; assuming already online");
                return Ok(());
            }

            println!("{} request failed! {}", &config.test_url, e);
            return Err(io::ErrorKind::ConnectionRefused.into());
        }
    };

    if config.expect_204_response && resp.status_code == 204 {
        return Ok(());
    }

    let resp = resp.as_str().map_err(|e| {
        println!("invalid resp format {}", e);
        io::ErrorKind::InvalidData
    })?;

    if config.expect_204_response
        && !resp.contains("/eportal/index.jsp")
        && !resp.contains("<script>top.self.location.href='http://")
    {
        println!(
            "probe url {} did not return 204; got a non-portal response instead",
            &config.test_url
        );
        return Err(io::ErrorKind::InvalidData.into());
    }

    // 检查响应内容，如果不包含认证页面标识，说明已经登录
    if !resp.contains("/eportal/index.jsp")
        && !resp.contains("<script>top.self.location.href='http://")
    {
        return Ok(());
    }

    // 第二步：从重定向页面中提取认证所需信息
    // 提取认证门户的 IP 地址
    let portal_ip = extract(
        resp,
        "<script>top.self.location.href='http://",
        "/eportal/index.jsp",
    )?;
    println!("portal ip: {}", portal_ip);

    // 提取设备的 MAC 地址
    let mac = extract(resp, "mac=", "&t=")?;
    println!("mac: {}", mac);

    // 第三步：加密密码
    // 将密码和 MAC 地址拼接后用 RSA 加密
    let encrypt_pass = encrypt::encrypt_pass(format!("{}>{}", password, mac));

    // 提取查询字符串参数
    let query_string = extract(resp, "/eportal/index.jsp?", "'</script>\r\n")?;
    println!("query_string: {}", query_string);

    // URL 编码查询字符串
    let query_string = urlencoding::encode(query_string);

    // 第四步：构造登录请求体
    let body = format!(
        "userId={}&password={}&service=&queryString={}&passwordEncrypt=true",
        username, encrypt_pass, query_string
    );

    // 构造登录 URL
    let login_url = format!("http://{}/eportal/InterFace.do?method=login", portal_ip);

    // 第五步：发送 POST 登录请求
    let resp = minreq::post(login_url)
        .with_body(body)
        .with_header(
            "Content-Type",
            "application/x-www-form-urlencoded; charset=UTF-8",
        )
        .with_header("Accept", "*/*")
        .with_header("User-Agent", "hust-network-login")
        .with_timeout(config.login_timeout_secs)
        .send()
        .map_err(|e| {
            println!("portal boom! {}", e);
            io::ErrorKind::ConnectionRefused
        })?;

    let resp = resp.as_str().map_err(|e| {
        println!("invalid login resp format {}", e);
        io::ErrorKind::InvalidData
    })?;

    println!("login resp: {}", resp);

    // 第六步：检查登录结果
    if resp.contains("success") {
        Ok(())
    } else {
        Err(io::ErrorKind::PermissionDenied.into())
    }
}

struct Config {
    username: String,
    password: String,
    test_url: String,
    probe_timeout_secs: u64,
    login_timeout_secs: u64,
    retry_delay_secs: u64,
    ok_sleep_secs: u64,
    expect_204_response: bool,
}

impl Config {
    fn with_main_defaults(username: String, password: String) -> Self {
        Self {
            username,
            password,
            test_url: MAIN_DEFAULT_TEST_URL.to_owned(),
            probe_timeout_secs: MAIN_DEFAULT_PROBE_TIMEOUT_SECS,
            login_timeout_secs: MAIN_DEFAULT_LOGIN_TIMEOUT_SECS,
            retry_delay_secs: MAIN_DEFAULT_RETRY_DELAY_SECS,
            ok_sleep_secs: MAIN_DEFAULT_OK_SLEEP_SECS,
            expect_204_response: MAIN_DEFAULT_EXPECT_204_RESPONSE,
        }
    }

    fn validate_and_assemble(
        username: Option<&str>,
        password: Option<&str>,
    ) -> Result<Self, &'static str> {
        match (username, password) {
            (Some(_), None) => Err("NO password"),
            (None, Some(_)) => Err("NO username"),
            (None, None) => Err("NO username and password"),
            (Some(username), Some(password)) => Ok(Self::with_main_defaults(
                username.to_owned(),
                password.to_owned(),
            )),
        }
    }

    fn parse_u64(key: &str, value: &str) -> Option<u64> {
        value
            .parse::<u64>()
            .inspect_err(|err| println!("invalid value for {key}: {err}"))
            .ok()
    }

    fn parse_bool(key: &str, value: &str) -> Option<bool> {
        match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => {
                println!("invalid value for {key}: {value}");
                None
            }
        }
    }

    fn apply_override(&mut self, key: &str, value: &str) {
        match key {
            "hust_url" => self.test_url = value.to_owned(),
            "probe_timeout_secs" => {
                if let Some(v) = Self::parse_u64(key, value) {
                    self.probe_timeout_secs = v;
                }
            }
            "login_timeout_secs" => {
                if let Some(v) = Self::parse_u64(key, value) {
                    self.login_timeout_secs = v;
                }
            }
            "retry_delay_secs" => {
                if let Some(v) = Self::parse_u64(key, value) {
                    self.retry_delay_secs = v;
                }
            }
            "ok_sleep_secs" => {
                if let Some(v) = Self::parse_u64(key, value) {
                    self.ok_sleep_secs = v;
                }
            }
            "expect_204_response" => {
                if let Some(v) = Self::parse_bool(key, value) {
                    self.expect_204_response = v;
                }
            }
            _ => println!("unknown config key: {key}"),
        }
    }

    pub fn from_file(path: &str) -> Option<Self> {
        println!("reading configuration from file: {path}");

        let metadata = fs::metadata(&path)
            .inspect_err(|err| println!("failed to read metadata from {path}: {err}"))
            .ok()?;

        if metadata.len() > 10240 {
            println!("configuration file is too large (max 10KB)");
            return None;
        }

        let raw = fs::read(&path)
            .inspect_err(|err| println!("failed to read from {path}: {err}"))
            .ok()?;

        let configuration = String::from_utf8(raw)
            .inspect_err(|err| println!("failed to parse content of {path}: {err}"))
            .ok()?;

        let mut lines = configuration.lines();
        let username = lines.next();
        let password = lines.next();
        let mut result = Self::validate_and_assemble(username, password)
            .inspect_err(|err| println!("invalid configuration: {err}"))
            .ok()?;

        for line in lines {
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                result.apply_override(key.trim(), value.trim());
            } else {
                println!("ignoring invalid config line: {line}");
            }
        }

        Some(result)
    }

    fn candidate_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        for name in MAIN_DEFAULT_CONFIG_PATHS {
            paths.push(PathBuf::from(name));
        }

        if let Ok(exe_path) = std::env::current_exe()
            && let Some(exe_dir) = exe_path.parent()
        {
            for name in MAIN_DEFAULT_CONFIG_PATHS {
                let candidate = exe_dir.join(name);
                if !paths.iter().any(|path| path == &candidate) {
                    paths.push(candidate);
                }
            }
        }

        paths
    }

    fn from_default_locations() -> Option<Self> {
        println!("reading configuration from default locations");

        for path in Self::candidate_paths() {
            if Path::new(&path).is_file() {
                if let Some(config) = Self::from_file(&path.to_string_lossy()) {
                    println!("using configuration file: {}", path.display());
                    return Some(config);
                }
            }
        }

        println!("no default configuration file found");
        None
    }

    pub fn from_args_or_default_file() -> Option<Self> {
        println!("reading configuration from arguments");

        let args: Vec<String> = std::env::args().skip(1).collect();

        match args.len() {
            0 => Self::from_default_locations(),
            1 => Self::from_file(&args[0]),
            _ => {
                println!("at most 1 argument is supported, got {}", args.len());
                None
            }
        }
    }

    fn from_embedded() -> Option<Self> {
        if !embedded_constants::EMBEDDED_CONSTANTS_AVAILABLE {
            println!("src/constants.rs is missing, falling back to configuration file mode");
            return None;
        }

        if !embedded_constants::USE_EMBEDDED_CONFIG {
            return None;
        }

        if embedded_constants::EMBEDDED_USERNAME.trim().is_empty()
            || embedded_constants::EMBEDDED_PASSWORD.trim().is_empty()
        {
            println!("embedded configuration is enabled but username or password is empty");
            return None;
        }

        Some(Self {
            username: embedded_constants::EMBEDDED_USERNAME.to_owned(),
            password: embedded_constants::EMBEDDED_PASSWORD.to_owned(),
            test_url: embedded_constants::DEFAULT_TEST_URL.to_owned(),
            probe_timeout_secs: embedded_constants::DEFAULT_PROBE_TIMEOUT_SECS,
            login_timeout_secs: embedded_constants::DEFAULT_LOGIN_TIMEOUT_SECS,
            retry_delay_secs: embedded_constants::DEFAULT_RETRY_DELAY_SECS,
            ok_sleep_secs: embedded_constants::DEFAULT_OK_SLEEP_SECS,
            expect_204_response: embedded_constants::DEFAULT_EXPECT_204_RESPONSE,
        })
    }
}

fn main() {
    println!("准备调用 login");
    let config = if let Some(config) = Config::from_embedded() {
        println!("using embedded configuration");
        config
    } else {
        Config::from_args_or_default_file().expect("FAILED to read configuration from file")
    };
    println!("开始循环 begin loop");

    loop {
        match login(&config) {
            Ok(_) => {
                println!("login 完成 sleeping {} seconds", config.ok_sleep_secs);
                thread::sleep(Duration::from_secs(config.ok_sleep_secs));
            }
            Err(e) => {
                println!("error 出错! {}", e);
                thread::sleep(Duration::from_secs(config.retry_delay_secs));
            }
        }
    }
}
