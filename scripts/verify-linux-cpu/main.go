// Linux-only functional CPU confinement probe; this is not a performance test.
package main

import (
	"bytes"
	"context"
	"flag"
	"fmt"
	"io"
	"net"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"sort"
	"strconv"
	"strings"
	"time"
)

func cpuList(text string) ([]int, error) {
	var cpus []int
	for _, part := range strings.Split(strings.TrimSpace(text), ",") {
		bounds := strings.Split(part, "-")
		first, err := strconv.Atoi(bounds[0])
		if err != nil || first < 0 {
			return nil, fmt.Errorf("bad CPU list %q", text)
		}
		last := first
		if len(bounds) == 2 {
			last, err = strconv.Atoi(bounds[1])
		}
		if err != nil || len(bounds) > 2 || last < first || last > 1048575 {
			return nil, fmt.Errorf("bad CPU list %q", text)
		}
		for cpu := first; cpu <= last; cpu++ {
			cpus = append(cpus, cpu)
		}
	}
	return cpus, nil
}

func statusCPUs(path string) ([]int, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	for _, line := range strings.Split(string(data), "\n") {
		if value, ok := strings.CutPrefix(line, "Cpus_allowed_list:"); ok {
			return cpuList(value)
		}
	}
	return nil, fmt.Errorf("no CPU affinity in %s", path)
}

func freeTCPAddress() (string, error) {
	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		return "", err
	}
	address := listener.Addr().String()
	err = listener.Close()
	return address, err
}

