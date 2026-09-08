package main

import (
	"fmt"
	"os/exec"
	"sync"
	"syscall"
	"unsafe"
)

var affinityLock sync.Mutex
var roleMasks = map[string]uintptr{}
var kernel = syscall.NewLazyDLL("kernel32.dll")
var currentProcess = kernel.NewProc("GetCurrentProcess")
var getAffinity = kernel.NewProc("GetProcessAffinityMask")
var setAffinity = kernel.NewProc("SetProcessAffinityMask")

func processMask() (uintptr, error) {
	h, _, _ := currentProcess.Call()
	var process, system uintptr
	ok, _, err := getAffinity.Call(h, uintptr(unsafe.Pointer(&process)), uintptr(unsafe.Pointer(&system)))
	if ok == 0 {
		return 0, err
	}
	return process, nil
}
func applyMask(mask uintptr) error {
	h, _, _ := currentProcess.Call()
	ok, _, err := setAffinity.Call(h, mask)
	if ok == 0 {
		return err
	}
	return nil
}
func initCPURoles() (map[string][]int, error) {
	mask, err := processMask()
	if err != nil {
		return nil, err
	}
	var cpus []int
	for i := 0; i < int(unsafe.Sizeof(mask))*8; i++ {
		if mask&(uintptr(1)<<i) != 0 {
			cpus = append(cpus, i)
		}
	}
	roles := map[string][]int{}
	if len(cpus) < 18 {
		return roles, nil
	}
	roles["gateway"] = cpus[:len(cpus)-15]
	names := []string{"http-echo", "ws-echo", "tcp-echo", "udp-echo", "sse-backend", "static-small", "static-large", "cdn-update", "https-static", "http-proxy", "sse", "websocket", "game-tcp", "tcp", "udp"}
	for i, name := range names {
		roles[name] = []int{cpus[len(cpus)-15+i]}
	}
	for role, ids := range roles {
		for _, id := range ids {
			roleMasks[role] |= uintptr(1) << id
		}
	}
	// This process owns the Go SSE fixture; children inherit their declared CPU
	// masks at creation, before the Rust runtime detects available parallelism.
	if err = applyMask(roleMasks["sse-backend"]); err != nil {
		return nil, err
	}
	return roles, nil
}
func startInRole(cmd *exec.Cmd, role string) error {
	if len(roleMasks) == 0 {
		return cmd.Start()
	}
	mask, ok := roleMasks[role]
	if !ok {
		return fmt.Errorf("unknown CPU role: %s", role)
	}
	affinityLock.Lock()
	defer affinityLock.Unlock()
	previous, err := processMask()
	if err != nil {
		return err
	}
	if err = applyMask(mask); err != nil {
		return err
	}
	startErr := cmd.Start()
	restoreErr := applyMask(previous)
	if restoreErr != nil {
		if startErr == nil {
			_ = cmd.Process.Kill()
			_ = cmd.Wait()
		}
		return restoreErr
	}
	return startErr
}

func hideWindow(cmd *exec.Cmd) {
	cmd.SysProcAttr = &syscall.SysProcAttr{HideWindow: true, CreationFlags: 0x08000000}
}
