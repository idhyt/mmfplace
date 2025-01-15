## What

将照片，视频等音频文件按照日期进行分类存放，用于整理相册.

该日期数据来自于[文件元数据](https://github.com/drewnoakes/metadata-extractor), 文件属性看到的时间, 以及特殊标识(如文件名)

整理之前的目录:

```bash
❯ tree tests
tests
├── 10x12x16bit-CMYK.psd
├── 16color-10x10.bmp
├── 24bpp-10x10.bmp
├── 256color-10x10.bmp
├── 8x4x8bit-Grayscale.eps
├── 8x4x8bit-Grayscale.psd
├── adobeJpeg1.eps
├── adobeJpeg1.jpg
├── adobeJpeg1.jpg.app2
├── crash01.jpg
├── dotnet-256x256-alpha-palette.png
├── gimp-8x12-greyscale-alpha-time-background.png
├── iccDataInvalid1.jpg.app2
├── invalid-iCCP-missing-adler32-checksum.png
├── manuallyAddedThumbnail.jpg
├── mspaint-10x10.gif
├── mspaint-8x10.png
├── nikonMakernoteType1.jpg
├── nikonMakernoteType2b.jpg
...

```

整理之后的目录:

```bash
❯ tree tests_output
tests_output
├── 1996
│   ├── 1996-11-04-14-55-52.jpg
│   ├── 1996-11-10-20-59-21.jpg
│   └── 1996-12-20-18-32-02.jpg
├── 2000
│   ├── 2000-01-01-00-00-00.jpg
│   └── 2000-10-26-16-46-51.jpg
├── 2001
│   ├── 2001-01-28-13-59-33.jpg
│   └── 2001-04-06-11-51-40.jpg
├── 2002
│   ├── 2002-05-08-17-28-03.jpg
│   ├── 2002-06-20-00-00-00.jpg
│   ├── 2002-08-29-17-31-40.jpg
│   ├── 2002-11-16-15-27-01.jpg
│   └── 2002-11-27-18-00-35.jpg
├── 2003
│   └── 2003-11-17-17-23-11.jpg
├── 2004
│   └── 2004-04-02-08-32-09.jpg
├── 2010
│   └── 2010-06-24-14-17-04.jpg
├── 2012
│   ├── 2012-05-22-15-51-47.psd
│   ├── 2012-05-22-15-52-27.psd
...

```

## Build

可以从[releases](https://github.com/idhyt/mmfplace/releases)中下载已编译好的二进制，或者本地构建：

```bash
╰─ make build
1) x86_64-unknown-linux-musl
2) aarch64-unknown-linux-musl
3) x86_64-apple-darwin
4) aarch64-apple-darwin
5) x86_64-pc-windows-gnu
选择目标平台的编号:
```

编译后的文件存放在 `dist` 文件夹

```bash
╰─ tree dist
dist
├── mmfplace.aarch64-apple-darwin.tar.gz
├── mmfplace.aarch64-unknown-linux-musl.tar.gz
├── mmfplace.x86_64-apple-darwin.tar.gz
├── mmfplace.x86_64-pc-windows-gnu.tar.gz
└── mmfplace.x86_64-unknown-linux-musl.tar.gz

0 directories, 5 files

╰─ cd dist && tar -xzvf mmfplace.x86_64-unknown-linux-musl.tar.gz && tree mmfplace.x86_64-unknown-linux-musl
mmfplace.x86_64-unknown-linux-musl
├── config.toml
├── mmfplace
└── tools
    ├── metadata-extractor.jar
    └── xmpcore.jar

1 directory, 4 files
```

## Usage

如果在主机运行，使用前请确保系统中已经安装 java 运行环境，当前测试基于 java-11 环境，其他版本请自行验证。

正式处理前建议先通过 `test` 模式进行测试, 看是否存在错误再进行整理, 命令如下:

```shell
mmfplace place --input=/path/to/directory --logfile=/path/to/log.txt --test
```

参数说明：

```

Usage: mmfplace [OPTIONS] <COMMAND>

Commands:
  place  place files into directories by datetime
  dupf   find duplicate files
  help   Print this message or the help of the given subcommand(s)

Options:
  -o, --output <OUTPUT>    output directory path
  -v, --verbose            enable verbose logging
  -l, --logfile <LOGFILE>  option point to the logfile path, must have RW permissions
  -h, --help               Print help
  -V, --version            Print version
```

## 错误处理

常见的错误基本都是尝试解析时间字符串过程中出错，如:

```
💥 Unrecognized time string format: 2002:11:16 15:27:01, must add parsing format `striptimes` in config.toml`
```

配置文件`config.toml`，加入如下内容:

```toml
striptimes = [
    { "fmt" = "%Y:%m:%d %H:%M:%S", "test" = "2002:11:16 15:27:01" },
]
```

## 特性

问题：某些文件时间信息缺失，每次修改都会以当前时间作为文件时间

如果原始文件已经按照一定的时间进行重命名，且想保留原始时间(从文件名中获取)

如原始文件为 `2018-05-02-13-13-39_dcf485515fb4c7611a704ff7f745abd3.jpg`, 而从解析获取的时间最早是`2021-xx`

如果想保留时间为`2018-05-02-13-13-39`, 在配置文件中加入如下字段

```toml
additionals = [
    { "name" = "filename", dateparse = [
        { "check" = "not check", "regex" = "(\\d{4}-\\d{2}-\\d{2}-\\d{2}-\\d{2}-\\d{2}).*" },
    ], striptimes = [
        { "fmt" = "%Y-%m-%d-%H-%M-%S", "test" = "2018-05-02-13-13-39-01.jpg" },
    ] },
]
```
