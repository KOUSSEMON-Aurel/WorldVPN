# Build targets for vpn-go
# Requires Android NDK r25+ in NDK_PATH or $ANDROID_NDK_HOME

NDK_PATH ?= $(ANDROID_NDK_HOME)
MIN_SDK   ?= 29

TOOLCHAIN_ARM64 = $(NDK_PATH)/toolchains/llvm/prebuilt/linux-x86_64/bin
CC_ARM64        = $(TOOLCHAIN_ARM64)/aarch64-linux-android$(MIN_SDK)-clang
CXX_ARM64       = $(TOOLCHAIN_ARM64)/aarch64-linux-android$(MIN_SDK)-clang++

OUT_DIR_ARM64   = ../frontend/worldvpn-mobile/android/app/src/main/jniLibs/arm64-v8a
OUT_DIR_X86     = ../frontend/worldvpn-mobile/android/app/src/main/jniLibs/x86_64

.PHONY: android-arm64 android-x86_64 android-all clean tidy

## Download and tidy Go dependencies
tidy:
	go mod tidy

## Build for Android ARM64 (physical devices)
android-arm64: tidy
	mkdir -p $(OUT_DIR_ARM64)
	CGO_ENABLED=1 \
	GOOS=android \
	GOARCH=arm64 \
	CC=$(CC_ARM64) \
	CXX=$(CXX_ARM64) \
	go build \
		-buildmode=c-shared \
		-trimpath \
		-ldflags="-s -w" \
		-o $(OUT_DIR_ARM64)/libvpngo.so \
		./cmd/libvpngo

## Build for Android x86_64 (emulator)
android-x86_64: tidy
	mkdir -p $(OUT_DIR_X86)
	CGO_ENABLED=1 \
	GOOS=android \
	GOARCH=amd64 \
	CC=$(TOOLCHAIN_ARM64)/x86_64-linux-android$(MIN_SDK)-clang \
	go build \
		-buildmode=c-shared \
		-trimpath \
		-ldflags="-s -w" \
		-o $(OUT_DIR_X86)/libvpngo.so \
		./cmd/libvpngo

## Build both architectures
android-all: android-arm64 android-x86_64

## Verify the .so was produced
check:
	@echo "=== ARM64 ==="
	@ls -lh $(OUT_DIR_ARM64)/libvpngo.so 2>/dev/null || echo "NOT FOUND"
	@echo "=== x86_64 ==="
	@ls -lh $(OUT_DIR_X86)/libvpngo.so 2>/dev/null || echo "NOT FOUND"

clean:
	rm -f $(OUT_DIR_ARM64)/libvpngo.so $(OUT_DIR_X86)/libvpngo.so
	rm -f $(OUT_DIR_ARM64)/libvpngo.h  $(OUT_DIR_X86)/libvpngo.h
