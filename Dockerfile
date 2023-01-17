FROM alpine:3.17

RUN apk add --no-cache openjdk8

WORKDIR /opt/mmfplace

COPY ./target/release/cli ./place
COPY ./extractor/jlibs/ ./extractor/jlibs/
COPY ./config/src/default_config.yml ./config.yml

ENTRYPOINT ["./place"]
