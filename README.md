## What

将照片，视频等音频文件按照日期进行分类存放，用于整理相册.

该日期数据来自于[文件元数据](https://github.com/drewnoakes/metadata-extractor), 文件属性看到的时间, 以及特殊标识(如文件名)

整理之前的目录:
```
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
```
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

```
╰─ make cross-build
1) x86_64-unknown-linux-musl
2) aarch64-unknown-linux-musl
3) x86_64-apple-darwin
4) aarch64-apple-darwin
5) x86_64-pc-windows-gnu
选择目标平台的编号:
```

之后将编译出来的`二进制`文件, `tools`目录以及`config.yaml`文件放在同级目录即可在本地运行

```
╰─ mkdir dist
╰─ cp builder/target/x86_64-unknown-linux-musl/release/mmfplace ./dist
╰─ cp builder/config/src/default.yaml ./dist/config.yaml
╰─ cp -rf tools ./dist
╰─ tree -L 2 ./dist
./dist
├── mmfplace
├── config.yaml
└── tools
    ├── metadata-extractor-2.18.0.jar
    └── xmpcore-6.1.11.jar

1 directory, 4 files
```


## Usage

```
Usage: mmfplace [OPTIONS] --input <INPUT>

Options:
  -w, --work-dir <WORK_DIR>  point to the run directory, must have RW permissions
  -i, --input <INPUT>        input file/directory path
  -o, --output <OUTPUT>      output directory path
  -c, --config <CONFIG>      custom config file path
      --logfile <LOGFILE>    custom the logfile path
  -v, --verbose              enable verbose logging
      --test                 test mode, do not copy/move file
  -h, --help                 Print help
  -V, --version              Print version
```

`--config`: 指定config配置, 格式参考[config.yml](./builder/config/src/default.yaml)

可以使用已经构建好的[容器镜像](https://hub.docker.com/r/idhyt/mmfplace)进行处理

```shell
export ROOT_DIR=$(shell pwd)
export BUILD_NAME=idhyt/mmfplace:0.1
docker run -it --rm \
        -v $(ROOT_DIR)/tests:/opt/tests \
        -v $(ROOT_DIR)/tests_output:/opt/tests_output $(BUILD_NAME) \
        --input=/opt/tests --output=/opt/tests_output --logfile=/opt/tests_output/tests.log
```

正式处理前建议先通过 `test` 模式进行测试, 看是否存在错误再进行整理, 命令如下:

```shell
mmfplace --input=/path/to/directory --logfile=/path/to/log.txt --test
```

## 错误处理

常见的错误基本都是尝试解析时间字符串过程中出错，如:

```
DEBUG [extractor::parser] [Exif IFD0] Date/Time = 2012:05:22 15:51:47
DEBUG [extractor::parser] NaiveDateTime try 2012:05:22 15:51:47 as %Y:%m:%d %H:%M:%S %:z, premature end of input
DEBUG [extractor::parser] Utc try 2012:05:22 15:51:47 as %Y:%m:%d %H:%M:%S %:z, premature end of input
DEBUG [extractor::parser] DateTime try 2012:05:22 15:51:47 as %Y:%m:%d %H:%M:%S %:z, premature end of input
ERROR [cli] splits process failed: parse 2012:05:22 15:51:47 failed
```

本地创建配置文件`config.yaml`，加入如下内容:

```
stripes:
  - name: "] Date/Time = "
    regex: "Date/Time = (.*)"
    strptimes:
      - fmt: "%Y:%m:%d %H:%M:%S"
        test: "2012:05:22 15:51:47"
```

之后执行命令加入 `--config` 参数即可


## 特性

问题：某些文件时间信息缺失，每次修改都会以当前时间作为文件时间

如果原始文件已经按照一定的时间进行重命名，且想保留原始时间(从文件名中获取)

如原始文件为 `2018-05-02_13-13-39_dcf485515fb4c7611a704ff7f745abd3.jpg`, 而从解析获取的时间最早是`2021-xx`

如果想保留时间为`2018-05-02_13-13-39`, 在配置文件中加入如下字段

```
additionals:
  - name: "filename" 
    regex: (\d{4}-\d{2}-\d{2}_\d{2}-\d{2}-\d{2})_.*
    strptimes:
      - fmt: "%Y-%m-%d_%H-%M-%S"
        test: "2018-05-02_13-13-39_dcf485515fb4c7611a704ff7f745abd3.jpg"
```
