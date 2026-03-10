# md2image

`md2image` 是一个 Rust 命令行工具，用本机 Chrome/Chromium 把 Markdown 渲染成单张纵向 PNG。

## 用法

```bash
md2image input.md -o out.png
cat input.md | md2image -o out.png
md2image input.md --stdout > out.png
cat input.md | md2image --stdout | your-shortcuts-command
md2image --width 1200 input.md -o out.png
md2image --width 960 --scale 2 input.md -o out@2x.png
md2image --width 960 --scale 2 --supersample 2 input.md -o out@2x.png
md2image --timing --width 960 --scale 2 --supersample 2 input.md -o out@2x.png
md2image --browser "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" input.md -o out.png
```

## 参数

- `INPUT`：可选。传入时从文件读取，不读取 stdin。
- `-o, --output <PATH>`：与 `--stdout` 二选一，输出 PNG 到文件。
- `--stdout`：与 `--output` 二选一，把 PNG 二进制直接写到标准输出，适合管道给 macOS 快捷指令之类的后续处理。
- `--width <PX>`：可选，默认 `960`，表示页面的 CSS 排版宽度。
- `--scale <MULTIPLIER>`：可选，默认 `1.0`，在排版宽度不变的前提下增加最终输出像素数。例如 `--width 960 --scale 2` 会输出约 `1920px` 宽的 PNG。
- `--supersample <MULTIPLIER>`：可选，默认 `1.0`，在 `scale` 基础上再提高内部渲染倍率，然后缩回目标输出尺寸，用于进一步改善边缘和文字平滑度。
- `--timing`：打印各阶段耗时，方便定位是浏览器启动、布局、截图还是缩放在拖慢速度。
- `--theme <NAME>`：当前仅支持 `default`。
- `--browser <PATH>`：显式指定 Chrome/Chromium 可执行文件。

## 浏览器依赖

工具需要本机安装 Chrome 或 Chromium。浏览器路径定位优先级如下：

1. `--browser <PATH>`
2. `MD2IMAGE_BROWSER`
3. 自动探测常见安装路径和 PATH 中的浏览器命令

如果自动探测失败，程序会提示使用 `--browser` 或 `MD2IMAGE_BROWSER`。

## 当前支持

- 标题、段落、粗体、斜体
- 引用、无序列表、有序列表
- 行内代码、代码块、分隔线
- 链接文本
- 行内公式、公式块（KaTeX，本地离线资源）

## 数学公式

- 默认支持标准行内公式和块级公式语法。
- 渲染使用内置 KaTeX 资源，不依赖 CDN 或额外系统安装。
- 若公式语法非法或包含 KaTeX 不支持的命令，命令仍会继续输出 PNG，公式区域退化为 KaTeX 的可读错误样式。

## 当前限制

- 不渲染 Markdown 图片
- 不支持表格、任务列表
- 不提供自定义 CSS、语法高亮、纯 Rust 渲染后端

仓库内部已经预留渲染抽象，未来可以补纯 Rust 后端。
