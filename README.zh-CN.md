# SMT5V 反向合体计算器

[English](README.md) | 简体中文

《真·女神转生 V Vengeance》的反向合体计算器。根据目标恶魔、必要技能和 DLC 设置查询合法路线，并支持逐节点更换配方。

在线使用：<https://megaten.rasp505.top/>

## 功能

- 支持普通合体、特殊合体和升级习得技能。
- 精确统计深度范围内的合法路线，默认展示最浅方案；支持更换配方、展开与收起分支、恢复默认路线。
- 提供简体中文、繁体中文、英文和日文界面，支持跨语言名称搜索。
- 适配桌面、平板和手机，支持键盘与触控操作；搜索条件和语言偏好保存在本地浏览器。

## 技术与结构

采用 Rust、Leptos 和 WebAssembly。搜索在 Web Worker 中执行，完整路线空间以压缩 DAG 表示，无需后端服务，可作为静态站点部署。

- `crates/smt5-fusion-core`：游戏数据、合体规则、路线搜索、精确计数与重放验证。
- `crates/smt5-fusion-web`：浏览器界面、Worker、本地化及交互状态。
- `tools/android-core-runner`：Android 原生核心性能测试工具。

## 本地开发

需要 Rust stable。以下命令均在仓库根目录执行：

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
trunk serve --config crates/smt5-fusion-web/Trunk.toml
```

访问 <http://127.0.0.1:8080/>。开发服务器支持自动重建和页面刷新，并禁用缓存。

## 构建与部署

```sh
trunk build --release --config crates/smt5-fusion-web/Trunk.toml --public-url /
```

## 测试

```sh
cargo test --locked --workspace --all-features
```

## 许可证

[MIT](LICENSE)
