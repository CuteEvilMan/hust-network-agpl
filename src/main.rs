mod constants;
mod encrypt;
use crate::constants::{
    DEFAULT_LOGIN_TIMEOUT_SECS, DEFAULT_OK_SLEEP_SECS, DEFAULT_PROBE_TIMEOUT_SECS,
    DEFAULT_RETRY_DELAY_SECS, DEFAULT_TEST_URL, EMBEDDED_PASSWORD, EMBEDDED_USERNAME,
    USE_EMBEDDED_CONFIG,
};
use std::fs;
use std::time::Duration;
use std::{io, thread};

fn extract<'a>(text: &'a str, prefix: &'a str, suffix: &'a str) -> io::Result<&'a str> {
    let left = text.find(prefix);
    let right = text.find(suffix);
    if let (Some(l), Some(r)) = (left, right) {
        if l + prefix.len() < r {
            return Ok(&text[l + prefix.len()..r]);
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

    let resp = resp.as_str().map_err(|e| {
        println!("invalid resp format {}", e);
        io::ErrorKind::InvalidData
    })?;
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
}

impl Config {
    fn validate_and_assemble(
        username: Option<&str>,
        password: Option<&str>,
    ) -> Result<Self, &'static str> {
        match (username, password) {
            (Some(_), None) => Err("NO password"),
            (None, Some(_)) => Err("NO username"),
            (None, None) => Err("NO username and password"),
            (Some(username), Some(password)) => Ok(Self {
                username: username.to_owned(),
                password: password.to_owned(),
                test_url: DEFAULT_TEST_URL.to_owned(),
                probe_timeout_secs: DEFAULT_PROBE_TIMEOUT_SECS,
                login_timeout_secs: DEFAULT_LOGIN_TIMEOUT_SECS,
                retry_delay_secs: DEFAULT_RETRY_DELAY_SECS,
                ok_sleep_secs: DEFAULT_OK_SLEEP_SECS,
            }),
        }
    }

    fn parse_u64(key: &str, value: &str) -> Option<u64> {
        value
            .parse::<u64>()
            .inspect_err(|err| println!("invalid value for {key}: {err}"))
            .ok()
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

    pub fn from_args() -> Option<Self> {
        println!("reading configuration from arguments");

        let args: Vec<String> = std::env::args().skip(1).collect();

        if args.len() != 1 {
            println!("exactly 1 argument is required, got {}", args.len());
            return None;
        }

        Self::from_file(&args[0])
    }
}

fn main() {
    println!("准备调用 login");
    let config = if USE_EMBEDDED_CONFIG {
        println!("using embedded configuration");
        Config {
            username: EMBEDDED_USERNAME.to_owned(),
            password: EMBEDDED_PASSWORD.to_owned(),
            test_url: DEFAULT_TEST_URL.to_owned(),
            probe_timeout_secs: DEFAULT_PROBE_TIMEOUT_SECS,
            login_timeout_secs: DEFAULT_LOGIN_TIMEOUT_SECS,
            retry_delay_secs: DEFAULT_RETRY_DELAY_SECS,
            ok_sleep_secs: DEFAULT_OK_SLEEP_SECS,
        }
    } else {
        Config::from_args().expect("FAILED to read configuration from arguments")
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
