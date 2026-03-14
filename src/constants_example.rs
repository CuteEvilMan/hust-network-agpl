pub const DEFAULT_TEST_URL: &str = "http://connectivitycheck.platform.hicloud.com/generate_204";
pub const DEFAULT_TEST_URL: &str = "http://hust.edu.cn/";
pub const DEFAULT_PROBE_TIMEOUT_SECS: u64 = 10;
pub const DEFAULT_LOGIN_TIMEOUT_SECS: u64 = 10;
pub const DEFAULT_RETRY_DELAY_SECS: u64 = 1;
pub const DEFAULT_OK_SLEEP_SECS: u64 = 15;
pub const USE_EMBEDDED_CONFIG: bool = true;
pub const MAIN_DEFAULT_EXPECT_204_RESPONSE: bool = true;
pub const EMBEDDED_USERNAME: &str = "U503203784"; //这里输入学号

pub const EMBEDDED_PASSWORD: &str = "503203784"; //这里输入密码
/*
推荐探测地址（优先选择可直接返回 204 的 HTTP 地址）

1. http://connectivitycheck.platform.hicloud.com/generate_204
	华为 EMUI/HarmonyOS，国内网络通常更稳，推荐优先使用

2. http://connect.rom.miui.com/generate_204
	小米 MIUI，适合作为国内网络下的备用地址

3. http://connectivitycheck.gstatic.com/generate_204
	Google，通用性强，但在部分校园网或国内网络环境下可能超时

4. http://connectivitycheck.android.com/generate_204
	Android 备用地址，可作为 gstatic 的替代

5. http://www.gstatic.com/generate_204
	Google 备用地址，连通性问题与 gstatic 类似

不太推荐但可以了解：

- http://captive.apple.com/hotspot-detect.html
  返回 HTML 页面，不是 204，适合人工测试，不是本项目的首选

- http://www.msftconnecttest.com/connecttest.txt
  返回固定文本 "Microsoft Connect Test"，更适合需要校验响应内容的场景

- http://nmcheck.gnome.org/check_network_status.txt
  返回固定文本，也更适合带内容校验的探测逻辑

建议：

- 当前项目更适合使用“直接返回 204”的地址
- 优先使用 HTTP 地址，避免 HTTPS 证书、重定向或握手带来的额外干扰
- 如果当前地址经常超时，优先切换到 hicloud 或 MIUI 的 204 地址
*/