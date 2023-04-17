FROM alpine:3.17

RUN apk add --no-cache openjdk8 tzdata && \
    cp /usr/share/zoneinfo/Asia/Shanghai /etc/localtime && \
    echo "Asia/Shanghai" > /etc/timezone && \
    apk del tzdata

WORKDIR /opt/app

COPY ./target/x86_64-unknown-linux-musl/release/mmfplace ./mmfplace
COPY ./tools ./tools
COPY ./src/config/default_config.yml ./config.yml

ENTRYPOINT ["/opt/app/mmfplace"]
