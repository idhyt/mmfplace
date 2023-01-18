FROM alpine:3.17

RUN apk add --no-cache openjdk8 tzdata && \
    cp /usr/share/zoneinfo/Asia/Shanghai /etc/localtime && \
    echo "Europe/Moscow" > /etc/timezone && echo "Asia/Shanghai" > /etc/timezone && \
    apk del tzdata

WORKDIR /opt/mmfplace

COPY ./target/release/cli ./place
COPY ./extractor/jlibs/ ./extractor/jlibs/
COPY ./config/src/default_config.yml ./config.yml

ENTRYPOINT ["./place"]
