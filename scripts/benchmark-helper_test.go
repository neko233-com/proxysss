package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net"
	"net/http"
	"net/http/httptest"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"sync/atomic"
	"testing"
)

func TestSSEFixtureFinishesAndReusesHTTP1Connection(t *testing.T) {
	for _, chunks := range []int{1, 3} {
		t.Run(fmt.Sprint(chunks), func(t *testing.T) {
			var accepted atomic.Int32
			server := httptest.NewUnstartedServer(benchmarkSSEHandler(chunks))
			server.Config.ConnState = func(_ net.Conn, state http.ConnState) {
				if state == http.StateNew {
					accepted.Add(1)
				}
			}
			server.Start()
			defer server.Close()
			client := server.Client()
			for i := 0; i < 3; i++ {
				response, err := client.Get(server.URL + "/sse")
				if err != nil {
					t.Fatal(err)
				}
				body, err := io.ReadAll(response.Body)
				response.Body.Close()
				if err != nil {
					t.Fatal(err)
				}
				if response.Close || response.Header.Get("Content-Type") != "text/event-stream" || strings.Count(string(body), `"token-`) != chunks {
					t.Fatalf("incomplete/nonreusable fixture: close=%v body=%s", response.Close, body)
				}
				if strings.Contains(string(body), "[DONE]") != (chunks > 1) {
					t.Fatal("fixture end marker changed")
				}
			}
			if accepted.Load() != 1 {
				t.Fatalf("expected one reused connection, accepted=%d", accepted.Load())
			}
		})
	}
}

func TestClientResourcesRejectFalseMetadata(t *testing.T) {
	dir := t.TempDir()
	resource, output := filepath.Join(dir, "resources.txt"), filepath.Join(dir, "output.txt")
	for _, tc := range []struct {
		name, planned, actual, environment, stdout string
		valid                                      bool
	}{
		{"canonical-range", "12,13,23", "12-13,23", "2", "runtime workers : 2\n", true},
		{"sparse-high-ids", "2048,4096", "4096,2048", "2", "runtime workers : 2\n", true},
		{"container-instead-of-task", "13", "12-23", "2", "runtime workers : 2\n", false},
		{"environment-wrong", "13", "13", "12", "runtime workers : 2\n", false},
		{"runtime-wrong", "13", "13", "2", "runtime workers : 12\n", false},
		{"runtime-missing", "13", "13", "2", "success : 100\n", false},
		{"runtime-duplicate", "13", "13", "2", "runtime workers : 2\nruntime workers : 2\n", false},
		{"resource-duplicate", "13", "13\nallowed_cpus=13", "2", "runtime workers : 2\n", false},
		{"missing-cpus", "13", "", "2", "runtime workers : 2\n", false},
		{"reversed", "13-12", "13", "2", "runtime workers : 2\n", false},
		{"excessive", "0-999999999", "13", "2", "runtime workers : 2\n", false},
		{"duplicate-cpus", "12-13,13", "12-13", "2", "runtime workers : 2\n", false},
	} {
		t.Run(tc.name, func(t *testing.T) {
			if err := os.WriteFile(resource, []byte("allowed_cpus="+tc.actual+"\nruntime_workers="+tc.environment+"\n"), 0600); err != nil {
				t.Fatal(err)
			}
			if err := os.WriteFile(output, []byte(tc.stdout), 0600); err != nil {
				t.Fatal(err)
			}
			err := runVerifyClientResources([]string{"--expected-cpuset", tc.planned, "--expected-workers", "2", "--resources", resource, "--output", output})
			if (err == nil) != tc.valid {
				t.Fatalf("valid=%v: %v", tc.valid, err)
			}
		})
	}
}

func TestClientExecRecordsActualTasksetAffinity(t *testing.T) {
	if runtime.GOOS != "linux" {
		t.Skip("Linux /proc and taskset required")
	}
	status, err := os.ReadFile("/proc/self/status")
	if err != nil {
		t.Fatal(err)
	}
	var allowed string
	for _, line := range strings.Split(string(status), "\n") {
		if strings.HasPrefix(line, "Cpus_allowed_list:") {
			allowed = strings.TrimSpace(strings.TrimPrefix(line, "Cpus_allowed_list:"))
		}
	}
	canonical, err := canonicalClientCPUSet(allowed)
	if err != nil {
		t.Fatal(err)
	}
	oneCPU := strings.Split(canonical, ",")[0]
	_, thisFile, _, _ := runtime.Caller(0)
	wrapper, err := os.ReadFile(filepath.Join(filepath.Dir(thisFile), "benchmark-client-exec.sh"))
	if err != nil {
		t.Fatal(err)
	}
	dir := t.TempDir()
	script, resource, output := filepath.Join(dir, "exec.sh"), filepath.Join(dir, "resources.txt"), filepath.Join(dir, "stdout.txt")
	if err := os.WriteFile(script, bytes.ReplaceAll(wrapper, []byte("\r\n"), []byte("\n")), 0600); err != nil {
		t.Fatal(err)
	}
	cmd := exec.Command("taskset", "-c", oneCPU, "env", "TOKIO_WORKER_THREADS=2", "bash", script, resource, "sh", "-c", `printf 'runtime workers : %s\n' "$TOKIO_WORKER_THREADS"`)
	raw, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("wrapper: %v: %s", err, raw)
	}
	if err := os.WriteFile(output, raw, 0600); err != nil {
		t.Fatal(err)
	}
	if err := runVerifyClientResources([]string{"--expected-cpuset", oneCPU, "--expected-workers", "2", "--resources", resource, "--output", output}); err != nil {
		t.Fatal(err)
	}
}

