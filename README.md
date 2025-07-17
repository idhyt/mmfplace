## What

同一个照片进行复制，传输，移动等操作后，其文件属性时间会被修改或丢失，导致大部分图片处理工具会将最新的修改时间作为照片创建日期，最终的结果就是多年前拍的照片被放在了今年的目录中。

这个工具将获取所有的文件时间信息，并取最早时间作为照片的创建日期，进行分类处理，同时确保对同一文件的每次处理结果都保持一致。

日期数据来自于 [文件元数据](https://github.com/drewnoakes/metadata-extractor) 和 `文件属性` 看到的时间，并取最早时间作为最终结果。

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
├── 1996
│   ├── 11
│   │   ├── withuncompressedycbcrthumbnail3.jpg
│   │   └── withuncompressedycbcrthumbnail.jpg
│   └── 12
│       └── withuncompressedycbcrthumbnail2.jpg
├── 2000
│   ├── 01
│   │   └── withiptc.jpg
│   └── 10
│       └── withuncompressedrgbthumbnail.jpg
├── 2001
│   ├── 01
│   │   └── withexif.jpg
│   └── 04
│       └── nikonmakernotetype1.jpg
├── 2002
│   ├── 05
│   │   └── crash01.jpg
│   ├── 06
│   │   └── withiptcexifgps.jpg
│   ├── 08
│   │   └── nikonmakernotetype2b.jpg
│   └── 11
│       ├── manuallyaddedthumbnail.jpg
│       ├── simple.jpg
│       └── simple.png
├── 2003
│   └── 11
│       └── adobejpeg1.jpg
├── 2004
│   └── 04
│       └── windowsxpfields.jpg
├── 2010
│   └── 06
│       └── withpanasonicfaces.jpg
├── 2012
│   ├── 05
│   │   ├── 10x12x16bit-cmyk.psd
│   │   └── 8x4x8bit-grayscale.psd
│   └── 12
│       └── photoshop-8x12-rgb24-all-metadata.png
├── 2013
│   └── 01
│       └── gimp-8x12-greyscale-alpha-time-background.png
...
```

默认情况下文件会复制到 `%Y/%m/xxx` 并会保留原始文件名， 如 `simple.jpg` -> `2002/11/simple.jpg`

可以通过参数 `--rename-with-ymd` 将文件重命名，如 `simple.jpg` -> `2025/11/2025-11-16.jpg`

```bash
❯ tree tests_output
├── 1996
│   ├── 11
│   │   ├── 1996-11-04.jpg
│   │   └── 1996-11-10.jpg
│   └── 12
│       └── 1996-12-20.jpg
├── 2000
│   ├── 01
│   │   └── 2000-01-01.jpg
│   └── 10
│       └── 2000-10-26.jpg
├── 2001
│   ├── 01
│   │   └── 2001-01-28.jpg
│   └── 04
│       └── 2001-04-06.jpg
├── 2002
│   ├── 05
│   │   └── 2002-05-08.jpg
│   ├── 06
│   │   └── 2002-06-20.jpg
│   ├── 08
│   │   └── 2002-08-29.jpg
│   └── 11
│       ├── 2002-11-16.jpg
│       ├── 2002-11-16_01.jpg
│       └── 2002-11-27.jpg
├── 2003
│   └── 11
│       └── 2003-11-17.jpg
├── 2004
...
```

## Build

[release](https://github.com/idhyt/mmfplace/releases) 直接下载二进制文件

或者本地构建

```bash
cd builder
cargo build -- release
```

或者采用交叉编译

```bash
╰─ ./xbuild
1) x86_64-unknown-linux-musl
2) aarch64-unknown-linux-musl
3) x86_64-apple-darwin
4) aarch64-apple-darwin
5) x86_64-pc-windows-gnu
选择目标平台的编号:
```

交叉编译后的文件存放在 `dist` 文件夹

```bash
╰─ tree dist
dist
├── mmfplace.aarch64-apple-darwin.tar.gz
├── mmfplace.aarch64-unknown-linux-musl.tar.gz
├── mmfplace.x86_64-apple-darwin.tar.gz
├── mmfplace.x86_64-pc-windows-gnu.tar.gz
└── mmfplace.x86_64-unknown-linux-musl.tar.gz
```

## Usage

如果在主机运行，使用前请确保系统中已经安装 java 运行环境，当前测试基于 `java-11` 环境，其他版本请自行验证。

程序执行后会在同级目录下释放必要的依赖文件（请勿删除）

```bash
├── config.toml                 # 配置文件
├── mmfplace.exe                # 主程序
├── place.db                    # 同步数据库，请勿删除，否则无法实现增量同步
└── tools                       # 依赖工具包
    ├── metadata-extractor-2.19.0.jar
    └── xmpcore-6.1.11.jar
```

可配置的几个环境变量：

`MMFPLACE_DATABASE`: 数据库路径
`MMFPLACE_JAVA`: java路径
