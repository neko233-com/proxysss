// Native, same-host mixed regression diagnostic. This is not nginx superiority
// evidence or a production release gate. Fixtures and reports stay in the repo.
package main

import (
	"bytes"
	"context"
	"crypto/ecdsa"
	"crypto/elliptic"
	"crypto/rand"
	"crypto/sha256"
	"crypto/tls"
	"crypto/x509"
	"crypto/x509/pkix"
	"encoding/hex"
	"encoding/json"
	"encoding/pem"
	"errors"
	"flag"
	"fmt"
	"io"
	"math/big"
	"net"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"runtime"
	"sort"
	"strconv"
	"strings"
	"sync"
	"time"
)

type row struct {
	Scenario           string
	Mode               string
	Pass               int
	Ops, P50, P95, P99 float64
	Errors             int
}
type comparison struct {
	Scenario           string
	OpsRatio, P95Ratio float64
	Errors             int
}
type child struct {
	cmd *exec.Cmd
	log *os.File
}

func (c *child) stop() {
	if c != nil {
		_ = c.cmd.Process.Kill()
		_ = c.cmd.Wait()
		_ = c.log.Close()
	}
}
func start(binary, work, logfile, role string, args ...string) (*child, error) {
	log, err := os.Create(logfile)
	if err != nil {
		return nil, err
	}
	cmd := exec.Command(binary, args...)
	hideWindow(cmd)
	cmd.Dir = work
	cmd.Env = append(os.Environ(), "TOKIO_WORKER_THREADS=2")
	cmd.Stdout = log
	cmd.Stderr = log
	if err = startInRole(cmd, role); err != nil {
		log.Close()
		return nil, err
	}
	return &child{cmd, log}, nil
}
func port() int {
	l, e := net.Listen("tcp", "127.0.0.1:0")
	must(e)
	defer l.Close()
	return l.Addr().(*net.TCPAddr).Port
}
func udpPort() int {
	l, e := net.ListenPacket("udp4", "127.0.0.1:0")
	must(e)
	defer l.Close()
	return l.LocalAddr().(*net.UDPAddr).Port
}
func udpReady(port int) bool {
	conn, e := net.Dial("udp4", fmt.Sprintf("127.0.0.1:%d", port))
	if e != nil {
		return false
	}
	defer conn.Close()
	_ = conn.SetDeadline(time.Now().Add(100 * time.Millisecond))
	if _, e = conn.Write([]byte("ready")); e != nil {
		return false
	}
	buf := make([]byte, 16)
	n, e := conn.Read(buf)
	return e == nil && string(buf[:n]) == "ready"
}
func tcpReady(port int) bool {
	conn, e := net.DialTimeout("tcp", fmt.Sprintf("127.0.0.1:%d", port), 100*time.Millisecond)
	if e != nil {
		return false
	}
	defer conn.Close()
	_ = conn.SetDeadline(time.Now().Add(100 * time.Millisecond))
	if _, e = conn.Write([]byte("ready")); e != nil {
		return false
	}
	buf := make([]byte, 5)
	_, e = io.ReadFull(conn, buf)
	return e == nil && string(buf) == "ready"
}
func must(err error) {
	if err != nil {
		panic(err)
	}
}
func yamlPath(path string) string { return strings.ReplaceAll(filepath.ToSlash(path), "'", "''") }
func cert(work string) {
	key, e := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	must(e)
	template := &x509.Certificate{SerialNumber: big.NewInt(1), Subject: pkix.Name{CommonName: "localhost"}, NotBefore: time.Now().Add(-time.Hour), NotAfter: time.Now().Add(time.Hour), IPAddresses: []net.IP{net.ParseIP("127.0.0.1")}, KeyUsage: x509.KeyUsageDigitalSignature, ExtKeyUsage: []x509.ExtKeyUsage{x509.ExtKeyUsageServerAuth}}
	der, e := x509.CreateCertificate(rand.Reader, template, template, &key.PublicKey, key)
	must(e)
	priv, e := x509.MarshalECPrivateKey(key)
	must(e)
	must(os.WriteFile(filepath.Join(work, "cert.pem"), pem.EncodeToMemory(&pem.Block{Type: "CERTIFICATE", Bytes: der}), 0600))
	must(os.WriteFile(filepath.Join(work, "key.pem"), pem.EncodeToMemory(&pem.Block{Type: "EC PRIVATE KEY", Bytes: priv}), 0600))
}
func memory(pid int) map[string]uint64 {
	result := map[string]uint64{}
	if runtime.GOOS == "linux" {
		data, _ := os.ReadFile(fmt.Sprintf("/proc/%d/status", pid))
		for _, key := range []string{"VmRSS", "VmHWM"} {
			re := regexp.MustCompile(`(?m)^` + key + `:\s+(\d+)`)
			m := re.FindSubmatch(data)
			if len(m) > 1 {
				v, _ := strconv.ParseUint(string(m[1]), 10, 64)
				result[key+"_bytes"] = v * 1024
			}
		}
	}
	if runtime.GOOS == "windows" {
		cmd := exec.Command("powershell.exe", "-NoProfile", "-Command", fmt.Sprintf("Get-Process -Id %d | Select-Object WorkingSet64,PeakWorkingSet64 | ConvertTo-Json -Compress", pid))
		hideWindow(cmd)
		data, e := cmd.Output()
		if e == nil {
			_ = json.Unmarshal(data, &result)
		}
	}
	return result
}
func metric(output, label string) float64 {
	re := regexp.MustCompile(`(?m)^` + regexp.QuoteMeta(label) + `\s*:\s*([\d.]+)`)
	m := re.FindStringSubmatch(output)
	if len(m) < 2 {
		return -1
	}
	v, e := strconv.ParseFloat(m[1], 64)
	if e != nil {
		return -1
	}
	return v
}
func median(values []float64) float64 {
	sort.Float64s(values)
	n := len(values)
	if n%2 == 1 {
		return values[n/2]
	}
	return (values[n/2-1] + values[n/2]) / 2
}
func safeWork(root, relative string) string {
	path := filepath.Join(root, relative)
	rel, e := filepath.Rel(root, path)
	must(e)
	if strings.HasPrefix(rel, "..") || filepath.IsAbs(rel) {
		panic("artifact path escapes project")
	}
	for p := path; p != root; p = filepath.Dir(p) {
		info, e := os.Lstat(p)
		if e == nil && info.Mode()&os.ModeSymlink != 0 {
			panic("artifact path crosses symlink")
		}
	}
	return path
}
func main() {
	if err := run(); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
func run() error {
	binaryFlag := flag.String("binary", "target/release-fast/proxysss.exe", "gateway binary")
	duration := flag.Int("seconds", 4, "seconds per concurrent wave")
	repetitions := flag.Int("repetitions", 4, "even number of reversed-order repetitions")
	flag.Parse()
	if *duration < 1 || *repetitions < 2 || *repetitions%2 != 0 {
		return errors.New("seconds >=1 and even repetitions >=2 required")
	}
	root, e := filepath.Abs(".")
	if e != nil {
		return e
	}
	if _, e = os.Stat(filepath.Join(root, "Cargo.toml")); e != nil {
		return errors.New("run from the proxysss project root")
	}
	binary, e := filepath.Abs(*binaryFlag)
	if e != nil {
		return e
	}
	output := safeWork(root, filepath.Join(".tmp", "platform-performance", runtime.GOOS))
	must(os.MkdirAll(output, 0755))
	lockPath := safeWork(root, filepath.Join(".tmp", "platform-performance-"+runtime.GOOS+".lock"))
	lock, e := os.OpenFile(lockPath, os.O_CREATE|os.O_EXCL|os.O_WRONLY, 0600)
	if e != nil {
		return errors.New("a platform diagnostic is already running; inspect the project lock")
	}
	defer func() { lock.Close(); os.Remove(lockPath) }()
	// Fixed report directory: shorter subsequent runs must not leave older waves.
	must(os.RemoveAll(output))
	must(os.MkdirAll(output, 0755))
	work := safeWork(root, filepath.Join(".tmp", "platform-performance", runtime.GOOS, "work"))
	must(os.RemoveAll(work))
	must(os.MkdirAll(filepath.Join(work, "public"), 0755))
	defer os.RemoveAll(work)
	cpuRoles, e := initCPURoles()
	if e != nil {
		return e
	}
	body := strings.Repeat("proxysss-static-", 256)
	must(os.WriteFile(filepath.Join(work, "public", "small.txt"), []byte(body), 0600))
	must(os.WriteFile(filepath.Join(work, "public", "large.bin"), make([]byte, 2*1024*1024), 0600))
	must(os.WriteFile(filepath.Join(work, "public", "hot.txt"), []byte(body), 0600))
	cert(work)
	bp, wp, tp, up, sp := port(), port(), port(), udpPort(), port()
	var backends []*child
	defer func() {
		for _, p := range backends {
			p.stop()
		}
	}()
	for _, demo := range []struct {
		name string
		port int
	}{{"http-echo", bp}, {"ws-echo", wp}, {"tcp-echo", tp}, {"udp-echo", up}} {
		p, e := start(binary, work, filepath.Join(output, demo.name+".log"), demo.name, "demo", demo.name, "--listen", fmt.Sprintf("127.0.0.1:%d", demo.port))
		if e != nil {
			return e
		}
		backends = append(backends, p)
	}
	sse := &http.Server{Addr: fmt.Sprintf("127.0.0.1:%d", sp), Handler: http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "text/event-stream")
		for i := 0; i < 8; i++ {
			fmt.Fprint(w, "data: platform-stream\n\n")
			if f, ok := w.(http.Flusher); ok {
				f.Flush()
			}
		}
	})}
	go func() { _ = sse.ListenAndServe() }()
	defer sse.Close()
	time.Sleep(time.Second)
	var rows []row
	var memories []map[string]any
	for pass := 0; pass < *repetitions; pass++ {
		modes := []string{"off", "on"}
		if pass%2 == 1 {
			modes = []string{"on", "off"}
		}
		for _, mode := range modes {
			gp, lp, gt, gu := port(), port(), port(), udpPort()
			enabled := mode == "on"
			master := true
			if runtime.GOOS == "linux" {
				master = enabled
			}
			config := fmt.Sprintf(`http:
  plain_bind: 127.0.0.1:%d
  tls_bind: 127.0.0.1:%d
  h3_bind: ''
  tls:
    mode: manual
    cert_path: '%s/cert.pem'
    key_path: '%s/key.pem'
admin:
  enabled: false
script:
  enabled: false
plugins:
  enabled: false
logging:
  access_log: false
  error_log_path: '%s/error.log'
runtime:
  performance:
    enabled: %t
    traffic_profile: small
    windows:
      enabled: %t
    macos:
      enabled: %t
  hot_reload:
    enabled: false
load_balance:
  active_health:
    enabled: false
services:
  static_sites:
    - name: static
      path_prefix: /static
      root: '%s/public'
  reverse_proxy:
    routes:
      - name: http
        path_prefix: /proxy
        upstream: http://127.0.0.1:%d
      - name: sse
        path_prefix: /sse
        upstream: http://127.0.0.1:%d
      - name: websocket
        path_prefix: /ws
        upstream: ws://127.0.0.1:%d
tcp:
  listeners:
    - name: stream
      bind: 127.0.0.1:%d
      upstream: 127.0.0.1:%d
udp:
  listeners:
    - name: datagrams
      bind: 127.0.0.1:%d
      upstream: 127.0.0.1:%d
`, gp, lp, yamlPath(work), yamlPath(work), yamlPath(work), master, enabled, enabled, yamlPath(work), bp, sp, wp, gt, tp, gu, up)
			configPath := filepath.Join(work, "gateway.yaml")
			must(os.WriteFile(configPath, []byte(config), 0600))
			tag := fmt.Sprintf("%d-%s", pass+1, mode)
			gateway, e := start(binary, work, filepath.Join(output, tag+"-gateway.log"), "gateway", "-c", configPath, "run")
			if e != nil {
				return e
			}
			waveErr := func() error {
				defer gateway.stop()
				base := fmt.Sprintf("http://127.0.0.1:%d", gp)
				ready := false
				client := http.Client{Timeout: time.Second}
				for n := 0; n < 100; n++ {
					// Public listeners bind only after config-load warm-up.
					r, e := client.Get(base + "/static/small.txt")
					if e == nil {
						b, _ := io.ReadAll(r.Body)
						r.Body.Close()
						if r.StatusCode == 200 && string(b) == body {
							ready = true
							break
						}
					}
					time.Sleep(100 * time.Millisecond)
				}
				if !ready {
					return fmt.Errorf("gateway readiness failed: %s", tag)
				}
				ready = false
				for n := 0; n < 50; n++ {
					if udpReady(gu) {
						ready = true
						break
					}
					time.Sleep(100 * time.Millisecond)
				}
				if !ready {
					return fmt.Errorf("gateway UDP readiness failed: %s", tag)
				}
				ready = false
				// Each listener starts independently. HTTP readiness alone does not
				// establish that TLS material or the TCP accept loop is ready yet.
				transport := &http.Transport{TLSClientConfig: &tls.Config{InsecureSkipVerify: true}} // generated, loopback-only fixture certificate
				tlsClient := &http.Client{Transport: transport, Timeout: time.Second}
				defer transport.CloseIdleConnections()
				for n := 0; n < 50; n++ {
					r, err := tlsClient.Get(fmt.Sprintf("https://127.0.0.1:%d/static/small.txt", lp))
					if err == nil {
						data, _ := io.ReadAll(r.Body)
						r.Body.Close()
						if r.StatusCode == 200 && string(data) == body && tcpReady(gt) {
							ready = true
							break
						}
					}
					time.Sleep(100 * time.Millisecond)
				}
				if !ready {
					return fmt.Errorf("gateway HTTPS/TCP readiness failed: %s", tag)
				}
				stopHot := make(chan struct{})
				hotDone := make(chan struct{})
				defer func() { close(stopHot); <-hotDone }()
				go func() {
					defer close(hotDone)
					ticker := time.NewTicker(500 * time.Millisecond)
					defer ticker.Stop()
					for {
						select {
						case <-ticker.C:
							_ = os.WriteFile(filepath.Join(work, "public", "hot.txt"), []byte(body), 0600)
						case <-stopHot:
							return
						}
					}
				}()
				scenario := []struct {
					name string
					args []string
				}{
					{"static-small", []string{"http", "--url", base + "/static/small.txt", "--concurrency", "8"}},
					{"static-large", []string{"http", "--url", base + "/static/large.bin", "--concurrency", "2"}},
					{"cdn-update", []string{"http", "--url", base + "/static/hot.txt", "--concurrency", "8"}},
					{"https-static", []string{"http", "--url", fmt.Sprintf("https://127.0.0.1:%d/static/small.txt", lp), "--concurrency", "8", "--insecure"}},
					{"http-proxy", []string{"http", "--url", base + "/proxy", "--concurrency", "8"}},
					{"sse", []string{"sse", "--url", base + "/sse", "--concurrency", "8"}},
					{"websocket", []string{"websocket", "--url", fmt.Sprintf("ws://127.0.0.1:%d/ws", gp), "--connections", "8", "--payload-bytes", "128"}},
					{"game-tcp", []string{"tcp", "--addr", fmt.Sprintf("127.0.0.1:%d", gt), "--connections", "8", "--payload-bytes", "128"}},
					{"tcp", []string{"tcp", "--addr", fmt.Sprintf("127.0.0.1:%d", gt), "--connections", "8", "--payload-bytes", "1024"}},
					{"udp", []string{"udp", "--addr", fmt.Sprintf("127.0.0.1:%d", gu), "--connections", "8", "--payload-bytes", "256"}},
				}
				startAt := strconv.FormatInt(time.Now().Add(2*time.Second).UnixMilli(), 10)
				var wg sync.WaitGroup
				var mu sync.Mutex
				var failures []error
				for _, s := range scenario {
					wg.Add(1)
					go func(name string, args []string) {
						defer wg.Done()
						args = append([]string{"bench"}, args...)
						args = append(args, "--duration-secs", strconv.Itoa(*duration), "--start-at-unix-ms", startAt)
						ctx, cancel := context.WithTimeout(context.Background(), time.Duration(*duration+20)*time.Second)
						defer cancel()
						cmd := exec.CommandContext(ctx, binary, args...)
						hideWindow(cmd)
						cmd.Dir = work
						cmd.Env = append(os.Environ(), "TOKIO_WORKER_THREADS=1")
						var buffer bytes.Buffer
						cmd.Stdout = &buffer
						cmd.Stderr = &buffer
						e := startInRole(cmd, name)
						if e == nil {
							e = cmd.Wait()
						}
						data := buffer.Bytes()
						_ = os.WriteFile(filepath.Join(output, tag+"-"+name+".log"), data, 0600)
						mu.Lock()
						defer mu.Unlock()
						if e != nil {
							failures = append(failures, fmt.Errorf("%s %s: %w", tag, name, e))
							return
						}
						text := string(data)
						parsed := row{name, mode, pass + 1, metric(text, "ops/sec"), metric(text, "latency p50"), metric(text, "latency p95"), metric(text, "latency p99"), int(metric(text, "errors"))}
						if parsed.Ops <= 0 || parsed.P50 < 0 || parsed.P95 < 0 || parsed.P99 < 0 || parsed.Errors < 0 {
							failures = append(failures, fmt.Errorf("missing/invalid benchmark metrics: %s %s", tag, name))
							return
						}
						rows = append(rows, parsed)
					}(s.name, s.args)
				}
				wg.Wait()
				memories = append(memories, map[string]any{"mode": mode, "pass": pass + 1, "memory": memory(gateway.cmd.Process.Pid)})
				if len(failures) > 0 {
					return errors.Join(failures...)
				}
				return nil
			}()
			if waveErr != nil {
				return waveErr
			}
			fmt.Println("mixed wave complete:", tag)
		}
	}
	names := map[string]bool{}
	for _, r := range rows {
		names[r.Scenario] = true
	}
	var comparisons []comparison
	for name := range names {
		var on, off, on95, off95 []float64
		errorCount := 0
		for _, r := range rows {
			if r.Scenario != name {
				continue
			}
			errorCount += r.Errors
			if r.Mode == "on" {
				on = append(on, r.Ops)
				on95 = append(on95, r.P95)
			} else {
				off = append(off, r.Ops)
				off95 = append(off95, r.P95)
			}
		}
		comparisons = append(comparisons, comparison{name, median(on) / median(off), median(on95) / median(off95), errorCount})
	}
	sort.Slice(comparisons, func(i, j int) bool { return comparisons[i].Scenario < comparisons[j].Scenario })
	data, e := os.ReadFile(binary)
	must(e)
	hash := sha256.Sum256(data)
	report := map[string]any{"os": runtime.GOOS, "arch": runtime.GOARCH, "cpu_threads": runtime.NumCPU(), "binary_sha256": hex.EncodeToString(hash[:]), "seconds": *duration, "repetitions": *repetitions, "rows": rows, "comparisons": comparisons, "memory": memories, "scope": "Same-host concurrent 10-scenario diagnostic. Windows/macOS compare platform socket settings off/on; Linux compares existing runtime performance off/on. No nginx, cross-host, or production superiority claim. Noise and shared client/backend CPU can dominate small differences."}
	report["cpu_roles"] = cpuRoles
	result, e := json.MarshalIndent(report, "", "  ")
	must(e)
	must(os.WriteFile(filepath.Join(output, "result.json"), result, 0600))
	fmt.Println("report:", filepath.Join(output, "result.json"))
	for _, r := range comparisons {
		fmt.Printf("%-14s ops %.3fx p95 %.3fx errors %d\n", r.Scenario, r.OpsRatio, r.P95Ratio, r.Errors)
	}
	for _, r := range comparisons {
		if r.Errors > 0 {
			return errors.New("mixed scenario errors detected; inspect raw logs")
		}
	}
	return nil
}