func TestVerifyHTTPBodyRequiresPublishedBytesAndSuccess(t *testing.T) {
	var attempts atomic.Int32
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path == "/error" {
			w.WriteHeader(http.StatusNotFound)
			fmt.Fprint(w, "new\n")
			return
		}
		if r.URL.Path == "/long" {
			fmt.Fprint(w, "new\nextra")
			return
		}
		if attempts.Add(1) == 1 {
			fmt.Fprint(w, "old\n")
			return
		}
		fmt.Fprint(w, "new\n")
	}))
	defer server.Close()
	if err := runVerifyHTTPBody([]string{"--url", server.URL, "--body", "new\n", "--timeout", "1s"}); err != nil {
		t.Fatal(err)
	}
	for _, path := range []string{"/error", "/long"} {
		if err := runVerifyHTTPBody([]string{"--url", server.URL + path, "--body", "new\n", "--timeout", "50ms"}); err == nil {
			t.Fatalf("invalid fixture accepted: %s", path)
		}
	}
}

func TestCapacityRejectsMissingRetentionAndLostIdleConnections(t *testing.T) {
	old := "requested=100 opened=100 failed=0 attempts=100 open_rate=1000 worst_p50_ms=1 worst_p95_ms=2 worst_p99_ms=3"
	if _, err := parseIsolatedWSSCapacity(old); err == nil {
		t.Fatal("legacy opened-only capacity must not prove idle retention")
	}
	dir := t.TempDir()
	for _, gateway := range []string{"nginx", "proxysss"} {
		line := "requested=100 opened=100 failed=0 retained=100 hold_failed=0 hold_secs=60 attempts=100 open_rate=1000 worst_p50_ms=1 worst_p95_ms=2 worst_p99_ms=3"
		if gateway == "nginx" {
			line = strings.Replace(line, "retained=100 hold_failed=0", "retained=0 hold_failed=100", 1)
		}
		if err := os.WriteFile(filepath.Join(dir, gateway+"-capacity-total.txt"), []byte(line), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	summary, err := summarizeIsolatedWSSRuns([]string{dir}, IsolatedWSSGateOptions{RequireCapacity: true})
	if err != nil {
		t.Fatal(err)
	}
	if summary.Passed || !strings.Contains(strings.Join(summary.Failures, " "), "nginx capacity incomplete") {
		t.Fatalf("closed idle sockets must fail: %+v", summary)
	}
}

func TestHeldMemoryRequiresAllConnectionsAndTwoTimesEnvelope(t *testing.T) {
	dir := t.TempDir()
	write := func(name, text string) {
		t.Helper()
		if err := os.WriteFile(filepath.Join(dir, name), []byte(text), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	write("run-metadata.txt", "active_connections=100\n")
	write("nginx-active.txt", "success : 100\nerrors : 0\nops/sec : 1000\nlatency p50 : 2 ms\nlatency p95 : 3 ms\nlatency p99 : 4 ms\n")
	write("proxysss-active.txt", "success : 200\nerrors : 0\nops/sec : 2000\nlatency p50 : 1 ms\nlatency p95 : 2 ms\nlatency p99 : 3 ms\n")
	write("nginx-active-sample-gateway-memory.txt", "memory_current_bytes=1000\nmemory_peak_bytes=1500\nestablished_frontend_tcp=100\n")
	options := IsolatedWSSGateOptions{RequireActive: true, GateActiveOps: true, GateActiveLatency: true, RequireMemory: true}
	for _, sample := range []struct {
		current, peak int
		pass          bool
	}{{2000, 3000, true}, {2001, 3000, false}, {2000, 3001, false}} {
		write("proxysss-active-sample-gateway-memory.txt", fmt.Sprintf("memory_current_bytes=%d\nmemory_peak_bytes=%d\nestablished_frontend_tcp=100\n", sample.current, sample.peak))
		summary, err := summarizeIsolatedWSSRuns([]string{dir}, options)
		if err != nil {
			t.Fatal(err)
		}
		if summary.Passed != sample.pass {
			t.Fatalf("2x boundary rejected/accepted incorrectly: %+v", summary)
		}
	}
	write("proxysss-active-sample-gateway-memory.txt", "memory_current_bytes=500\nmemory_peak_bytes=1500\nestablished_frontend_tcp=0\n")
	if _, err := summarizeIsolatedWSSRuns([]string{dir}, options); err == nil {
		t.Fatal("post-disconnect memory cannot pass as held-connection memory")
	}
}

func writeBenchRows(t *testing.T, path string, rows []BenchRow) {
	t.Helper()
	payload, err := json.Marshal(rows)
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(path, payload, 0o644); err != nil {
		t.Fatal(err)
	}
}

func floatPointer(value float64) *float64 {
	return &value
}

func TestAggregateBenchRowMediansPreservesWorstError(t *testing.T) {
	rows := []BenchRow{
		{Scenario: "https", Gateway: "proxysss", OpsPerSec: 100, Errors: 0, LatencyP50MS: floatPointer(0.60)},
		{Scenario: "https", Gateway: "proxysss", OpsPerSec: 120, Errors: 1, LatencyP50MS: floatPointer(0.50)},
		{Scenario: "https", Gateway: "proxysss", OpsPerSec: 110, Errors: 0, LatencyP50MS: floatPointer(0.55)},
		{Scenario: "https", Gateway: "nginx", OpsPerSec: 90, Errors: 0, LatencyP50MS: floatPointer(0.70)},
		{Scenario: "https", Gateway: "nginx", OpsPerSec: 100, Errors: 0, LatencyP50MS: floatPointer(0.65)},
		{Scenario: "https", Gateway: "nginx", OpsPerSec: 95, Errors: 0, LatencyP50MS: floatPointer(0.68)},
	}
	aggregated, err := aggregateBenchRowMedians(rows)
	if err != nil {
		t.Fatal(err)
	}
	if len(aggregated) != 2 {
		t.Fatalf("unexpected rows: %+v", aggregated)
	}
	var proxy BenchRow
	for _, row := range aggregated {
		if row.Gateway == "proxysss" {
			proxy = row
		}
	}
	if proxy.OpsPerSec != 110 || proxy.LatencyP50MS == nil || *proxy.LatencyP50MS != 0.55 {
		t.Fatalf("unexpected proxy medians: %+v", proxy)
	}
	if proxy.Errors != 1 {
		t.Fatalf("worst repetition error must survive aggregation: %+v", proxy)
	}
}

func TestAggregateBenchRowMediansRejectsPartialPercentiles(t *testing.T) {
	rows := []BenchRow{
		{Scenario: "tcp", Gateway: "proxysss", OpsPerSec: 10, LatencyP99MS: floatPointer(1)},
		{Scenario: "tcp", Gateway: "proxysss", OpsPerSec: 11},
	}
	aggregated, err := aggregateBenchRowMedians(rows)
	if err != nil {
		t.Fatal(err)
	}
	if aggregated[0].LatencyP99MS != nil {
		t.Fatalf("partial percentile data must not look complete: %+v", aggregated[0])
	}
}

func TestParseIsolatedWSSCapacity(t *testing.T) {
	capacity, err := parseIsolatedWSSCapacity("requested=100000 opened=100000 failed=0 retained=100000 hold_failed=0 hold_secs=60 attempts=100001 open_rate=8123.5 worst_p50_ms=2.1 worst_p95_ms=4.2 worst_p99_ms=7.3\n")
	if err != nil {
		t.Fatal(err)
	}
	if capacity.Opened != 100000 || capacity.Failed != 0 || capacity.P99MS != 7.3 {
		t.Fatalf("unexpected capacity: %+v", capacity)
	}
}

func TestParseIsolatedWSSCapacityClient(t *testing.T) {
	raw := "protocol : websocket-connections\nconnections requested : 20000\nconnections opened : 20000\nconnections failed : 0\nconnections retained : 20000\nhold failures : 0\nhold duration : 60 s\nhandshake attempts : 20001\nopen rate : 4321.50 connections/s\nhandshake p50 : 12.3 ms\nhandshake p95 : 45.6 ms\nhandshake p99 : 78.9 ms\n"
	sample, err := parseIsolatedWSSCapacityClient(raw)
	if err != nil {
		t.Fatal(err)
	}
	if sample.Requested != 20000 || sample.OpenRate != 4321.5 || sample.P99MS != 78.9 {
		t.Fatalf("unexpected sample: %+v", sample)
	}
}

func TestIsolatedWSSMedianGateRejectsEqualTailLatency(t *testing.T) {
	runDir := t.TempDir()
	active := "success : 100\nerrors : 0\nops/sec : 1000\nlatency p50 : 1 ms\nlatency p95 : 2 ms\nlatency p99 : 3 ms\n"
	capacity := "requested=100 opened=100 failed=0 retained=100 hold_failed=0 hold_secs=60 attempts=100 open_rate=1000 worst_p50_ms=1 worst_p95_ms=2 worst_p99_ms=3\n"
	for _, gateway := range []string{"nginx", "proxysss"} {
		if err := os.WriteFile(filepath.Join(runDir, gateway+"-active.txt"), []byte(active), 0o644); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(filepath.Join(runDir, gateway+"-capacity-total.txt"), []byte(capacity), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	summary, err := summarizeIsolatedWSSRuns([]string{runDir}, IsolatedWSSGateOptions{
		RequireActive:     true,
		RequireCapacity:   true,
		GateActiveOps:     true,
		GateActiveLatency: true,
	})
	if err != nil {
		t.Fatal(err)
	}
	if summary.Passed {
		t.Fatal("strict median gate must reject equality")
	}
	joined := strings.Join(summary.Failures, "\n")
	if !strings.Contains(joined, "active p99") || !strings.Contains(joined, "handshake p99") {
		t.Fatalf("expected active and handshake equality failures, got %v", summary.Failures)
	}
}

func TestEqualLoadGateAllowsEqualOpsAndRequiresLatencyWin(t *testing.T) {
	runDir := t.TempDir()
	nginx := "target ops/sec : 40500\nsuccess : 100\nerrors : 0\nops/sec : 40000\nlatency p50 : 2 ms\nlatency p95 : 4 ms\nlatency p99 : 8 ms\n"
	proxy := "target ops/sec : 40500\nsuccess : 100\nerrors : 0\nops/sec : 40000\nlatency p50 : 1 ms\nlatency p95 : 3 ms\nlatency p99 : 7 ms\n"
	if err := os.WriteFile(filepath.Join(runDir, "nginx-active.txt"), []byte(nginx), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(runDir, "proxysss-active.txt"), []byte(proxy), 0o644); err != nil {
		t.Fatal(err)
	}
	summary, err := summarizeIsolatedWSSRuns([]string{runDir}, IsolatedWSSGateOptions{
		RequireActive:      true,
		GateActiveOps:      false,
		GateActiveLatency:  true,
		MinActiveOpsPerSec: 39000,
	})
	if err != nil {
		t.Fatal(err)
	}
	if !summary.Passed {
		t.Fatalf("equal-load latency gate should pass: %v", summary.Failures)
	}
	if summary.Nginx.ActiveOpsPerSec != 40000 || summary.Proxysss.ActiveOpsPerSec != 40000 {
		t.Fatalf("parser used target rather than actual ops: %+v", summary)
	}
}

func TestEqualLoadGateRequiresEveryRunToMeetOfferedLoad(t *testing.T) {
	var runDirs []string
	for _, nginxOps := range []string{"38000", "41000"} {
		runDir := t.TempDir()
		runDirs = append(runDirs, runDir)
		nginx := "success : 100\nerrors : 0\nops/sec : " + nginxOps + "\nlatency p50 : 2 ms\nlatency p95 : 4 ms\nlatency p99 : 8 ms\n"
		proxy := "success : 100\nerrors : 0\nops/sec : 41000\nlatency p50 : 1 ms\nlatency p95 : 3 ms\nlatency p99 : 7 ms\n"
		if err := os.WriteFile(filepath.Join(runDir, "nginx-active.txt"), []byte(nginx), 0o644); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(filepath.Join(runDir, "proxysss-active.txt"), []byte(proxy), 0o644); err != nil {
			t.Fatal(err)
		}
	}

	summary, err := summarizeIsolatedWSSRuns(runDirs, IsolatedWSSGateOptions{
		RequireActive:      true,
		GateActiveLatency:  true,
		MinActiveOpsPerSec: 39000,
	})
	if err != nil {
		t.Fatal(err)
	}
	if summary.Passed {
		t.Fatal("equal-load gate must reject an under-target run even when its median passes")
	}
	if !strings.Contains(strings.Join(summary.Failures, "\n"), "nginx active ops/sec 38000.000 below 39000.000") {
		t.Fatalf("expected per-run offered-load failure, got %v", summary.Failures)
	}
}

func TestEqualLoadGateAutomaticallyChecksExecutableTarget(t *testing.T) {
	for _, test := range []struct {
		name, nginxTarget, proxyTarget string
		ops                            float64
		pass                           bool
	}{
		{"boundary", "100", "100", 98, true},
		{"below", "100", "100", 97.99, false},
		{"mismatch", "100", "99", 100, false},
		{"missing", "", "100", 100, false},
		{"zero ticks", "0", "0", 100, false},
	} {
		t.Run(test.name, func(t *testing.T) {
			dir := t.TempDir()
			for gateway, target := range map[string]string{"nginx": test.nginxTarget, "proxysss": test.proxyTarget} {
				latency := 2
				if gateway == "proxysss" {
					latency = 1
				}
				raw := fmt.Sprintf("target ops/sec : %s\nsuccess : 100\nerrors : 0\nops/sec : %.2f\nlatency p50 : %d ms\nlatency p95 : %d ms\nlatency p99 : %d ms\n", target, test.ops, latency, latency, latency)
				if err := os.WriteFile(filepath.Join(dir, gateway+"-active.txt"), []byte(raw), 0o644); err != nil {
					t.Fatal(err)
				}
			}
			summary, err := summarizeIsolatedWSSRuns([]string{dir}, IsolatedWSSGateOptions{RequireActive: true, GateActiveLatency: true})
			if err != nil {
				t.Fatal(err)
			}
			if summary.Passed != test.pass {
				t.Fatalf("unexpected gate: %+v", summary)
			}
		})
	}
}

func TestStrictLatencyGateRequiresEveryPercentileToBeatNginx(t *testing.T) {
	proxy := BenchRow{
		LatencyP50MS: floatPointer(0.80),
		LatencyP95MS: floatPointer(1.20),
		LatencyP99MS: floatPointer(2.00),
	}
	nginx := BenchRow{
		LatencyP50MS: floatPointer(1.00),
		LatencyP95MS: floatPointer(1.20),
		LatencyP99MS: floatPointer(2.50),
	}

	failures := latencyGateFailures("websocket", proxy, nginx, 1.0, true, true)
	if len(failures) != 1 || !strings.Contains(failures[0], "p95") {
		t.Fatalf("expected equal p95 to fail strict gate, got %v", failures)
	}
}

func TestLatencyGateRejectsMissingPercentilesWhenRequired(t *testing.T) {
	proxy := BenchRow{LatencyP50MS: floatPointer(0.8)}
	nginx := BenchRow{LatencyP50MS: floatPointer(1.0)}

	failures := latencyGateFailures("tcp", proxy, nginx, 1.0, true, true)
	if len(failures) != 2 {
		t.Fatalf("expected missing p95 and p99 failures, got %v", failures)
	}
}

func TestEqualLoadPlanUsesSlowerGatewayAndConcurrency(t *testing.T) {
	dir := t.TempDir()
	results := filepath.Join(dir, "saturation.json")
	plan := filepath.Join(dir, "plan.txt")
	writeBenchRows(t, results, []BenchRow{
		{Scenario: "websocket", Gateway: "nginx", Protocol: "websocket", Concurrency: 8, OpsPerSec: 800},
		{Scenario: "websocket", Gateway: "proxysss", Protocol: "websocket", Concurrency: 8, OpsPerSec: 1200},
	})
	if err := runWriteEqualLoadPlan([]string{
		"--results", results,
		"--out", plan,
		"--fraction", "0.70",
		"--duration-secs", "1",
	}); err != nil {
		t.Fatal(err)
	}
	raw, err := os.ReadFile(plan)
	if err != nil {
		t.Fatal(err)
	}
	if !strings.HasPrefix(string(raw), "websocket|14286|552.000000") {
		t.Fatalf("unexpected equal-load plan: %s", raw)
	}
}

func TestEqualLoadPlanReservesRealtimeCompletionGuard(t *testing.T) {
	dir := t.TempDir()
	results := filepath.Join(dir, "saturation.json")
	plan := filepath.Join(dir, "plan.txt")
	writeBenchRows(t, results, []BenchRow{
		{Scenario: "qcp-transparent", Gateway: "nginx", Protocol: "udp", Concurrency: 64, OpsPerSec: 8964},
		{Scenario: "qcp-transparent", Gateway: "proxysss", Protocol: "udp", Concurrency: 64, OpsPerSec: 12000},
	})
	if err := runWriteEqualLoadPlan([]string{
		"--results", results,
		"--out", plan,
		"--fraction", "0.25",
		"--duration-secs", "1",
	}); err != nil {
		t.Fatal(err)
	}
	raw, err := os.ReadFile(plan)
	if err != nil {
		t.Fatal(err)
	}
	if !strings.HasSuffix(string(raw), "|2176.000000\n") {
		t.Fatalf("realtime target must leave one millisecond for final completion: %s", raw)
	}
}

func TestEqualLoadPlanReservesWebSocketCompletionGuard(t *testing.T) {
	dir := t.TempDir()
	results := filepath.Join(dir, "saturation.json")
	plan := filepath.Join(dir, "plan.txt")
	writeBenchRows(t, results, []BenchRow{
		{Scenario: "websocket-long-connection", Gateway: "nginx", Protocol: "websocket", Concurrency: 128, OpsPerSec: 8211},
		{Scenario: "websocket-long-connection", Gateway: "proxysss", Protocol: "websocket", Concurrency: 128, OpsPerSec: 10381},
	})
	if err := runWriteEqualLoadPlan([]string{
		"--results", results,
		"--out", plan,
		"--fraction", "0.25",
		"--duration-secs", "1",
	}); err != nil {
		t.Fatal(err)
	}
	raw, err := os.ReadFile(plan)
	if err != nil {
		t.Fatal(err)
	}
	if !strings.HasSuffix(string(raw), "|1920.000000\n") {
		t.Fatalf("websocket target must leave five milliseconds for final framing and completion: %s", raw)
	}
}

func TestEqualLoadPlanReservesTCPCompletionGuard(t *testing.T) {
	dir := t.TempDir()
	results := filepath.Join(dir, "saturation.json")
	plan := filepath.Join(dir, "plan.txt")
	writeBenchRows(t, results, []BenchRow{
		{Scenario: "tcp-stream", Gateway: "nginx", Protocol: "tcp", Concurrency: 64, OpsPerSec: 9226},
		{Scenario: "tcp-stream", Gateway: "proxysss", Protocol: "tcp", Concurrency: 64, OpsPerSec: 14468},
	})
	if err := runWriteEqualLoadPlan([]string{
		"--results", results,
		"--out", plan,
		"--fraction", "0.25",
		"--duration-secs", "1",
	}); err != nil {
		t.Fatal(err)
	}
	raw, err := os.ReadFile(plan)
	if err != nil {
		t.Fatal(err)
	}
	if !strings.HasSuffix(string(raw), "|2240.000000\n") {
		t.Fatalf("tcp target must leave two milliseconds for final relay completion: %s", raw)
	}
}

func TestEqualLoadPlanQuantizesShortWindowTarget(t *testing.T) {
	dir := t.TempDir()
	results := filepath.Join(dir, "saturation.json")
	plan := filepath.Join(dir, "plan.txt")
	writeBenchRows(t, results, []BenchRow{
		{Scenario: "static-large", Gateway: "nginx", Protocol: "http", Concurrency: 4, OpsPerSec: 83.5},
		{Scenario: "static-large", Gateway: "proxysss", Protocol: "http", Concurrency: 4, OpsPerSec: 90},
	})
	if err := runWriteEqualLoadPlan([]string{
		"--results", results,
		"--out", plan,
		"--fraction", "0.25",
		"--duration-secs", "1",
	}); err != nil {
		t.Fatal(err)
	}
	raw, err := os.ReadFile(plan)
	if err != nil {
		t.Fatal(err)
	}
	if !strings.HasSuffix(string(raw), "|21.000000\n") {
		t.Fatalf("one-second target must match the 21 phase-spread operations: %s", raw)
	}
}

func TestEqualLoadPlanAllowsTargetBelowConnectionCount(t *testing.T) {
	dir := t.TempDir()
	results := filepath.Join(dir, "saturation.json")
	plan := filepath.Join(dir, "plan.txt")
	writeBenchRows(t, results, []BenchRow{
		{Scenario: "static-large", Gateway: "nginx", Protocol: "http", Concurrency: 32, OpsPerSec: 64},
		{Scenario: "static-large", Gateway: "proxysss", Protocol: "http", Concurrency: 32, OpsPerSec: 96},
	})
	if err := runWriteEqualLoadPlan([]string{
		"--results", results,
		"--out", plan,
		"--fraction", "0.25",
		"--duration-secs", "1",
	}); err != nil {
		t.Fatal(err)
	}
	raw, err := os.ReadFile(plan)
	if err != nil {
		t.Fatal(err)
	}
	if !strings.HasSuffix(string(raw), "|16.000000\n") {
		t.Fatalf("low-rate target must not force all 32 connections to run: %s", raw)
	}
}

func TestEqualLoadPlanExcludesCenteredDeadlineSlot(t *testing.T) {
	for _, protocol := range []string{"http", "sse"} {
		dir := t.TempDir()
		results, plan := filepath.Join(dir, "rows.json"), filepath.Join(dir, "plan.txt")
		writeBenchRows(t, results, []BenchRow{
			{Scenario: "static-large", Gateway: "nginx", Protocol: protocol, Concurrency: 48, OpsPerSec: 50},
			{Scenario: "static-large", Gateway: "proxysss", Protocol: protocol, Concurrency: 48, OpsPerSec: 50},
		})
		if err := runWriteEqualLoadPlan([]string{"--results", results, "--out", plan, "--fraction", "0.25", "--duration-secs", "1"}); err != nil {
			t.Fatal(err)
		}
		raw, err := os.ReadFile(plan)
		if err != nil {
			t.Fatal(err)
		}
		// At 12.5 ops/s, centered slots are 0.04, 0.12, ..., 0.92, 1.00.
		// Only the first twelve start strictly before the one-second deadline.
		if string(raw) != "static-large|3840000|12.000000\n" {
			t.Fatalf("impossible boundary request counted: %s", raw)
		}
		if err := runWriteEqualLoadPlan([]string{"--results", results, "--out", plan, "--fraction", "0.001", "--duration-secs", "1"}); err == nil {
			t.Fatal("an empty schedule must not invent one mandatory request")
		}
	}
}

func TestAllScenariosEqualLoadGateRequiresTargetCompletion(t *testing.T) {
	dir := t.TempDir()
	results := filepath.Join(dir, "equal-load.json")
	md := filepath.Join(dir, "summary.md")
	html := filepath.Join(dir, "summary.html")
	target := 100.0
	nginxP50, nginxP95, nginxP99 := 2.0, 4.0, 8.0
	proxyP50, proxyP95, proxyP99 := 1.0, 3.0, 7.0
	rows := []BenchRow{
		{Scenario: "tcp", Gateway: "nginx", Protocol: "tcp", DurationSecs: 1, Concurrency: 1, Success: 99, OpsPerSec: 99, TargetOpsPerSec: &target, LatencyP50MS: &nginxP50, LatencyP95MS: &nginxP95, LatencyP99MS: &nginxP99},
		{Scenario: "tcp", Gateway: "proxysss", Protocol: "tcp", DurationSecs: 1, Concurrency: 1, Success: 99, OpsPerSec: 99, TargetOpsPerSec: &target, LatencyP50MS: &proxyP50, LatencyP95MS: &proxyP95, LatencyP99MS: &proxyP99},
	}
	writeBenchRows(t, results, rows)
	args := []string{
		"--results", results,
		"--md", md,
		"--html", html,
		"--gate-ops=false",
		"--gate-latency=true",
		"--min-target-achievement=0.98",
		"--require-latency-percentiles=true",
		"--require-zero-errors=true",
		"--strict-superiority=true",
	}
	if err := runWriteAllScenariosSummary(args); err != nil {
		t.Fatalf("99%% fixed-load completion should pass: %v", err)
	}
	rows[0].OpsPerSec = 97
	rows[0].Success = 97
	writeBenchRows(t, results, rows)
	if err := runWriteAllScenariosSummary(args); err == nil || !strings.Contains(err.Error(), "nginx target achievement") {
		t.Fatalf("reference under-completion invalidates the equal-load comparison: %v", err)
	}
	rows[0].OpsPerSec = 99
	rows[1].OpsPerSec = 97
	rows[0].Success = 99
	rows[1].Success = 97
	writeBenchRows(t, results, rows)
	if err := runWriteAllScenariosSummary(args); err == nil || !strings.Contains(err.Error(), "target achievement") {
		t.Fatalf("97%% fixed-load completion should fail, got %v", err)
	}
}

func TestStrictOpsGateRequiresRatioAboveFloor(t *testing.T) {
	if !ratioGateFailed(1.0, 1.0, true) {
		t.Fatal("strict gate must reject equality")
	}
	if ratioGateFailed(1.001, 1.0, true) {
		t.Fatal("strict gate must accept a ratio above the floor")
	}
	if ratioGateFailed(1.0, 1.0, false) {
		t.Fatal("non-strict gate keeps legacy equality behavior")
	}
}

func equalLoadRepeatedRows() []BenchRow {
	var rows []BenchRow
	// Reverse candidate order in the second wave, as the real matrix does.
	for _, gateway := range []string{"nginx", "proxysss", "proxysss", "nginx"} {
		rows = append(rows, BenchRow{Scenario: "qcp-transparent", Gateway: gateway,
			Protocol: "udp", Concurrency: 24, DurationSecs: 1, Success: 1056,
			OpsPerSec: 1056, TargetOpsPerSec: floatPointer(1056),
			LatencyP50MS: floatPointer(1), LatencyP95MS: floatPointer(2), LatencyP99MS: floatPointer(3)})
	}
	return rows
}

func TestRepeatedCompletionSurvivesSerializationAndMedianAggregation(t *testing.T) {
	for _, failedIndex := range []int{-1, 0, 1, 2, 3} {
		t.Run(fmt.Sprint(failedIndex), func(t *testing.T) {
			dir := t.TempDir()
			rows := equalLoadRepeatedRows()
			if failedIndex >= 0 {
				// The real failing sample: 1032/1056=.97727; averaging with
				// 1056/1056 used to produce .98864 and falsely pass the gate.
				rows[failedIndex].Success, rows[failedIndex].OpsPerSec = 1032, 1032
			}
			raw := filepath.Join(dir, "raw.jsonl")
			var input strings.Builder
			for _, row := range rows {
				data, err := json.Marshal(row)
				if err != nil {
					t.Fatal(err)
				}
				input.Write(data)
				input.WriteByte('\n')
			}
			if err := os.WriteFile(raw, []byte(input.String()), 0o644); err != nil {
				t.Fatal(err)
			}
			results := filepath.Join(dir, "results.json")
			if err := runAggregateBenchMedians([]string{"--in", raw, "--out", results}); err != nil {
				t.Fatal(err)
			}
			aggregated, err := loadResults(results)
			if err != nil {
				t.Fatal(err)
			}
			for _, row := range aggregated {
				if row.SampleCount != 2 || row.WorstTargetAchievement == nil {
					t.Fatalf("missing per-sample evidence: %+v", row)
				}
				if failedIndex >= 0 && row.Gateway == rows[failedIndex].Gateway {
					if row.OpsPerSec != 1044 || row.LatencyP99MS == nil || *row.LatencyP99MS != 3 || *row.WorstTargetAchievement != 1032.0/1056 || row.WorstTargetSample != 1+failedIndex/2 {
						t.Fatalf("median or worst-sample evidence changed: %+v", row)
					}
				}
			}
			md, html := filepath.Join(dir, "report.md"), filepath.Join(dir, "report.html")
			err = runWriteAllScenariosSummary([]string{"--results", results, "--md", md, "--html", html,
				"--gate-ops=false", "--gate-latency=false", "--min-target-achievement=.98", "--samples-per-gateway=2"})
			if failedIndex < 0 {
				if err != nil {
					t.Fatal(err)
				}
				return
			}
			if err == nil || !strings.Contains(err.Error(), rows[failedIndex].Gateway+" target achievement") || !strings.Contains(err.Error(), fmt.Sprintf("worst sample=%d/2", 1+failedIndex/2)) {
				t.Fatalf("single failed repetition must fail: %v", err)
			}
			for _, path := range []string{md, html} {
				data, err := os.ReadFile(path)
				if err != nil {
					t.Fatal(err)
				}
				if !strings.Contains(string(data), "0.977x") {
					t.Fatalf("%s concealed worst completion: %s", path, data)
				}
			}
		})
	}
}

func TestFixedLoadEvidenceRejectsMissingSamplesAndUnequalTargets(t *testing.T) {
	for _, name := range []string{"missing repetition", "unequal gateways", "missing worst completion", "invalid worst sample", "empty report"} {
		t.Run(name, func(t *testing.T) {
			rows := equalLoadRepeatedRows()
			if name == "missing repetition" {
				rows = rows[:3]
			}
			if name == "unequal gateways" {
				rows[0].TargetOpsPerSec = floatPointer(1057)
				rows[3].TargetOpsPerSec = floatPointer(1057)
			}
			aggregated, err := aggregateBenchRowMedians(rows)
			if err != nil {
				t.Fatal(err)
			}
			if name == "missing worst completion" {
				aggregated[0].WorstTargetAchievement = nil
			}
			if name == "invalid worst sample" {
				aggregated[0].WorstTargetSample = 3
			}
			if name == "empty report" {
				aggregated = nil
			}
			dir := t.TempDir()
			results := filepath.Join(dir, "results.json")
			writeBenchRows(t, results, aggregated)
			if err := runWriteAllScenariosSummary([]string{"--results", results, "--md", filepath.Join(dir, "report.md"), "--html", filepath.Join(dir, "report.html"), "--gate-ops=false", "--gate-latency=false", "--min-target-achievement=.98", "--samples-per-gateway=2"}); err == nil {
				t.Fatal("invalid evidence passed")
			}
		})
	}
}

func TestAggregationRejectsIncompatibleOrReaggregatedSamples(t *testing.T) {
	for _, field := range []string{"target", "partial target", "protocol", "concurrency", "duration", "ops", "sample count"} {
		t.Run(field, func(t *testing.T) {
			rows := equalLoadRepeatedRows()
			switch field {
			case "target":
				rows[3].TargetOpsPerSec = floatPointer(1057)
			case "partial target":
				rows[3].TargetOpsPerSec = nil
			case "protocol":
				rows[3].Protocol = "tcp"
			case "concurrency":
				rows[3].Concurrency = 25
			case "duration":
				rows[3].DurationSecs = 2
			case "ops":
				rows[3].OpsPerSec = 1057
			case "sample count":
				rows[3].SampleCount = 2
			}
			if _, err := aggregateBenchRowMedians(rows); err == nil {
				t.Fatal("inconsistent raw samples accepted")
			}
		})
	}
}

func TestBenchmarkReportFormatsPositiveImprovementForFasterProxy(t *testing.T) {
	if got := opsImprovementPercent(1.125); got != "+12.50%" {
		t.Fatalf("unexpected ops improvement: %s", got)
	}
	proxy, nginx := 0.75, 1.0
	if got := latencyImprovementPercent(&proxy, &nginx); got != "+25.00%" {
		t.Fatalf("unexpected latency improvement: %s", got)
	}
}
