//go:build android
// +build android

package main

/*
#include <stdlib.h>
#include <jni.h>

static const char* get_jni_string(JNIEnv* env, jstring str) {
    return (*env)->GetStringUTFChars(env, str, 0);
}

static void release_jni_string(JNIEnv* env, jstring str, const char* chars) {
    (*env)->ReleaseStringUTFChars(env, str, chars);
}
*/
import "C"

//export Java_com_aurel_worldvpn_worldvpn_1mobile_WorldVpnService_StartTunnel
func Java_com_aurel_worldvpn_worldvpn_1mobile_WorldVpnService_StartTunnel(env *C.JNIEnv, obj C.jobject, tunFd C.int, configJSON C.jstring) C.int {
	cStr := C.get_jni_string(env, configJSON)
	defer C.release_jni_string(env, configJSON, cStr)

	// Call the main StartTunnel function from tunnel.go
	return StartTunnel(tunFd, cStr)
}

//export Java_com_aurel_worldvpn_worldvpn_1mobile_WorldVpnService_StopTunnel
func Java_com_aurel_worldvpn_worldvpn_1mobile_WorldVpnService_StopTunnel(env *C.JNIEnv, obj C.jobject) {
	StopTunnel()
}
