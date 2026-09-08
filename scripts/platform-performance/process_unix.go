//go:build !windows

package main

import "os/exec"

func hideWindow(cmd *exec.Cmd)                  {}
func initCPURoles() (map[string][]int, error)   { return map[string][]int{}, nil }
func startInRole(cmd *exec.Cmd, _ string) error { return cmd.Start() }
