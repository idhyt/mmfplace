ROOT_DIR=$(shell pwd)
BUILD_IMAGE=messense/rust-musl-cross:x86_64-musl
BUILD_CACHE=/tmp/cargo_registry:/root/.cargo/registry
BUILD_NAME=idhyt/mmfplace:0.1

host-build-linux:
	cargo build --release --target x86_64-unknown-linux-musl

docker-build-linux:
	docker run -it --rm -v $(BUILD_CACHE) -v $(ROOT_DIR):/home/rust/src $(BUILD_IMAGE) /bin/bash -c "make host-build-linux"

image-build:
	# [ -f openlogic-openjdk-8u352-b08-linux-x64.tar.gz ] || wget https://builds.openlogic.com/downloadJDK/openlogic-openjdk/8u352-b08/openlogic-openjdk-8u352-b08-linux-x64.tar.gz
	docker build -f Dockerfile -t $(BUILD_NAME) .

release:
	make docker-build-linux
	make image-build
	docker push $(BUILD_NAME)

docker-tests:
	[ -d tests_output ] && rm -rf tests_output || true
	docker run -it --rm -v $(ROOT_DIR)/tests:/opt/tests -v $(ROOT_DIR)/tests_output:/opt/tests_output -e RUST_LOG=DEBUG $(BUILD_NAME) --input=/opt/tests --output=/opt/tests_output --logfile=/opt/tests_output/tests.log
	@tree tests_output
	@echo "\n---------------- ERROR ----------------\n"
	@cat tests_output/tests.log | grep ERROR
	@echo "\n---------------- SUCCESS ----------------\n"
	@cat tests_output/tests.log | grep Success
	@echo "\n---------------- DUPLICATE ----------------\n"
	@cat tests_output/tests.log | grep Duplicate

host-tests:
	RUST_LOG=DEBUG cargo run -- test -i ./tests
