# HUST-Network-Login

极简主义的华中科技大学校园网络认证工具，支持有线和无线网络。下载即用，大小约为 400k，静态链接无依赖。为路由器等嵌入式设备开发，支持所有主流硬件软件平台。No Python, No Dependencies, No Bullshit.

## 许可证说明

本项目基于原始MIT许可证的代码（版权归 [Black Binary](https://github.com/black-binary) 所有）进行开发。根据MIT许可证的条款，该衍生版本同时在以下许可证下发布：

- **MIT License** - 原始代码许可证（详见 [LICENSE-MIT](LICENSE-MIT)）
- **AGPL-3.0-or-later** - 衍生版本许可证（详见 [LICENSE-AGPL](LICENSE-AGPL)）

您可以选择在上述任一许可证下使用本软件。如果您修改并分发本软件，或通过网络提供服务，请遵守相应许可证的要求。

### 原作者

- 原始项目：[black-binary/hust-network-login](https://github.com/black-binary/hust-network-login)
- 版权所有：Copyright (c) 2020 Black Binary

## 使用

从 Release 下载对应硬件和操作系统平台的可执行文件。

两种配置方式：

1) 内置配置（默认开启）：在 [src/constants.rs](src/constants.rs) 里把 `USE_EMBEDDED_CONFIG` 设为 `true`，并填写 `EMBEDDED_USERNAME` / `EMBEDDED_PASSWORD`。重新编译后程序会直接使用这些值，不再读取外部文件。

2) 外部配置文件：将 `USE_EMBEDDED_CONFIG` 设为 `false`，运行时从文件读取。配置文件前两行仍是用户名和密码，后续可选写入键值覆盖默认参数，例如

```text
M2020123123
mypasswordmypassword
hust_url=http://www.hust.edu.cn
probe_timeout_secs=3
login_timeout_secs=10
retry_delay_secs=1
ok_sleep_secs=15
```

保存为 my.conf 后运行

```shell
./hust-network-login ./my.conf
```

成功后程序默认每隔 15s 测试网络连通性，失败则重新登录，可通过上面的覆盖项调整间隔和超时。

## 编译

编译本地平台只需要使用 `cargo`。

```shell
cargo build --release
strip ./target/release/hust-network-login
```

strip 可以去除调试符号表，将体积减少到 500k 以下。

交叉编译推荐使用 `cross`，当然你也可以自己手动配置工具链。

```shell
cargo install cross
cross build --release --target mips-unknown-linux-musl
mips-linux-gnu-strip ./target/mips-unknown-linux-musl/release/hust-network-login
```

你应当根据自己的路由器平台选择硬件平台。支持的目标平台戳[这里](https://github.com/rust-embedded/cross)。
