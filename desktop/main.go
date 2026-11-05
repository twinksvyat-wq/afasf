// Сетка — десктоп-обёртка: встроенный интерфейс, локальный сервер и окно браузера в режиме приложения.
package main

import (
	"bytes"
	_ "embed"
	"fmt"
	"io"
	"net"
	"net/http"
	neturl "net/url"
	"os"
	"strings"
	"sync"
	"time"
)

// Учитывает системный прокси (переменные HTTPS_PROXY) — удобно, если API доступен только через него.
var apiClient = &http.Client{Timeout: 4 * time.Minute, Transport: &http.Transport{Proxy: http.ProxyFromEnvironment}}

//go:embed index.html
var page []byte

const port = 47831 // фиксированный порт: второй запуск просто откроет уже работающее окно

var (
	mu      sync.Mutex
	clients int
	seen    bool
)

func main() {
	url := fmt.Sprintf("http://127.0.0.1:%d/", port)
	ln, err := net.Listen("tcp", fmt.Sprintf("127.0.0.1:%d", port))
	if err != nil { // уже запущено
		openWindow(url)
		return
	}

	mux := http.NewServeMux()
	mux.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/" && r.URL.Path != "/index.html" {
			http.NotFound(w, r)
			return
		}
		w.Header().Set("Content-Type", "text/html; charset=utf-8")
		w.Header().Set("Cache-Control", "no-store")
		w.Write(page)
	})
	// Страница держит это соединение открытым; когда все окна закрыты — приложение завершается.
	mux.HandleFunc("/alive", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "text/event-stream")
		w.Header().Set("Cache-Control", "no-cache")
		fl, _ := w.(http.Flusher)
		mu.Lock()
		clients++
		seen = true
		mu.Unlock()
		defer func() { mu.Lock(); clients--; mu.Unlock() }()
		t := time.NewTicker(20 * time.Second)
		defer t.Stop()
		fmt.Fprint(w, ": hi\n\n")
		if fl != nil {
			fl.Flush()
		}
		for {
			select {
			case <-r.Context().Done():
				return
			case <-t.C:
				fmt.Fprint(w, ": ping\n\n")
				if fl != nil {
					fl.Flush()
				}
			}
		}
	})

	// Прокси к API распознавания: окно приложения обходится без ограничений CORS.
	mux.HandleFunc("/proxy", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			http.Error(w, "method", http.StatusMethodNotAllowed)
			return
		}
		if o := r.Header.Get("Origin"); o != "" && o != strings.TrimSuffix(url, "/") {
			http.Error(w, "origin", http.StatusForbidden)
			return
		}
		target := r.Header.Get("X-Target")
		u, err := neturl.Parse(target)
		local := err == nil && (u.Hostname() == "127.0.0.1" || u.Hostname() == "localhost")
		if err != nil || !(u.Scheme == "https" || (u.Scheme == "http" && local)) {
			http.Error(w, "bad target", http.StatusBadRequest)
			return
		}
		body, err := io.ReadAll(io.LimitReader(r.Body, 40<<20))
		if err != nil {
			http.Error(w, err.Error(), http.StatusBadRequest)
			return
		}
		req, err := http.NewRequestWithContext(r.Context(), http.MethodPost, target, bytes.NewReader(body))
		if err != nil {
			http.Error(w, err.Error(), http.StatusBadRequest)
			return
		}
		for _, h := range []string{"Content-Type", "Authorization", "X-Api-Key", "Anthropic-Version", "Anthropic-Beta", "Anthropic-Dangerous-Direct-Browser-Access", "X-Goog-Api-Key", "Http-Referer", "X-Title"} {
			if v := r.Header.Get(h); v != "" {
				req.Header.Set(h, v)
			}
		}
		resp, err := apiClient.Do(req)
		if err != nil {
			http.Error(w, "upstream: "+err.Error(), http.StatusBadGateway)
			return
		}
		defer resp.Body.Close()
		w.Header().Set("Content-Type", resp.Header.Get("Content-Type"))
		w.WriteHeader(resp.StatusCode)
		io.Copy(w, resp.Body)
	})

	go http.Serve(ln, mux)
	openWindow(url)

	// Сторож: выходим, если окно не появилось за 2 минуты или закрыто дольше 8 секунд.
	start := time.Now()
	var zeroSince time.Time
	for range time.Tick(time.Second) {
		mu.Lock()
		c, s := clients, seen
		mu.Unlock()
		if !s {
			if time.Since(start) > 2*time.Minute {
				os.Exit(0)
			}
			continue
		}
		if c > 0 {
			zeroSince = time.Time{}
			continue
		}
		if zeroSince.IsZero() {
			zeroSince = time.Now()
		} else if time.Since(zeroSince) > 8*time.Second {
			os.Exit(0)
		}
	}
}
