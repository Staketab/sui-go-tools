DAEMON_NAME = mcli
BUILD_FLAGS :=

all: go.sum build install

install: go.sum
	@echo "--> Installing mcli"
	go mod tidy
	go install $(BUILD_FLAGS) ./cmd/suid

build: go.sum
	@echo "--> Building mcli"
	go mod tidy
	go build $(BUILD_FLAGS) -o ./build/$(DAEMON_NAME) ./cmd/suid

go.sum: go.mod
	@echo "--> Ensure dependencies have not been modified"
	GO111MODULE=on go mod verify

.DEFAULT_GOAL := build
