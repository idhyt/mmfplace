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

host-tests:
	mkdir -p tests_output
	cd builder && cargo run -- -i ../tests  --logfile ../tests_output/tests.log -o ../tests_output

test: host-tests

clean:
	rm -rf tests/*.mmfplace
	rm -rf tests_output

cross-build:
	./xbuild

build: cross-build
