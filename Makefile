DAEMON_NAME = sui-tool
BUILD_FLAGS :=

all: go.sum build install

install: go.sum
	@echo "--> Installing sui-tool"
	go mod tidy
	go install $(BUILD_FLAGS) ./cmd/$(DAEMON_NAME)

build: go.sum
	@echo "--> Building sui-tool"
	go mod tidy
	go build $(BUILD_FLAGS) -o ./build/$(DAEMON_NAME) ./cmd/$(DAEMON_NAME)

go.sum: go.mod
	@echo "--> Ensure dependencies have not been modified"
	GO111MODULE=on go mod verify

.DEFAULT_GOAL := build
