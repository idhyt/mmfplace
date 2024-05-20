FROM alpine:3.17

RUN sed -i 's/dl-cdn.alpinelinux.org/mirrors.tuna.tsinghua.edu.cn/g' /etc/apk/repositories && \
    apk add --no-cache openjdk8 tzdata && \
    cp /usr/share/zoneinfo/Asia/Shanghai /etc/localtime && \
    echo "Asia/Shanghai" > /etc/timezone && \
    apk del tzdata

WORKDIR /opt/app

COPY ./builder/target/x86_64-unknown-linux-musl/release/mmfplace ./mmfplace
COPY ./builder/config/src/default.yaml ./config.yml
COPY ./tools ./tools

ENTRYPOINT ["/opt/app/mmfplace"]
