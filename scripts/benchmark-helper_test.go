package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

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
	capacity, err := parseIsolatedWSSCapacity("requested=100000 opened=100000 failed=0 attempts=100001 open_rate=8123.5 worst_p50_ms=2.1 worst_p95_ms=4.2 worst_p99_ms=7.3\n")
	if err != nil {
		t.Fatal(err)
	}
	if capacity.Opened != 100000 || capacity.Failed != 0 || capacity.P99MS != 7.3 {
		t.Fatalf("unexpected capacity: %+v", capacity)
	}
}

func TestParseIsolatedWSSCapacityClient(t *testing.T) {
	raw := "protocol : websocket-connections\nconnections requested : 20000\nconnections opened : 20000\nconnections failed : 0\nhandshake attempts : 20001\nopen rate : 4321.50 connections/s\nhandshake p50 : 12.3 ms\nhandshake p95 : 45.6 ms\nhandshake p99 : 78.9 ms\n"
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
	capacity := "requested=100 opened=100 failed=0 attempts=100 open_rate=1000 worst_p50_ms=1 worst_p95_ms=2 worst_p99_ms=3\n"
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
	nginx := "target ops/sec : 99999\nsuccess : 100\nerrors : 0\nops/sec : 40000\nlatency p50 : 2 ms\nlatency p95 : 4 ms\nlatency p99 : 8 ms\n"
	proxy := "target ops/sec : 99999\nsuccess : 100\nerrors : 0\nops/sec : 40000\nlatency p50 : 1 ms\nlatency p95 : 3 ms\nlatency p99 : 7 ms\n"
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
		{Scenario: "websocket", Gateway: "nginx", Concurrency: 8, OpsPerSec: 800},
		{Scenario: "websocket", Gateway: "proxysss", Concurrency: 8, OpsPerSec: 1200},
	})
	if err := runWriteEqualLoadPlan([]string{
		"--results", results,
		"--out", plan,
		"--fraction", "0.70",
	}); err != nil {
		t.Fatal(err)
	}
	raw, err := os.ReadFile(plan)
	if err != nil {
		t.Fatal(err)
	}
	if !strings.HasPrefix(string(raw), "websocket|14286|559.98") {
		t.Fatalf("unexpected equal-load plan: %s", raw)
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
		{Scenario: "tcp", Gateway: "nginx", Protocol: "tcp", OpsPerSec: 99, TargetOpsPerSec: &target, LatencyP50MS: &nginxP50, LatencyP95MS: &nginxP95, LatencyP99MS: &nginxP99},
		{Scenario: "tcp", Gateway: "proxysss", Protocol: "tcp", OpsPerSec: 99, TargetOpsPerSec: &target, LatencyP50MS: &proxyP50, LatencyP95MS: &proxyP95, LatencyP99MS: &proxyP99},
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
	writeBenchRows(t, results, rows)
	if err := runWriteAllScenariosSummary(args); err != nil {
		t.Fatalf("reference under-completion should remain a warning when candidate meets target: %v", err)
	}
	rows[0].OpsPerSec = 99
	rows[1].OpsPerSec = 97
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

func TestBenchmarkReportFormatsPositiveImprovementForFasterProxy(t *testing.T) {
	if got := opsImprovementPercent(1.125); got != "+12.50%" {
		t.Fatalf("unexpected ops improvement: %s", got)
	}
	proxy, nginx := 0.75, 1.0
	if got := latencyImprovementPercent(&proxy, &nginx); got != "+25.00%" {
		t.Fatalf("unexpected latency improvement: %s", got)
	}
}