func run() (runErr error) {
	binaryFlag := flag.String("binary", "", "proxysss binary")
	expectedFlag := flag.Int("expected-workers", 0, "assert quota-aware base budget; zero checks reported budget against actual owners")
	flag.Parse()
	binary, err := filepath.Abs(*binaryFlag)
	if err != nil || *binaryFlag == "" {
		return fmt.Errorf("binary required")
	}
	allowed, err := statusCPUs("/proc/self/status")
	if err != nil {
		return err
	}
	launch := []int{allowed[0]}
	if len(allowed) > 1 {
		launch = append(launch, allowed[len(allowed)-1])
	}
	var cpuStrings []string
	for _, cpu := range launch {
		cpuStrings = append(cpuStrings, strconv.Itoa(cpu))
	}
	mask := strings.Join(cpuStrings, ",")
	root, err := os.MkdirTemp("", "proxysss-cpu-probe-")
	if err != nil {
		return err
	}
	defer os.RemoveAll(root)
	tcpBackend, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		return err
	}
	defer tcpBackend.Close()
	go func() {
		for {
			conn, err := tcpBackend.Accept()
			if err != nil {
				return
			}
			go func() { defer conn.Close(); _, _ = io.Copy(conn, conn) }()
		}
	}()
	udpBackend, err := net.ListenPacket("udp", "127.0.0.1:0")
	if err != nil {
		return err
	}
	defer udpBackend.Close()
	go func() {
		buffer := make([]byte, 1024)
		for {
			n, peer, err := udpBackend.ReadFrom(buffer)
			if err != nil {
				return
			}
			_, _ = udpBackend.WriteTo(buffer[:n], peer)
		}
	}()
	httpAddress, err := freeTCPAddress()
	if err != nil {
		return err
	}
	tcpAddress, err := freeTCPAddress()
	if err != nil {
		return err
	}
	udpReservation, err := net.ListenPacket("udp", "127.0.0.1:0")
	if err != nil {
		return err
	}
	udpAddress := udpReservation.LocalAddr().String()
	if err = udpReservation.Close(); err != nil {
		return err
	}
	config := fmt.Sprintf("http:\n  plain_bind: %s\nadmin:\n  enabled: false\nruntime:\n  performance:\n    traffic_profile: balanced\naffinity:\n  enabled: false\nload_balance:\n  retries: { enabled: false }\n  passive_health: { enabled: false }\n  active_health: { enabled: false }\ntcp:\n  listeners:\n    - name: cpu-tcp\n      bind: %s\n      upstream: %s\nudp:\n  listeners:\n    - name: cpu-udp\n      bind: %s\n      upstream: %s\n", httpAddress, tcpAddress, tcpBackend.Addr(), udpAddress, udpBackend.LocalAddr())
	configPath := filepath.Join(root, "proxysss.yaml")
	if err = os.WriteFile(configPath, []byte(config), 0600); err != nil {
		return err
	}
	ctx, cancel := context.WithTimeout(context.Background(), 15*time.Second)
	defer cancel()
	reportCommand := exec.CommandContext(ctx, "taskset", "--cpu-list", mask, binary, "-c", configPath, "config", "performance")
	reportCommand.Dir = root
	report, err := reportCommand.CombinedOutput()
	if err != nil {
		return fmt.Errorf("performance report: %w: %s", err, report)
	}
	workerMatches := regexp.MustCompile(`(?m)^  data_plane_workers: ([0-9]+)$`).FindAllSubmatch(report, -1)
	if len(workerMatches) != 1 {
		return fmt.Errorf("missing unique Linux CPU budget: %s", report)
	}
	workers, err := strconv.Atoi(string(workerMatches[0][1]))
	if err != nil || workers < 1 || workers > len(launch) {
		return fmt.Errorf("invalid budget: %d", workers)
	}
	if *expectedFlag > 0 && workers != *expectedFlag {
		return fmt.Errorf("quota expected %d workers, reported %d", *expectedFlag, workers)
	}
	logPath := filepath.Join(root, "gateway.log")
	log, err := os.Create(logPath)
	if err != nil {
		return err
	}
	defer log.Close()
	command := exec.Command("taskset", "--cpu-list", mask, binary, "-c", configPath)
	command.Dir, command.Stdout, command.Stderr = root, log, log
	if err = command.Start(); err != nil {
		return err
	}
	defer func() {
		_ = command.Process.Kill()
		_ = command.Wait()
		if runErr != nil {
			data, _ := os.ReadFile(logPath)
			runErr = fmt.Errorf("%w; gateway log: %s", runErr, data)
		}
	}()
	client := &http.Client{Timeout: 500 * time.Millisecond}
	defer client.CloseIdleConnections()
	ready := false
	for deadline := time.Now().Add(12 * time.Second); time.Now().Before(deadline); {
		response, requestErr := client.Get("http://" + httpAddress + "/")
		if requestErr == nil {
			body, readErr := io.ReadAll(response.Body)
			response.Body.Close()
			ready = readErr == nil && response.StatusCode == 200 && bytes.Contains(body, []byte("Welcome to proxysss"))
			if ready {
				break
			}
		}
		time.Sleep(50 * time.Millisecond)
	}
	if !ready {
		data, _ := os.ReadFile(logPath)
		return fmt.Errorf("gateway failed readiness: %s", data)
	}
	payload := []byte("cpu-confinement-echo\r\n")
	for _, pair := range [][2]string{{"tcp", tcpAddress}, {"udp", udpAddress}} {
		conn, err := net.DialTimeout(pair[0], pair[1], 3*time.Second)
		if err != nil {
			return err
		}
		defer conn.Close()
		if err = conn.SetDeadline(time.Now().Add(5 * time.Second)); err != nil {
			return err
		}
		if _, err = conn.Write(payload); err != nil {
			return err
		}
		body := make([]byte, len(payload))
		if _, err = io.ReadFull(conn, body); err != nil {
			return fmt.Errorf("%s relay: %w", pair[0], err)
		}
		if !bytes.Equal(body, payload) {
			return fmt.Errorf("%s relay corrupted payload", pair[0])
		}
	}
	// Balanced native owners are prewarmed for both realtime and downloads.
	maskSet := map[int]bool{}
	for _, cpu := range launch {
		maskSet[cpu] = true
	}
	httpShards := map[string]bool{}
	realtime, sendfile, checked := 0, 0, 0
	for deadline := time.Now().Add(2 * time.Second); ; {
		paths, err := filepath.Glob(fmt.Sprintf("/proc/%d/task/*/status", command.Process.Pid))
		if err != nil {
			return err
		}
		httpShards = map[string]bool{}
		realtime, sendfile, checked = 0, 0, 0
		for _, path := range paths {
			cpus, err := statusCPUs(path)
			if os.IsNotExist(err) {
				continue
			}
			if err != nil {
				return err
			}
			for _, cpu := range cpus {
				if !maskSet[cpu] {
					return fmt.Errorf("thread escaped launch affinity %s: %s CPU=%d", mask, path, cpu)
				}
			}
			nameBytes, err := os.ReadFile(filepath.Join(filepath.Dir(path), "comm"))
			if os.IsNotExist(err) {
				continue
			}
			if err != nil {
				return err
			}
			name := strings.TrimSpace(string(nameBytes))
			checked++
			if strings.HasPrefix(name, "proxysss-http-") {
				httpShards[name] = true
			}
			if strings.HasPrefix(name, "proxysss-ws-epo") {
				realtime++
			}
			if strings.HasPrefix(name, "proxysss-sendfi") {
				sendfile++
			}
		}
		if len(httpShards) == workers && realtime == workers && sendfile == 0 {
			break
		}
		if time.Now().After(deadline) {
			return fmt.Errorf("owner count expected=%d HTTP=%d shared-realtime-download=%d standalone-sendfile=%d", workers, len(httpShards), realtime, sendfile)
		}
		time.Sleep(10 * time.Millisecond)
	}
	var names []string
	for name := range httpShards {
		names = append(names, name)
	}
	sort.Strings(names)
	fmt.Printf("Linux CPU confinement passed: taskset=%s base_workers=%d HTTP=%v shared-realtime-download=%d standalone-sendfile=%d checked_threads=%d; HTTP/TCP/UDP payloads passed\n", mask, workers, names, realtime, sendfile, checked)
	fmt.Printf("%s", report)
	return nil
}

func main() {
	if err := run(); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
