# SMT5V 反向合体计算器

[English](README.md) | 简体中文

《真·女神转生 V Vengeance》的反向合体计算器。根据目标恶魔、必要技能和 DLC 设置查询合法路线，并支持逐节点更换配方。

在线使用：<https://megaten.rasp505.top/>

## 功能

- 支持普通合体、特殊合体和升级习得技能。
- 精确统计深度范围内的合法路线，默认展示最浅方案；支持更换配方、展开与收起分支、恢复默认路线。
- 提供简体中文、繁体中文、英文和日文界面，支持跨语言名称搜索。
- 支持浅色和深色主题，可自动跟随系统外观。
- 适配桌面、平板和手机，支持键盘与触控操作；搜索条件、语言和外观偏好保存在本地浏览器。

## 技术与结构

搜索在 Web Worker 中执行，完整路线空间以压缩 DAG 表示，无需后端服务，可作为静态站点部署。

- `crates/smt5-fusion-core`：游戏数据、合体规则、路线搜索、精确计数与重放验证。
- `crates/smt5-fusion-web`：浏览器界面、Worker、本地化及交互状态。
- `tools/android-core-runner`：Android 原生核心性能测试工具。
- `tools/web-config-sync`：启动 HTML 与 UI 编译配置同步。

## 本地开发

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
trunk serve --config crates/smt5-fusion-web/Trunk.toml
```

访问 <http://127.0.0.1:8080/>。

## 构建与部署

```sh
trunk build --release --locked --config crates/smt5-fusion-web/Trunk.toml --public-url /
```

默认构建正式版。添加 `--features preview` 可启用预览构建，为语言、主题和搜索设置使用独立存储键。构建类型由条件编译决定，与部署地址无关。启动 HTML 与 UI 编译配置自动同步，Worker 的构建配置不受影响。

## 测试

```sh
cargo test --locked --workspace --all-features
cargo test --locked -p smt5-fusion-web --no-default-features --features ui,worker
```

第一条命令包含 Preview 和 HTML 钩子测试，第二条覆盖正式 UI 配置。Rust 测试覆盖编译配置、存储键、启动 HTML、主题状态与配色规则。

## 许可证

[MIT](LICENSE)
