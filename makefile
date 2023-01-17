ROOT_DIR=$(shell pwd)
BUILD_IMAGE=rust:1.66.1-alpine3.17
BUILD_CACHE=/tmp/cargo_registry:/usr/local/cargo/registry
BUILD_NAME=idhyt/mmfplace:0.1

host-build-linux:
	# rustup target add x86_64-unknown-linux-musl && cargo build --release --target x86_64-unknown-linux-gnu
	cargo build --release

docker-build-linux:
	# default musl in alpine, rustup show
	docker run -it --rm -v $(BUILD_CACHE) -v $(ROOT_DIR):/opt/splits4mf -w /opt/splits4mf $(BUILD_IMAGE) sh -c "apk add --no-cache musl-dev make && make host-build-linux"

image-build:
	# [ -f openlogic-openjdk-8u352-b08-linux-x64.tar.gz ] || wget https://builds.openlogic.com/downloadJDK/openlogic-openjdk/8u352-b08/openlogic-openjdk-8u352-b08-linux-x64.tar.gz
	docker build -f Dockerfile -t $(BUILD_NAME) .

docker-tests:
	docker run -it --rm -v $(ROOT_DIR)/tests:/opt/tests -v $(ROOT_DIR)/tests_output:/opt/tests_output -e RUST_LOG=DEBUG $(BUILD_NAME) --input=/opt/tests --output=/opt/tests_output --logfile=/opt/tests_output/tests.log
	tree tests_output
	cat tests_output/tests.log | grep ERROR

host-tests:
	RUST_LOG=DEBUG cargo run -- -i ./tests --test
