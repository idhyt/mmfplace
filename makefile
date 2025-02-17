ROOT_DIR=$(shell pwd)
BUILD_IMAGE=messense/rust-musl-cross:x86_64-musl
BUILD_CACHE=~/.cargo/registry:/root/.cargo/registry
BUILD_NAME=idhyt/mmfplace:latest

host-build-linux:
	cd builder && cargo build --release --target x86_64-unknown-linux-musl

docker-build-linux:
	docker run -it --rm -v $(BUILD_CACHE) -v $(ROOT_DIR):/home/rust/src $(BUILD_IMAGE) /bin/bash -c "make host-build-linux"

image-build:
	docker build -f Dockerfile -t $(BUILD_NAME) .

image-release:
	make docker-build-linux
	make image-build
	docker push $(BUILD_NAME)

check-diff:
	@if [ -z "$$(diff -x '*.mmfplace' -r tests_output tests_output_tests)" ];then \
		echo "\033[32m+++ Test success +++\033[0m";\
	else \
		echo "\033[31m--- Test failed ---\033[0m";\
	fi

debug-link:
	mkdir -p builder/target/debug
	cd builder/target/debug && ln -sf ../../../tools ./tools && ln -sf ../../config/src/default.toml ./config.toml

host-tests: clean debug-link
	rm -rf tests_output && mkdir -p tests_output
	cd builder && cargo run -- place -i ../tests -o ../tests_output
	rm -rf tests_output_tests && mkdir -p tests_output_tests
	cd builder && cargo run -- place -i ../tests_output -o ../tests_output_tests
	make check-diff

test: host-tests

clean:
	rm -rf tests/*.mmfplace
	rm -rf tests_output*

cross-build:
	./xbuild

build: cross-build
