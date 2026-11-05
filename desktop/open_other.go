//go:build !windows

package main

import (
	"os/exec"
	"runtime"
)

func openWindow(url string) {
	for _, c := range [][]string{
		{"google-chrome", "--app=" + url},
		{"chromium", "--app=" + url},
		{"microsoft-edge", "--app=" + url},
	} {
		if runtime.GOOS == "linux" {
			if p, err := exec.LookPath(c[0]); err == nil && exec.Command(p, c[1:]...).Start() == nil {
				return
			}
		}
	}
	if runtime.GOOS == "darwin" {
		exec.Command("open", url).Start()
		return
	}
	exec.Command("xdg-open", url).Start()
}
