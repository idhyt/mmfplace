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

## Usage

```
Usage: mmfplace [OPTIONS] --input <INPUT> [MODE]

Arguments:
  [MODE]
          which mode to used

          [default: copy]

          Possible values:
          - test: test mode, do not copy/move file
          - copy: Copy file to output directory
          - move: Move file to output directory

Options:
  -w, --work-dir <WORK_DIR>
          point to the run directory, must have RW permissions

  -i, --input <INPUT>
          input file/directory path

  -o, --output <OUTPUT>
          output directory path

  -c, --config <CONFIG>
          custom config file path

      --logfile <LOGFILE>
          custom the logfile path

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

`--config`: 可以追加config, 格式参考[config.yml](./src/config/default_config.yml)

可以使用已经构建好的[容器镜像](https://hub.docker.com/r/idhyt/mmfplace)进行处理

可以先运行测试命令 `make docker-tests` 详见 [makefile](./makefile) 中 docker-tests 部分

正式处理前建议先通过 `test` 模式进行测试, 看是否存在错误再进行整理, 命令如下:

```shell
mmfplace test --input=/path/to/directory --logfile=/path/to/log.txt
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
