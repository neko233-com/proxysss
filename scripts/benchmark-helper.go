package main

import (
	"bufio"
	"bytes"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"html"
	"io"
	"math"
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

type BenchRow struct {
	Scenario        string   `json:"scenario,omitempty"`
	Gateway         string   `json:"gateway,omitempty"`
	Name            string   `json:"name"`
	Protocol        string   `json:"protocol,omitempty"`
	Target          string   `json:"target,omitempty"`
	URL             string   `json:"url,omitempty"`
	Concurrency     int      `json:"concurrency"`
	DurationSecs    int      `json:"duration_secs"`
	Success         int      `json:"success"`
	Errors          int      `json:"errors"`
	OpsPerSec       float64  `json:"ops_per_sec"`
	ThroughputMiBS  float64  `json:"throughput_mib_s"`
	TargetOpsPerSec *float64 `json:"target_ops_per_sec,omitempty"`
	LatencyP50MS    *float64 `json:"latency_p50_ms"`
	LatencyP95MS    *float64 `json:"latency_p95_ms"`
	LatencyP99MS    *float64 `json:"latency_p99_ms"`
}

type SimpleBaseline struct {
	MaxErrorCount              int     `json:"max_error_count"`
	MinProxysssVsNginxOpsRatio float64 `json:"min_proxysss_vs_nginx_ops_ratio"`
}

type latencyMetric struct {
	Name  string
	Proxy *float64
	Nginx *float64
}

var (
	successPattern    = regexp.MustCompile(`(?m)^success\s+:\s+(\d+)`)
	errorPattern      = regexp.MustCompile(`(?m)^errors\s+:\s+(\d+)`)
	opsPattern        = regexp.MustCompile(`(?m)^ops/sec\s+:\s+([\d.]+)`)
	throughputPattern = regexp.MustCompile(`(?m)^throughput\s+:\s+([\d.]+)\s+MiB/s`)
	targetOpsPattern  = regexp.MustCompile(`(?m)^target ops/sec\s+:\s+([\d.]+)`)
	p50Pattern        = regexp.MustCompile(`(?m)^latency p50\s+:\s+([\d.]+)\s+ms`)
	p95Pattern        = regexp.MustCompile(`(?m)^latency p95\s+:\s+([\d.]+)\s+ms`)
	p99Pattern        = regexp.MustCompile(`(?m)^latency p99\s+:\s+([\d.]+)\s+ms`)
)

func main() {
	if len(os.Args) < 2 {
		usage()
		os.Exit(2)
	}
	var err error
	switch os.Args[1] {
	case "write-large-file":
		err = runWriteLargeFile(os.Args[2:])
	case "serve-sse":
		err = runServeSSE(os.Args[2:])
	case "parse-bench":
		err = runParseBench(os.Args[2:])
	case "write-json-array":
		err = runWriteJSONArray(os.Args[2:])
	case "aggregate-bench-medians":
		err = runAggregateBenchMedians(os.Args[2:])
	case "write-equal-load-plan":
		err = runWriteEqualLoadPlan(os.Args[2:])
	case "quick-gate":
		err = runQuickGate(os.Args[2:])
	case "write-all-scenarios-summary":
		err = runWriteAllScenariosSummary(os.Args[2:])
	case "print-results-summary":
		err = runPrintResultsSummary(os.Args[2:])
	case "write-gateway-report":
		err = runWriteGatewayReport(os.Args[2:])
	case "write-gateway-compare":
		err = runWriteGatewayCompare(os.Args[2:])
	case "check-simple-gate":
		err = runCheckSimpleGate(os.Args[2:])
	case "summarize-isolated-wss":
		err = runSummarizeIsolatedWSS(os.Args[2:])
	case "aggregate-isolated-wss-capacity":
		err = runAggregateIsolatedWSSCapacity(os.Args[2:])
	default:
		usage()
		err = fmt.Errorf("unknown subcommand %q", os.Args[1])
	}
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

var (
	connectionRequestedPattern = regexp.MustCompile(`(?m)^connections requested\s+:\s+(\d+)`)
	connectionOpenedPattern    = regexp.MustCompile(`(?m)^connections opened\s+:\s+(\d+)`)
	connectionFailedPattern    = regexp.MustCompile(`(?m)^connections failed\s+:\s+(\d+)`)
	handshakeAttemptsPattern   = regexp.MustCompile(`(?m)^handshake attempts\s+:\s+(\d+)`)
	openRatePattern            = regexp.MustCompile(`(?m)^open rate\s+:\s+([\d.]+)\s+connections/s`)
	handshakeP50Pattern        = regexp.MustCompile(`(?m)^handshake p50\s+:\s+([\d.]+)\s+ms`)
	handshakeP95Pattern        = regexp.MustCompile(`(?m)^handshake p95\s+:\s+([\d.]+)\s+ms`)
	handshakeP99Pattern        = regexp.MustCompile(`(?m)^handshake p99\s+:\s+([\d.]+)\s+ms`)
)

func runAggregateIsolatedWSSCapacity(args []string) error {
	fs := flag.NewFlagSet("aggregate-isolated-wss-capacity", flag.ContinueOnError)
	var inputs repeatedStringFlag
	fs.Var(&inputs, "input", "capacity client output (repeatable)")
	expected := fs.Int("expected", 0, "expected aggregate connection count")
	fs.SetOutput(io.Discard)
	if err := fs.Parse(args); err != nil {
		return err
	}
	if len(inputs) == 0 || *expected <= 0 {
		return errors.New("aggregate-isolated-wss-capacity: --input and positive --expected are required")
	}
	var total IsolatedWSSCapacity
	for _, input := range inputs {
		raw, err := os.ReadFile(input)
		if err != nil {
			return err
		}
		sample, err := parseIsolatedWSSCapacityClient(string(raw))
		if err != nil {
			return fmt.Errorf("parse %s: %w", input, err)
		}
		total.Requested += sample.Requested
		total.Opened += sample.Opened
		total.Failed += sample.Failed
		total.Attempts += sample.Attempts
		total.OpenRate += sample.OpenRate
		if sample.P50MS > total.P50MS {
			total.P50MS = sample.P50MS
		}
		if sample.P95MS > total.P95MS {
			total.P95MS = sample.P95MS
		}
		if sample.P99MS > total.P99MS {
			total.P99MS = sample.P99MS
		}
	}
	fmt.Printf("requested=%d opened=%d failed=%d attempts=%d open_rate=%.3f worst_p50_ms=%.3f worst_p95_ms=%.3f worst_p99_ms=%.3f\n",
		total.Requested, total.Opened, total.Failed, total.Attempts, total.OpenRate,
		total.P50MS, total.P95MS, total.P99MS)
	if total.Requested != *expected || total.Opened != *expected || total.Failed != 0 {
		return fmt.Errorf("capacity incomplete: expected=%d requested=%d opened=%d failed=%d", *expected, total.Requested, total.Opened, total.Failed)
	}
	return nil
}

func parseIsolatedWSSCapacityClient(raw string) (IsolatedWSSCapacity, error) {
	valueInt := func(name string, pattern *regexp.Regexp) (int, error) {
		match := pattern.FindStringSubmatch(raw)
		if len(match) < 2 {
			return 0, fmt.Errorf("missing %s", name)
		}
		return strconv.Atoi(match[1])
	}
	valueFloat := func(name string, pattern *regexp.Regexp) (float64, error) {
		match := pattern.FindStringSubmatch(raw)
		if len(match) < 2 {
			return 0, fmt.Errorf("missing %s", name)
		}
		return strconv.ParseFloat(match[1], 64)
	}
	var sample IsolatedWSSCapacity
	var err error
	if sample.Requested, err = valueInt("connections requested", connectionRequestedPattern); err != nil {
		return sample, err
	}
	if sample.Opened, err = valueInt("connections opened", connectionOpenedPattern); err != nil {
		return sample, err
	}
	if sample.Failed, err = valueInt("connections failed", connectionFailedPattern); err != nil {
		return sample, err
	}
	if sample.Attempts, err = valueInt("handshake attempts", handshakeAttemptsPattern); err != nil {
		return sample, err
	}
	if sample.OpenRate, err = valueFloat("open rate", openRatePattern); err != nil {
		return sample, err
	}
	if sample.P50MS, err = valueFloat("handshake p50", handshakeP50Pattern); err != nil {
		return sample, err
	}
	if sample.P95MS, err = valueFloat("handshake p95", handshakeP95Pattern); err != nil {
		return sample, err
	}
	if sample.P99MS, err = valueFloat("handshake p99", handshakeP99Pattern); err != nil {
		return sample, err
	}
	return sample, nil
}

type repeatedStringFlag []string

func (values *repeatedStringFlag) String() string {
	return strings.Join(*values, ",")
}

func (values *repeatedStringFlag) Set(value string) error {
	value = strings.TrimSpace(value)
	if value == "" {
		return errors.New("value must not be empty")
	}
	*values = append(*values, value)
	return nil
}

type IsolatedWSSCapacity struct {
	Requested int     `json:"requested"`
	Opened    int     `json:"opened"`
	Failed    int     `json:"failed"`
	Attempts  int     `json:"attempts"`
	OpenRate  float64 `json:"open_rate"`
	P50MS     float64 `json:"handshake_p50_ms"`
	P95MS     float64 `json:"handshake_p95_ms"`
	P99MS     float64 `json:"handshake_p99_ms"`
}

type IsolatedWSSGatewaySample struct {
	Active   BenchRow            `json:"active"`
	Capacity IsolatedWSSCapacity `json:"capacity"`
}

type IsolatedWSSRunSample struct {
	RunDir   string                   `json:"run_dir"`
	Nginx    IsolatedWSSGatewaySample `json:"nginx"`
	Proxysss IsolatedWSSGatewaySample `json:"proxysss"`
}

type IsolatedWSSGatewayMedian struct {
	ActiveOpsPerSec float64 `json:"active_ops_per_sec"`
	ActiveP50MS     float64 `json:"active_p50_ms"`
	ActiveP95MS     float64 `json:"active_p95_ms"`
	ActiveP99MS     float64 `json:"active_p99_ms"`
	ActiveErrorsMax int     `json:"active_errors_max"`
	CapacityOpened  int     `json:"capacity_opened_min"`
	CapacityFailed  int     `json:"capacity_failed_max"`
	CapacityRate    float64 `json:"capacity_open_rate"`
	HandshakeP50MS  float64 `json:"handshake_p50_ms"`
	HandshakeP95MS  float64 `json:"handshake_p95_ms"`
	HandshakeP99MS  float64 `json:"handshake_p99_ms"`
}

type IsolatedWSSSummary struct {
	Samples  int                      `json:"samples"`
	Runs     []IsolatedWSSRunSample   `json:"runs"`
	Nginx    IsolatedWSSGatewayMedian `json:"nginx_median"`
	Proxysss IsolatedWSSGatewayMedian `json:"proxysss_median"`
	Passed   bool                     `json:"passed"`
	Failures []string                 `json:"failures,omitempty"`
}

type IsolatedWSSGateOptions struct {
	RequireActive      bool
	RequireCapacity    bool
	GateActiveOps      bool
	GateActiveLatency  bool
	MinActiveOpsPerSec float64
}

func runSummarizeIsolatedWSS(args []string) error {
	fs := flag.NewFlagSet("summarize-isolated-wss", flag.ContinueOnError)
	var runDirs repeatedStringFlag
	fs.Var(&runDirs, "run-dir", "isolated WSS run directory (repeatable)")
	outJSON := fs.String("out-json", "", "write machine-readable summary")
	outMarkdown := fs.String("out-markdown", "", "write Markdown summary")
	requireActive := fs.Bool("require-active", true, "require and gate active WSS metrics")
	requireCapacity := fs.Bool("require-capacity", true, "require and gate capacity metrics")
	gateActiveOps := fs.Bool("gate-active-ops", true, "require proxysss saturation ops/sec to exceed nginx")
	gateActiveLatency := fs.Bool("gate-active-latency", true, "require proxysss p50/p95/p99 to beat nginx")
	minActiveOps := fs.Float64("min-active-ops", 0, "minimum ops/sec required from both gateways in every run")
	strict := fs.Bool("strict", false, "fail unless every median proxysss metric strictly beats nginx")
	fs.SetOutput(io.Discard)
	if err := fs.Parse(args); err != nil {
		return err
	}
	if len(runDirs) == 0 {
		return errors.New("summarize-isolated-wss: at least one --run-dir is required")
	}

	options := IsolatedWSSGateOptions{
		RequireActive:      *requireActive,
		RequireCapacity:    *requireCapacity,
		GateActiveOps:      *gateActiveOps,
		GateActiveLatency:  *gateActiveLatency,
		MinActiveOpsPerSec: *minActiveOps,
	}
	summary, err := summarizeIsolatedWSSRuns(runDirs, options)
	if err != nil {
		return err
	}
	if *outJSON != "" {
		payload, err := json.MarshalIndent(summary, "", "  ")
		if err != nil {
			return err
		}
		if err := os.WriteFile(*outJSON, append(payload, '\n'), 0o644); err != nil {
			return err
		}
	}
	markdown := renderIsolatedWSSMarkdown(summary, *requireActive, *requireCapacity)
	if *outMarkdown != "" {
		if err := os.WriteFile(*outMarkdown, []byte(markdown), 0o644); err != nil {
			return err
		}
	}
	fmt.Print(markdown)
	if *strict && !summary.Passed {
		return fmt.Errorf("strict isolated WSS median gate failed: %s", strings.Join(summary.Failures, "; "))
	}
	return nil
}

func summarizeIsolatedWSSRuns(
	runDirs []string,
	options IsolatedWSSGateOptions,
) (IsolatedWSSSummary, error) {
	summary := IsolatedWSSSummary{Samples: len(runDirs), Passed: true}
	for _, runDir := range runDirs {
		sample := IsolatedWSSRunSample{RunDir: filepath.Clean(runDir)}
		for gateway, target := range map[string]*IsolatedWSSGatewaySample{
			"nginx": &sample.Nginx, "proxysss": &sample.Proxysss,
		} {
			if options.RequireActive {
				raw, err := os.ReadFile(filepath.Join(runDir, gateway+"-active.txt"))
				if err != nil {
					return summary, fmt.Errorf("read %s active sample: %w", gateway, err)
				}
				target.Active = parseBenchOutput(string(raw))
				if target.Active.OpsPerSec <= 0 || target.Active.LatencyP50MS == nil ||
					target.Active.LatencyP95MS == nil || target.Active.LatencyP99MS == nil {
					return summary, fmt.Errorf("%s active sample in %s is incomplete", gateway, runDir)
				}
			}
			if options.RequireCapacity {
				raw, err := os.ReadFile(filepath.Join(runDir, gateway+"-capacity-total.txt"))
				if err != nil {
					return summary, fmt.Errorf("read %s capacity sample: %w", gateway, err)
				}
				target.Capacity, err = parseIsolatedWSSCapacity(string(raw))
				if err != nil {
					return summary, fmt.Errorf("parse %s capacity sample in %s: %w", gateway, runDir, err)
				}
			}
		}
		summary.Runs = append(summary.Runs, sample)
	}

	summary.Nginx = isolatedWSSMedian(summary.Runs, false, options.RequireActive, options.RequireCapacity)
	summary.Proxysss = isolatedWSSMedian(summary.Runs, true, options.RequireActive, options.RequireCapacity)
	fail := func(condition bool, message string) {
		if condition {
			summary.Passed = false
			summary.Failures = append(summary.Failures, message)
		}
	}
	if options.RequireActive {
		fail(summary.Nginx.ActiveErrorsMax != 0 || summary.Proxysss.ActiveErrorsMax != 0,
			fmt.Sprintf("active errors nginx=%d proxysss=%d", summary.Nginx.ActiveErrorsMax, summary.Proxysss.ActiveErrorsMax))
		if options.MinActiveOpsPerSec > 0 {
			for _, run := range summary.Runs {
				fail(run.Nginx.Active.OpsPerSec < options.MinActiveOpsPerSec,
					fmt.Sprintf("nginx active ops/sec %.3f below %.3f in %s", run.Nginx.Active.OpsPerSec, options.MinActiveOpsPerSec, run.RunDir))
				fail(run.Proxysss.Active.OpsPerSec < options.MinActiveOpsPerSec,
					fmt.Sprintf("proxysss active ops/sec %.3f below %.3f in %s", run.Proxysss.Active.OpsPerSec, options.MinActiveOpsPerSec, run.RunDir))
			}
		}
		if options.GateActiveOps {
			fail(summary.Proxysss.ActiveOpsPerSec <= summary.Nginx.ActiveOpsPerSec,
				fmt.Sprintf("active ops/sec proxysss=%.3f nginx=%.3f", summary.Proxysss.ActiveOpsPerSec, summary.Nginx.ActiveOpsPerSec))
		}
		if options.GateActiveLatency {
			fail(summary.Proxysss.ActiveP50MS >= summary.Nginx.ActiveP50MS,
				fmt.Sprintf("active p50 proxysss=%.3f nginx=%.3f", summary.Proxysss.ActiveP50MS, summary.Nginx.ActiveP50MS))
			fail(summary.Proxysss.ActiveP95MS >= summary.Nginx.ActiveP95MS,
				fmt.Sprintf("active p95 proxysss=%.3f nginx=%.3f", summary.Proxysss.ActiveP95MS, summary.Nginx.ActiveP95MS))
			fail(summary.Proxysss.ActiveP99MS >= summary.Nginx.ActiveP99MS,
				fmt.Sprintf("active p99 proxysss=%.3f nginx=%.3f", summary.Proxysss.ActiveP99MS, summary.Nginx.ActiveP99MS))
		}
	}
	if options.RequireCapacity {
		for _, run := range summary.Runs {
			fail(run.Nginx.Capacity.Failed != 0 || run.Nginx.Capacity.Opened != run.Nginx.Capacity.Requested,
				fmt.Sprintf("nginx capacity incomplete in %s", run.RunDir))
			fail(run.Proxysss.Capacity.Failed != 0 || run.Proxysss.Capacity.Opened != run.Proxysss.Capacity.Requested,
				fmt.Sprintf("proxysss capacity incomplete in %s", run.RunDir))
		}
		fail(summary.Proxysss.CapacityRate <= summary.Nginx.CapacityRate,
			fmt.Sprintf("capacity open rate proxysss=%.3f nginx=%.3f", summary.Proxysss.CapacityRate, summary.Nginx.CapacityRate))
		fail(summary.Proxysss.HandshakeP50MS >= summary.Nginx.HandshakeP50MS,
			fmt.Sprintf("handshake p50 proxysss=%.3f nginx=%.3f", summary.Proxysss.HandshakeP50MS, summary.Nginx.HandshakeP50MS))
		fail(summary.Proxysss.HandshakeP95MS >= summary.Nginx.HandshakeP95MS,
			fmt.Sprintf("handshake p95 proxysss=%.3f nginx=%.3f", summary.Proxysss.HandshakeP95MS, summary.Nginx.HandshakeP95MS))
		fail(summary.Proxysss.HandshakeP99MS >= summary.Nginx.HandshakeP99MS,
			fmt.Sprintf("handshake p99 proxysss=%.3f nginx=%.3f", summary.Proxysss.HandshakeP99MS, summary.Nginx.HandshakeP99MS))
	}
	return summary, nil
}

func parseIsolatedWSSCapacity(raw string) (IsolatedWSSCapacity, error) {
	values := map[string]string{}
	for _, field := range strings.Fields(raw) {
		key, value, ok := strings.Cut(field, "=")
		if ok {
			values[key] = value
		}
	}
	required := []string{"requested", "opened", "failed", "attempts", "open_rate", "worst_p50_ms", "worst_p95_ms", "worst_p99_ms"}
	for _, key := range required {
		if values[key] == "" {
			return IsolatedWSSCapacity{}, fmt.Errorf("missing %s", key)
		}
	}
	parseInt := func(key string) (int, error) { return strconv.Atoi(values[key]) }
	parseFloat := func(key string) (float64, error) { return strconv.ParseFloat(values[key], 64) }
	var capacity IsolatedWSSCapacity
	var err error
	if capacity.Requested, err = parseInt("requested"); err != nil {
		return capacity, err
	}
	if capacity.Opened, err = parseInt("opened"); err != nil {
		return capacity, err
	}
	if capacity.Failed, err = parseInt("failed"); err != nil {
		return capacity, err
	}
	if capacity.Attempts, err = parseInt("attempts"); err != nil {
		return capacity, err
	}
	if capacity.OpenRate, err = parseFloat("open_rate"); err != nil {
		return capacity, err
	}
	if capacity.P50MS, err = parseFloat("worst_p50_ms"); err != nil {
		return capacity, err
	}
	if capacity.P95MS, err = parseFloat("worst_p95_ms"); err != nil {
		return capacity, err
	}
	if capacity.P99MS, err = parseFloat("worst_p99_ms"); err != nil {
		return capacity, err
	}
	return capacity, nil
}

func isolatedWSSMedian(
	runs []IsolatedWSSRunSample,
	proxy bool,
	requireActive bool,
	requireCapacity bool,
) IsolatedWSSGatewayMedian {
	var result IsolatedWSSGatewayMedian
	result.CapacityOpened = int(^uint(0) >> 1)
	var ops, activeP50, activeP95, activeP99 []float64
	var rates, handshakeP50, handshakeP95, handshakeP99 []float64
	for _, run := range runs {
		sample := run.Nginx
		if proxy {
			sample = run.Proxysss
		}
		if requireActive {
			ops = append(ops, sample.Active.OpsPerSec)
			activeP50 = append(activeP50, *sample.Active.LatencyP50MS)
			activeP95 = append(activeP95, *sample.Active.LatencyP95MS)
			activeP99 = append(activeP99, *sample.Active.LatencyP99MS)
			if sample.Active.Errors > result.ActiveErrorsMax {
				result.ActiveErrorsMax = sample.Active.Errors
			}
		}
		if requireCapacity {
			rates = append(rates, sample.Capacity.OpenRate)
			handshakeP50 = append(handshakeP50, sample.Capacity.P50MS)
			handshakeP95 = append(handshakeP95, sample.Capacity.P95MS)
			handshakeP99 = append(handshakeP99, sample.Capacity.P99MS)
			if sample.Capacity.Opened < result.CapacityOpened {
				result.CapacityOpened = sample.Capacity.Opened
			}
			if sample.Capacity.Failed > result.CapacityFailed {
				result.CapacityFailed = sample.Capacity.Failed
			}
		}
	}
	if requireActive {
		result.ActiveOpsPerSec = medianFloat(ops)
		result.ActiveP50MS = medianFloat(activeP50)
		result.ActiveP95MS = medianFloat(activeP95)
		result.ActiveP99MS = medianFloat(activeP99)
	}
	if requireCapacity {
		result.CapacityRate = medianFloat(rates)
		result.HandshakeP50MS = medianFloat(handshakeP50)
		result.HandshakeP95MS = medianFloat(handshakeP95)
		result.HandshakeP99MS = medianFloat(handshakeP99)
	} else {
		result.CapacityOpened = 0
	}
	return result
}

func medianFloat(values []float64) float64 {
	if len(values) == 0 {
		return 0
	}
	values = append([]float64(nil), values...)
	sort.Float64s(values)
	middle := len(values) / 2
	if len(values)%2 == 1 {
		return values[middle]
	}
	return (values[middle-1] + values[middle]) / 2
}

func renderIsolatedWSSMarkdown(summary IsolatedWSSSummary, active bool, capacity bool) string {
	var output strings.Builder
	fmt.Fprintf(&output, "# Isolated WSS median report\n\n- repetitions: %d\n- gate: %s\n\n", summary.Samples, map[bool]string{true: "PASS", false: "FAIL"}[summary.Passed])
	advantage := func(proxy, nginx float64, lowerIsBetter bool) float64 {
		if nginx == 0 {
			return 0
		}
		if lowerIsBetter {
			return (1 - proxy/nginx) * 100
		}
		return (proxy/nginx - 1) * 100
	}
	if active {
		output.WriteString("| Active WSS median | nginx | proxysss | proxysss advantage |\n| --- | ---: | ---: | ---: |\n")
		fmt.Fprintf(&output, "| ops/sec | %.2f | %.2f | %+.2f%% |\n", summary.Nginx.ActiveOpsPerSec, summary.Proxysss.ActiveOpsPerSec, advantage(summary.Proxysss.ActiveOpsPerSec, summary.Nginx.ActiveOpsPerSec, false))
		fmt.Fprintf(&output, "| p50 ms | %.3f | %.3f | %+.2f%% |\n", summary.Nginx.ActiveP50MS, summary.Proxysss.ActiveP50MS, advantage(summary.Proxysss.ActiveP50MS, summary.Nginx.ActiveP50MS, true))
		fmt.Fprintf(&output, "| p95 ms | %.3f | %.3f | %+.2f%% |\n", summary.Nginx.ActiveP95MS, summary.Proxysss.ActiveP95MS, advantage(summary.Proxysss.ActiveP95MS, summary.Nginx.ActiveP95MS, true))
		fmt.Fprintf(&output, "| p99 ms | %.3f | %.3f | %+.2f%% |\n\n", summary.Nginx.ActiveP99MS, summary.Proxysss.ActiveP99MS, advantage(summary.Proxysss.ActiveP99MS, summary.Nginx.ActiveP99MS, true))
	}
	if capacity {
		output.WriteString("| Capacity median | nginx | proxysss | proxysss advantage |\n| --- | ---: | ---: | ---: |\n")
		fmt.Fprintf(&output, "| open rate connections/s | %.2f | %.2f | %+.2f%% |\n", summary.Nginx.CapacityRate, summary.Proxysss.CapacityRate, advantage(summary.Proxysss.CapacityRate, summary.Nginx.CapacityRate, false))
		fmt.Fprintf(&output, "| handshake p50 ms | %.3f | %.3f | %+.2f%% |\n", summary.Nginx.HandshakeP50MS, summary.Proxysss.HandshakeP50MS, advantage(summary.Proxysss.HandshakeP50MS, summary.Nginx.HandshakeP50MS, true))
		fmt.Fprintf(&output, "| handshake p95 ms | %.3f | %.3f | %+.2f%% |\n", summary.Nginx.HandshakeP95MS, summary.Proxysss.HandshakeP95MS, advantage(summary.Proxysss.HandshakeP95MS, summary.Nginx.HandshakeP95MS, true))
		fmt.Fprintf(&output, "| handshake p99 ms | %.3f | %.3f | %+.2f%% |\n\n", summary.Nginx.HandshakeP99MS, summary.Proxysss.HandshakeP99MS, advantage(summary.Proxysss.HandshakeP99MS, summary.Nginx.HandshakeP99MS, true))
	}
	if len(summary.Failures) > 0 {
		output.WriteString("Failures:\n\n")
		for _, failure := range summary.Failures {
			fmt.Fprintf(&output, "- %s\n", failure)
		}
	}
	return output.String()
}

func usage() {
	fmt.Fprintln(os.Stderr, "usage: benchmark-helper <subcommand> [flags]")
}

func runWriteLargeFile(args []string) error {
	fs := flag.NewFlagSet("write-large-file", flag.ContinueOnError)
	path := fs.String("path", "", "output path")
	fs.SetOutput(io.Discard)
	if err := fs.Parse(args); err != nil {
		return err
	}
	if strings.TrimSpace(*path) == "" {
		return errors.New("write-large-file: --path is required")
	}
	payload := bytes.Repeat([]byte("proxysss-large-static-benchmark\n"), 4096*128)
	return os.WriteFile(*path, payload, 0o644)
}

func runServeSSE(args []string) error {
	fs := flag.NewFlagSet("serve-sse", flag.ContinueOnError)
	listen := fs.String("listen", "127.0.0.1:18191", "listen address")
	chunks := fs.Int("chunks", 8, "data chunks per stream")
	fs.SetOutput(io.Discard)
	if err := fs.Parse(args); err != nil {
		return err
	}
	if *chunks < 1 {
		return errors.New("serve-sse: --chunks must be positive")
	}
	handler := http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method == http.MethodPost || r.Method == http.MethodPut || r.Method == http.MethodPatch {
			_, _ = io.Copy(io.Discard, r.Body)
		}
		if strings.HasPrefix(r.URL.Path, "/v1/chat/completions") || strings.HasPrefix(r.URL.Path, "/sse") {
			flusher, ok := w.(http.Flusher)
			if !ok {
				http.Error(w, "streaming unsupported", http.StatusInternalServerError)
				return
			}
			w.Header().Set("Content-Type", "text/event-stream")
			w.Header().Set("Cache-Control", "no-cache")
			w.Header().Set("Connection", "close")
			w.WriteHeader(http.StatusOK)
			for idx := 0; idx < *chunks; idx++ {
				payload := map[string]any{
					"id":     "chatcmpl-proxysss-bench",
					"object": "chat.completion.chunk",
					"choices": []map[string]any{
						{"index": 0, "delta": map[string]any{"content": fmt.Sprintf("token-%d", idx)}},
					},
				}
				chunk, err := json.Marshal(payload)
				if err != nil {
					return
				}
				if _, err := fmt.Fprintf(w, "data: %s\n\n", chunk); err != nil {
					return
				}
				flusher.Flush()
				time.Sleep(2 * time.Millisecond)
			}
			if *chunks > 1 {
				_, _ = io.WriteString(w, "data: [DONE]\n\n")
				flusher.Flush()
			}
			return
		}
		body, _ := json.Marshal(map[string]any{"ok": true, "path": r.URL.Path})
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		_, _ = w.Write(body)
	})
	server := &http.Server{
		Addr:              *listen,
		Handler:           handler,
		ReadHeaderTimeout: 5 * time.Second,
	}
	return server.ListenAndServe()
}

func runParseBench(args []string) error {
	fs := flag.NewFlagSet("parse-bench", flag.ContinueOnError)
	scenario := fs.String("scenario", "", "scenario name")
	gateway := fs.String("gateway", "", "gateway name")
	protocol := fs.String("protocol", "", "protocol")
	target := fs.String("target", "", "target")
	concurrency := fs.Int("concurrency", 0, "concurrency")
	duration := fs.Int("duration", 0, "duration secs")
	fs.SetOutput(io.Discard)
	if err := fs.Parse(args); err != nil {
		return err
	}
	raw, err := io.ReadAll(os.Stdin)
	if err != nil {
		return err
	}
	row := parseBenchOutput(string(raw))
	row.Scenario = *scenario
	row.Gateway = *gateway
	if strings.TrimSpace(*scenario) == "" {
		row.Name = *gateway
	} else {
		row.Name = fmt.Sprintf("%s:%s", *scenario, *gateway)
	}
	row.Protocol = *protocol
	row.Target = *target
	row.Concurrency = *concurrency
	row.DurationSecs = *duration
	row.URL = *target
	payload, err := json.Marshal(row)
	if err != nil {
		return err
	}
	fmt.Println(string(payload))
	return nil
}

func parseBenchOutput(output string) BenchRow {
	row := BenchRow{}
	row.Success = matchInt(output, successPattern)
	row.Errors = matchInt(output, errorPattern)
	row.OpsPerSec = matchFloat(output, opsPattern)
	row.ThroughputMiBS = matchFloat(output, throughputPattern)
	row.TargetOpsPerSec = matchOptionalFloat(output, targetOpsPattern)
	row.LatencyP50MS = matchOptionalFloat(output, p50Pattern)
	row.LatencyP95MS = matchOptionalFloat(output, p95Pattern)
	row.LatencyP99MS = matchOptionalFloat(output, p99Pattern)
	return row
}

func matchInt(raw string, pattern *regexp.Regexp) int {
	match := pattern.FindStringSubmatch(raw)
	if len(match) < 2 {
		return 0
	}
	value, _ := strconv.Atoi(match[1])
	return value
}

func matchFloat(raw string, pattern *regexp.Regexp) float64 {
	match := pattern.FindStringSubmatch(raw)
	if len(match) < 2 {
		return 0
	}
	value, _ := strconv.ParseFloat(match[1], 64)
	return value
}

func matchOptionalFloat(raw string, pattern *regexp.Regexp) *float64 {
	match := pattern.FindStringSubmatch(raw)
	if len(match) < 2 {
		return nil
	}
	value, err := strconv.ParseFloat(match[1], 64)
	if err != nil {
		return nil
	}
	return &value
}

func runWriteJSONArray(args []string) error {
	fs := flag.NewFlagSet("write-json-array", flag.ContinueOnError)
	inPath := fs.String("in", "", "jsonl input path")
	outPath := fs.String("out", "", "json output path")
	fs.SetOutput(io.Discard)
	if err := fs.Parse(args); err != nil {
		return err
	}
	if *inPath == "" || *outPath == "" {
		return errors.New("write-json-array: --in and --out are required")
	}
	rows, err := loadJSONLRows(*inPath)
	if err != nil {
		return err
	}
	payload, err := json.MarshalIndent(rows, "", "  ")
	if err != nil {
		return err
	}
	payload = append(payload, '\n')
	return os.WriteFile(*outPath, payload, 0o644)
}

// runAggregateBenchMedians turns repeated, interleaved samples into one row
// per scenario/gateway. Throughput and percentiles use medians so a one-off
// scheduler tick cannot decide a release; errors use the maximum so a flaky
// repetition can never be hidden by aggregation.
func runAggregateBenchMedians(args []string) error {
	fs := flag.NewFlagSet("aggregate-bench-medians", flag.ContinueOnError)
	inPath := fs.String("in", "", "repeated jsonl input path")
	outPath := fs.String("out", "", "aggregated json output path")
	fs.SetOutput(io.Discard)
	if err := fs.Parse(args); err != nil {
		return err
	}
	if *inPath == "" || *outPath == "" {
		return errors.New("aggregate-bench-medians: --in and --out are required")
	}
	rows, err := loadJSONLRows(*inPath)
	if err != nil {
		return err
	}
	aggregated, err := aggregateBenchRowMedians(rows)
	if err != nil {
		return err
	}
	payload, err := json.MarshalIndent(aggregated, "", "  ")
	if err != nil {
		return err
	}
	payload = append(payload, '\n')
	return os.WriteFile(*outPath, payload, 0o644)
}

func aggregateBenchRowMedians(rows []BenchRow) ([]BenchRow, error) {
	type key struct{ scenario, gateway string }
	groups := make(map[key][]BenchRow)
	for _, row := range rows {
		if row.Scenario == "" || row.Gateway == "" {
			return nil, errors.New("median aggregation requires scenario and gateway on every row")
		}
		k := key{row.Scenario, row.Gateway}
		groups[k] = append(groups[k], row)
	}
	if len(groups) == 0 {
		return nil, errors.New("median aggregation received no rows")
	}
	keys := make([]key, 0, len(groups))
	for k := range groups {
		keys = append(keys, k)
	}
	sort.Slice(keys, func(i, j int) bool {
		if keys[i].scenario == keys[j].scenario {
			return keys[i].gateway < keys[j].gateway
		}
		return keys[i].scenario < keys[j].scenario
	})
	result := make([]BenchRow, 0, len(keys))
	for _, k := range keys {
		samples := groups[k]
		row := samples[0]
		ops := make([]float64, 0, len(samples))
		throughput := make([]float64, 0, len(samples))
		success := make([]float64, 0, len(samples))
		var targets, p50s, p95s, p99s []float64
		row.Errors = 0
		for _, sample := range samples {
			ops = append(ops, sample.OpsPerSec)
			throughput = append(throughput, sample.ThroughputMiBS)
			success = append(success, float64(sample.Success))
			row.Errors = max(row.Errors, sample.Errors)
			appendOptional := func(dst *[]float64, value *float64) {
				if value != nil {
					*dst = append(*dst, *value)
				}
			}
			appendOptional(&targets, sample.TargetOpsPerSec)
			appendOptional(&p50s, sample.LatencyP50MS)
			appendOptional(&p95s, sample.LatencyP95MS)
			appendOptional(&p99s, sample.LatencyP99MS)
		}
		row.OpsPerSec = medianFloat(ops)
		row.ThroughputMiBS = medianFloat(throughput)
		row.Success = int(math.Round(medianFloat(success)))
		row.TargetOpsPerSec = medianFloatPointer(targets, len(samples))
		row.LatencyP50MS = medianFloatPointer(p50s, len(samples))
		row.LatencyP95MS = medianFloatPointer(p95s, len(samples))
		row.LatencyP99MS = medianFloatPointer(p99s, len(samples))
		result = append(result, row)
	}
	return result, nil
}

func medianFloatPointer(values []float64, expected int) *float64 {
	if len(values) != expected || expected == 0 {
		return nil
	}
	value := medianFloat(values)
	return &value
}

func runWriteEqualLoadPlan(args []string) error {
	fs := flag.NewFlagSet("write-equal-load-plan", flag.ContinueOnError)
	resultsPath := fs.String("results", "", "saturation results json path")
	outPath := fs.String("out", "", "pipe-delimited output path")
	fraction := fs.Float64("fraction", 0.70, "fraction of the slower gateway saturation rate")
	fs.SetOutput(io.Discard)
	if err := fs.Parse(args); err != nil {
		return err
	}
	if *resultsPath == "" || *outPath == "" {
		return errors.New("write-equal-load-plan: --results and --out are required")
	}
	if *fraction <= 0 || *fraction >= 1 {
		return errors.New("write-equal-load-plan: --fraction must be between 0 and 1")
	}
	rows, err := loadResults(*resultsPath)
	if err != nil {
		return err
	}
	byScenario := map[string]map[string]BenchRow{}
	for _, row := range rows {
		if byScenario[row.Scenario] == nil {
			byScenario[row.Scenario] = map[string]BenchRow{}
		}
		byScenario[row.Scenario][row.Gateway] = row
	}

	var lines []string
	for _, scenario := range sortedKeys(byScenario) {
		gateways := byScenario[scenario]
		proxy, proxyOK := gateways["proxysss"]
		nginx, nginxOK := gateways["nginx"]
		if !proxyOK || !nginxOK {
			return fmt.Errorf("equal-load plan %s requires proxysss and nginx rows", scenario)
		}
		concurrency := proxy.Concurrency
		if concurrency <= 0 {
			concurrency = nginx.Concurrency
		}
		if concurrency <= 0 || (proxy.Concurrency > 0 && nginx.Concurrency > 0 && proxy.Concurrency != nginx.Concurrency) {
			return fmt.Errorf("equal-load plan %s has invalid/mismatched concurrency", scenario)
		}
		slowerOps := math.Min(proxy.OpsPerSec, nginx.OpsPerSec)
		if slowerOps <= 0 {
			return fmt.Errorf("equal-load plan %s has non-positive saturation rate", scenario)
		}
		desiredOps := slowerOps * *fraction
		intervalMicros := int64(math.Ceil(float64(concurrency) * 1_000_000 / desiredOps))
		if intervalMicros < 1 {
			intervalMicros = 1
		}
		actualTarget := float64(concurrency) * 1_000_000 / float64(intervalMicros)
		lines = append(lines, fmt.Sprintf("%s|%d|%.6f", scenario, intervalMicros, actualTarget))
	}
	if len(lines) == 0 {
		return errors.New("write-equal-load-plan: no benchmark scenarios found")
	}
	return os.WriteFile(*outPath, []byte(strings.Join(lines, "\n")+"\n"), 0o644)
}

func runQuickGate(args []string) error {
	fs := flag.NewFlagSet("quick-gate", flag.ContinueOnError)
	minRatio := fs.Float64("min-ratio", 0.97, "minimum acceptable ratio")
	maxLatencyRatio := fs.Float64("max-latency-ratio", 1.0, "maximum proxysss/nginx latency ratio")
	requireLatency := fs.Bool("require-latency-percentiles", false, "require p50/p95/p99 for both gateways")
	fs.Bool("require-zero-errors", false, "require zero errors from both gateways (quick gate is always zero-error)")
	strictSuperiority := fs.Bool("strict-superiority", false, "require ops ratio above the floor and every latency ratio below its ceiling")
	rowsPath := fs.String("rows", "", "jsonl rows path")
	fs.SetOutput(io.Discard)
	if err := fs.Parse(args); err != nil {
		return err
	}
	rows, err := loadJSONLRows(*rowsPath)
	if err != nil {
		return err
	}
	byScenario := map[string]map[string]BenchRow{}
	for _, row := range rows {
		if byScenario[row.Scenario] == nil {
			byScenario[row.Scenario] = map[string]BenchRow{}
		}
		byScenario[row.Scenario][row.Gateway] = row
	}
	var failures []string
	scenarios := sortedKeys(byScenario)
	for _, scenario := range scenarios {
		gateways := byScenario[scenario]
		proxy := gateways["proxysss"]
		nginx := gateways["nginx"]
		ratio := safeRatio(proxy.OpsPerSec, nginx.OpsPerSec)
		fmt.Printf("quick gate %s: proxysss=%.2f nginx=%.2f ratio=%.3fx\n", scenario, proxy.OpsPerSec, nginx.OpsPerSec, ratio)
		// The fast gate has always rejected any benchmark error. Keep that
		// invariant even when the full summary uses protocol tolerances.
		if proxy.Errors > 0 || nginx.Errors > 0 {
			failures = append(failures, fmt.Sprintf("%s errors proxysss=%d nginx=%d", scenario, proxy.Errors, nginx.Errors))
		} else if ratioGateFailed(ratio, *minRatio, *strictSuperiority) {
			failures = append(failures, fmt.Sprintf("%s ratio=%.3fx < %.2fx", scenario, ratio, *minRatio))
		}
		failures = append(failures, latencyGateFailures(scenario, proxy, nginx, *maxLatencyRatio, *requireLatency, *strictSuperiority)...)
	}
	if len(failures) > 0 {
		return fmt.Errorf("quick benchmark gate failed; deep matrix skipped: %s", strings.Join(failures, "; "))
	}
	fmt.Println("quick benchmark gate passed; starting deep matrix")
	return nil
}

func runWriteAllScenariosSummary(args []string) error {
	fs := flag.NewFlagSet("write-all-scenarios-summary", flag.ContinueOnError)
	resultsPath := fs.String("results", "", "results json path")
	mdPath := fs.String("md", "", "summary markdown path")
	htmlPath := fs.String("html", "", "summary html path")
	minRatio := fs.Float64("min-ratio", 0.50, "non-diagnostic min ratio")
	criticalRatio := fs.Float64("critical-ratio", 0.97, "critical scenario ratio")
	criticalScenarios := fs.String("critical-scenarios", "", "space-separated critical scenarios")
	diagnosticScenarios := fs.String("diagnostic-scenarios", "", "space-separated diagnostic scenarios")
	websocketTolerance := fs.Int("websocket-error-tolerance", 4, "websocket error tolerance")
	sseTolerance := fs.Int("sse-error-tolerance", 1, "sse error tolerance")
	udpTolerance := fs.Int("udp-error-tolerance", 4, "udp error tolerance")
	aggregateRatio := fs.Float64("aggregate-ratio", 0.97, "aggregate mixed ratio")
	maxLatencyRatio := fs.Float64("max-latency-ratio", 1.0, "maximum proxysss/nginx p50/p95/p99 ratio")
	requireLatency := fs.Bool("require-latency-percentiles", false, "require p50/p95/p99 for both gateways")
	requireZeroErrors := fs.Bool("require-zero-errors", false, "require zero errors from both gateways")
	strictSuperiority := fs.Bool("strict-superiority", false, "require ops ratios strictly above floors and latency ratios strictly below ceiling")
	gateOps := fs.Bool("gate-ops", true, "gate saturation ops ratios and aggregate throughput")
	gateLatency := fs.Bool("gate-latency", true, "gate p50/p95/p99 latency ratios")
	minTargetAchievement := fs.Float64("min-target-achievement", 0, "minimum actual/target ops ratio for fixed offered-load rows")
	phase := fs.String("phase", "saturation", "benchmark phase label")
	mixedMatrix := fs.Bool("mixed-matrix", true, "whether results came from mixed matrix")
	cpuCores := fs.String("cpu-cores", "", "detected cores")
	httpConcurrency := fs.String("http-concurrency", "", "http concurrency")
	httpsConcurrency := fs.String("https-concurrency", "", "https concurrency")
	staticLargeConcurrency := fs.String("static-large-concurrency", "", "static large concurrency")
	sseConcurrency := fs.String("sse-concurrency", "", "sse concurrency")
	streamConnections := fs.String("stream-connections", "", "stream connections")
	samplesPerGateway := fs.Int("samples-per-gateway", 1, "interleaved samples aggregated for each gateway")
	fs.SetOutput(io.Discard)
	if err := fs.Parse(args); err != nil {
		return err
	}
	rows, err := loadResults(*resultsPath)
	if err != nil {
		return err
	}
	criticalSet := makeSet(*criticalScenarios)
	diagnosticSet := makeSet(*diagnosticScenarios)
	byScenario := map[string]map[string]BenchRow{}
	for _, row := range rows {
		if byScenario[row.Scenario] == nil {
			byScenario[row.Scenario] = map[string]BenchRow{}
		}
		byScenario[row.Scenario][row.Gateway] = row
	}
	var errorFailures []string
	var targetFailures []string
	var referenceTargetWarnings []string
	scenarios := sortedKeys(byScenario)
	for _, scenario := range scenarios {
		gateways := byScenario[scenario]
		proxy := gateways["proxysss"]
		nginx := gateways["nginx"]
		protocol := proxy.Protocol
		if protocol == "" {
			protocol = nginx.Protocol
		}
		if *requireZeroErrors {
			if proxy.Errors > 0 || nginx.Errors > 0 {
				errorFailures = append(errorFailures, fmt.Sprintf("%s errors proxysss=%d nginx=%d", scenario, proxy.Errors, nginx.Errors))
			}
		} else {
			switch protocol {
			case "udp":
				if proxy.Errors > nginx.Errors+*udpTolerance {
					errorFailures = append(errorFailures, fmt.Sprintf("%s udp errors proxysss=%d nginx=%d", scenario, proxy.Errors, nginx.Errors))
				}
			case "sse":
				if proxy.Errors > nginx.Errors+*sseTolerance {
					errorFailures = append(errorFailures, fmt.Sprintf("%s sse errors proxysss=%d nginx=%d", scenario, proxy.Errors, nginx.Errors))
				}
			case "websocket":
				if proxy.Errors > nginx.Errors+*websocketTolerance {
					errorFailures = append(errorFailures, fmt.Sprintf("%s websocket errors proxysss=%d nginx=%d", scenario, proxy.Errors, nginx.Errors))
				}
			default:
				if proxy.Errors > 0 || nginx.Errors > 0 {
					errorFailures = append(errorFailures, fmt.Sprintf("%s errors proxysss=%d nginx=%d", scenario, proxy.Errors, nginx.Errors))
				}
			}
		}
		if *minTargetAchievement > 0 {
			for _, gateway := range []string{"proxysss", "nginx"} {
				row := gateways[gateway]
				if row.TargetOpsPerSec == nil || *row.TargetOpsPerSec <= 0 {
					targetFailures = append(targetFailures, fmt.Sprintf("%s %s missing fixed offered-load target", scenario, gateway))
					continue
				}
				achievement := row.OpsPerSec / *row.TargetOpsPerSec
				if achievement < *minTargetAchievement {
					message := fmt.Sprintf(
						"%s %s target achievement %.3f < %.3f (actual=%.2f target=%.2f)",
						scenario, gateway, achievement, *minTargetAchievement, row.OpsPerSec, *row.TargetOpsPerSec,
					)
					if gateway == "proxysss" {
						targetFailures = append(targetFailures, message)
					} else {
						referenceTargetWarnings = append(referenceTargetWarnings, message)
					}
				}
			}
		}
	}

	var lines []string
	lines = append(lines,
		"# proxysss all-scenarios benchmark",
		"",
		fmt.Sprintf("- Matrix mode: `%s`", ternary(*mixedMatrix, "mixed concurrent", "serial diagnostic")),
		fmt.Sprintf("- Measurement phase: `%s`", *phase),
		fmt.Sprintf("- Interleaved samples per gateway: `%d` (median metrics, maximum observed errors)", *samplesPerGateway),
		fmt.Sprintf("- Detected CPU cores: `%s`", *cpuCores),
		fmt.Sprintf("- Auto concurrency: HTTP `%s`, HTTPS `%s`, static-large `%s`, SSE `%s`, TCP/UDP/WebSocket `%s`", *httpConcurrency, *httpsConcurrency, *staticLargeConcurrency, *sseConcurrency, *streamConnections),
		fmt.Sprintf("- Non-critical minimum proxysss/nginx ops ratio: `%.2f` except diagnostic scenarios `%s`", *minRatio, strings.Join(sortedSetMembers(diagnosticSet), ", ")),
		fmt.Sprintf("- SSE stream error tolerance: `proxysss <= nginx + %d`", *sseTolerance),
		fmt.Sprintf("- WebSocket reconnect/error tolerance: `proxysss <= nginx + %d`", *websocketTolerance),
		fmt.Sprintf("- UDP datagram error tolerance: `proxysss <= nginx + %d`", *udpTolerance),
		fmt.Sprintf("- Critical long-connection fair ratio gate: `%.2f` for `%s`", *criticalRatio, strings.Join(sortedSetMembers(criticalSet), ", ")),
		fmt.Sprintf("- Aggregate mixed-load fair ratio gate: `%.2f`", *aggregateRatio),
		fmt.Sprintf("- Maximum proxysss/nginx p50/p95/p99 latency ratio: `%.2f` (required=%t, strict=%t)", *maxLatencyRatio, *requireLatency, *strictSuperiority),
		fmt.Sprintf("- Saturation ops gate: `%t`", *gateOps),
		fmt.Sprintf("- Equal-load latency gate: `%t`", *gateLatency),
		fmt.Sprintf("- Minimum fixed-load completion: `%.3f`", *minTargetAchievement),
		fmt.Sprintf("- Reference under-target policy: `report warning; candidate must still meet target and win latency`"),
		fmt.Sprintf("- Zero-error gate: `%t`", *requireZeroErrors),
		fmt.Sprintf("- Result file: `%s`", *resultsPath),
		"",
		"| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |",
		"| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
	)
	if len(referenceTargetWarnings) > 0 {
		lines = append(lines, fmt.Sprintf("- Reference under-target warnings: `%s`", strings.Join(referenceTargetWarnings, "; ")))
	}

	var ratioPairs []struct {
		Scenario string
		Ratio    float64
	}
	var proxyTotalOps float64
	var nginxTotalOps float64
	for _, scenario := range scenarios {
		proxy := byScenario[scenario]["proxysss"]
		nginx := byScenario[scenario]["nginx"]
		ratio := safeRatio(proxy.OpsPerSec, nginx.OpsPerSec)
		proxyTotalOps += proxy.OpsPerSec
		nginxTotalOps += nginx.OpsPerSec
		ratioPairs = append(ratioPairs, struct {
			Scenario string
			Ratio    float64
		}{Scenario: scenario, Ratio: ratio})
		errCount := proxy.Errors + nginx.Errors
		target, proxyCompletion, nginxCompletion := fixedLoadColumns(proxy, nginx)
		lines = append(lines, fmt.Sprintf(
			"| %s | %.2f | %.2f | %.3fx | %s | %s | %s | %s | %.3fx | %s | %.3fx | %s | %.3fx | %s | %d |",
			scenario,
			proxy.OpsPerSec,
			nginx.OpsPerSec,
			ratio,
			opsImprovementPercent(ratio),
			target,
			proxyCompletion,
			nginxCompletion,
			latencyRatio(proxy.LatencyP50MS, nginx.LatencyP50MS),
			latencyImprovementPercent(proxy.LatencyP50MS, nginx.LatencyP50MS),
			latencyRatio(proxy.LatencyP95MS, nginx.LatencyP95MS),
			latencyImprovementPercent(proxy.LatencyP95MS, nginx.LatencyP95MS),
			latencyRatio(proxy.LatencyP99MS, nginx.LatencyP99MS),
			latencyImprovementPercent(proxy.LatencyP99MS, nginx.LatencyP99MS),
			errCount,
		))
	}
	aggregate := safeRatio(proxyTotalOps, nginxTotalOps)
	lines = append(lines,
		"",
		fmt.Sprintf("- Aggregate proxysss ops/s: `%.2f`", proxyTotalOps),
		fmt.Sprintf("- Aggregate nginx ops/s: `%.2f`", nginxTotalOps),
		fmt.Sprintf("- Aggregate proxysss/nginx ratio: `%.3fx`", aggregate),
		fmt.Sprintf("- Aggregate throughput improvement: `%s`", opsImprovementPercent(aggregate)),
		"",
	)
	if err := os.WriteFile(*mdPath, []byte(strings.Join(lines, "\n")), 0o644); err != nil {
		return err
	}
	if err := os.WriteFile(*htmlPath, []byte(buildAllScenariosHTML(byScenario, ratioPairs, aggregate)), 0o644); err != nil {
		return err
	}

	var softFailures []string
	var latencyFailures []string
	for _, item := range ratioPairs {
		if *gateOps && !diagnosticSet[item.Scenario] && ratioGateFailed(item.Ratio, *minRatio, *strictSuperiority) {
			softFailures = append(softFailures, fmt.Sprintf("%s ratio=%.3f", item.Scenario, item.Ratio))
		}
		proxy := byScenario[item.Scenario]["proxysss"]
		nginx := byScenario[item.Scenario]["nginx"]
		if *gateLatency {
			latencyFailures = append(latencyFailures, latencyGateFailures(item.Scenario, proxy, nginx, *maxLatencyRatio, *requireLatency, *strictSuperiority)...)
		}
	}
	var criticalFailures []string
	for _, item := range ratioPairs {
		if *gateOps && criticalSet[item.Scenario] && ratioGateFailed(item.Ratio, *criticalRatio, *strictSuperiority) {
			criticalFailures = append(criticalFailures, fmt.Sprintf("%s ratio=%.3f < %.2f", item.Scenario, item.Ratio, *criticalRatio))
		}
	}
	var gateFailures []string
	if len(errorFailures) > 0 {
		gateFailures = append(gateFailures, "errors: "+strings.Join(errorFailures, "; "))
	}
	if len(targetFailures) > 0 {
		gateFailures = append(gateFailures, "fixed offered-load: "+strings.Join(targetFailures, "; "))
	}
	if len(criticalFailures) > 0 {
		gateFailures = append(gateFailures, "critical ratios: "+strings.Join(criticalFailures, "; "))
	}
	if *gateOps && *mixedMatrix && ratioGateFailed(aggregate, *aggregateRatio, *strictSuperiority) {
		gateFailures = append(gateFailures, fmt.Sprintf("aggregate mixed ratio: %.3f < %.2f", aggregate, *aggregateRatio))
	}
	if len(latencyFailures) > 0 {
		gateFailures = append(gateFailures, "latency: "+strings.Join(latencyFailures, "; "))
	}
	if len(softFailures) > 0 {
		gateFailures = append(gateFailures, "ratios: "+strings.Join(softFailures, "; "))
	}
	if len(gateFailures) > 0 {
		return fmt.Errorf("benchmark gate failed: %s", strings.Join(gateFailures, " | "))
	}
	fmt.Printf("all-scenarios benchmark gate passed (%d rows, aggregate ratio %.3fx)\n", len(rows), aggregate)
	return nil
}

func buildAllScenariosHTML(byScenario map[string]map[string]BenchRow, ratios []struct {
	Scenario string
	Ratio    float64
}, aggregate float64) string {
	var bodyRows strings.Builder
	for _, item := range ratios {
		proxy := byScenario[item.Scenario]["proxysss"]
		nginx := byScenario[item.Scenario]["nginx"]
		bodyRows.WriteString("<tr>")
		bodyRows.WriteString(fmt.Sprintf("<td>%s</td>", html.EscapeString(item.Scenario)))
		bodyRows.WriteString(fmt.Sprintf("<td>%.2f</td>", proxy.OpsPerSec))
		bodyRows.WriteString(fmt.Sprintf("<td>%.2f</td>", nginx.OpsPerSec))
		bodyRows.WriteString(fmt.Sprintf("<td>%.3fx</td>", item.Ratio))
		bodyRows.WriteString(fmt.Sprintf("<td>%s</td>", opsImprovementPercent(item.Ratio)))
		bodyRows.WriteString(fmt.Sprintf("<td>%.3fx</td>", latencyRatio(proxy.LatencyP50MS, nginx.LatencyP50MS)))
		bodyRows.WriteString(fmt.Sprintf("<td>%s</td>", latencyImprovementPercent(proxy.LatencyP50MS, nginx.LatencyP50MS)))
		bodyRows.WriteString(fmt.Sprintf("<td>%.3fx</td>", latencyRatio(proxy.LatencyP95MS, nginx.LatencyP95MS)))
		bodyRows.WriteString(fmt.Sprintf("<td>%s</td>", latencyImprovementPercent(proxy.LatencyP95MS, nginx.LatencyP95MS)))
		bodyRows.WriteString(fmt.Sprintf("<td>%.3fx</td>", latencyRatio(proxy.LatencyP99MS, nginx.LatencyP99MS)))
		bodyRows.WriteString(fmt.Sprintf("<td>%s</td>", latencyImprovementPercent(proxy.LatencyP99MS, nginx.LatencyP99MS)))
		bodyRows.WriteString("</tr>")
	}
	return fmt.Sprintf(`<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <title>proxysss all-scenarios benchmark</title>
  <style>
    body { font-family: Segoe UI, system-ui, sans-serif; margin: 0; background: #0b1020; color: #e8eefc; }
    main { max-width: 1100px; margin: 0 auto; padding: 2rem 1.25rem 3rem; }
    table { width: 100%%; border-collapse: collapse; background: #121a2f; }
    th, td { border: 1px solid #24304f; padding: 0.7rem 0.8rem; text-align: right; }
    th:first-child, td:first-child { text-align: left; }
    h1 { color: #5eead4; }
  </style>
</head>
<body>
  <main>
    <h1>proxysss all-scenarios benchmark</h1>
    <p>Aggregate proxysss/nginx ratio: <strong>%.3fx</strong></p>
    <table>
      <thead>
		<tr><th>Scenario</th><th>proxysss ops/s</th><th>nginx ops/s</th><th>ops ratio</th><th>ops improvement</th><th>p50 ratio</th><th>p50 improvement</th><th>p95 ratio</th><th>p95 improvement</th><th>p99 ratio</th><th>p99 improvement</th></tr>
      </thead>
      <tbody>%s</tbody>
    </table>
  </main>
</body>
</html>`, aggregate, bodyRows.String())
}

func runPrintResultsSummary(args []string) error {
	fs := flag.NewFlagSet("print-results-summary", flag.ContinueOnError)
	resultsPath := fs.String("results", "", "results json path")
	fs.SetOutput(io.Discard)
	if err := fs.Parse(args); err != nil {
		return err
	}
	rows, err := loadResults(*resultsPath)
	if err != nil {
		return err
	}
	sort.Slice(rows, func(i, j int) bool { return rows[i].OpsPerSec > rows[j].OpsPerSec })
	fmt.Printf("%-16s %12s %10s %8s %8s\n", "name", "ops/s", "MiB/s", "p50ms", "errors")
	for _, row := range rows {
		fmt.Printf("%-16s %12.2f %10.2f %8.2f %8d\n", row.Name, row.OpsPerSec, row.ThroughputMiBS, floatOrZero(row.LatencyP50MS), row.Errors)
	}
	return nil
}

func runWriteGatewayReport(args []string) error {
	fs := flag.NewFlagSet("write-gateway-report", flag.ContinueOnError)
	resultsPath := fs.String("results", "", "results json path")
	outDir := fs.String("out-dir", "", "output dir")
	concurrency := fs.Int("concurrency", 0, "concurrency")
	duration := fs.Int("duration", 0, "duration secs")
	fs.SetOutput(io.Discard)
	if err := fs.Parse(args); err != nil {
		return err
	}
	rows, err := loadResults(*resultsPath)
	if err != nil {
		return err
	}
	sort.Slice(rows, func(i, j int) bool { return rows[i].OpsPerSec > rows[j].OpsPerSec })
	if err := os.MkdirAll(*outDir, 0o755); err != nil {
		return err
	}
	metaConcurrency := *concurrency
	metaDuration := *duration
	if metaConcurrency == 0 && len(rows) > 0 {
		metaConcurrency = rows[0].Concurrency
	}
	if metaDuration == 0 && len(rows) > 0 {
		metaDuration = rows[0].DurationSecs
	}
	md := buildGatewayReportMarkdown(rows, metaConcurrency, metaDuration)
	htmlBody := buildGatewayReportHTML(rows, metaConcurrency, metaDuration)
	if err := os.WriteFile(joinPath(*outDir, "report.md"), []byte(md), 0o644); err != nil {
		return err
	}
	if err := os.WriteFile(joinPath(*outDir, "report.html"), []byte(htmlBody), 0o644); err != nil {
		return err
	}
	fmt.Printf("benchmark report markdown: %s\n", joinPath(*outDir, "report.md"))
	fmt.Printf("benchmark report html:     %s\n", joinPath(*outDir, "report.html"))
	return nil
}

func buildGatewayReportMarkdown(rows []BenchRow, concurrency, duration int) string {
	nginx := rowByName(rows, "nginx")
	var lines []string
	lines = append(lines,
		"# Gateway throughput benchmark",
		"",
		fmt.Sprintf("- Generated: %s", time.Now().UTC().Format("2006-01-02 15:04:05 UTC")),
		fmt.Sprintf("- Concurrency: %d", concurrency),
		fmt.Sprintf("- Duration: %ds", duration),
		"- Workload: static `index.html` over HTTP/1.1",
		"",
		"## Summary ranking (ops/sec)",
		"",
		"| Rank | Gateway | ops/sec | vs nginx | MiB/s | p50 ms | p95 ms | p99 ms | success | errors |",
		"| ---: | ------- | ------: | -------: | ----: | -----: | -----: | -----: | ------: | -----: |",
	)
	for index, row := range rows {
		ratioText := "—"
		if nginx != nil && nginx.OpsPerSec > 0 {
			ratioText = fmt.Sprintf("%.2fx", row.OpsPerSec/nginx.OpsPerSec)
		}
		lines = append(lines, fmt.Sprintf(
			"| %d | %s | %.2f | %s | %.2f | %.2f | %.2f | %.2f | %d | %d |",
			index+1,
			row.Name,
			row.OpsPerSec,
			ratioText,
			row.ThroughputMiBS,
			floatOrZero(row.LatencyP50MS),
			floatOrZero(row.LatencyP95MS),
			floatOrZero(row.LatencyP99MS),
			row.Success,
			row.Errors,
		))
	}
	if nginx != nil {
		if proxy := rowByName(rows, "proxysss"); proxy != nil && nginx.OpsPerSec > 0 {
			lines = append(lines,
				"",
				"## proxysss vs nginx",
				"",
				fmt.Sprintf("- proxysss ops/sec: **%.2f**", proxy.OpsPerSec),
				fmt.Sprintf("- nginx ops/sec: **%.2f**", nginx.OpsPerSec),
				fmt.Sprintf("- ratio: **%.3fx**", proxy.OpsPerSec/nginx.OpsPerSec),
			)
		}
	}
	lines = append(lines, "", "## Raw targets", "")
	for _, row := range rows {
		lines = append(lines, fmt.Sprintf("- `%s` → `%s`", row.Name, row.URL))
	}
	lines = append(lines, "")
	return strings.Join(lines, "\n")
}

func buildGatewayReportHTML(rows []BenchRow, concurrency, duration int) string {
	nginx := rowByName(rows, "nginx")
	ratioBanner := ""
	if proxy := rowByName(rows, "proxysss"); proxy != nil && nginx != nil && nginx.OpsPerSec > 0 {
		ratioBanner = fmt.Sprintf("<p class='hero'>proxysss / nginx = <strong>%.3fx</strong></p>", proxy.OpsPerSec/nginx.OpsPerSec)
	}
	var tableRows strings.Builder
	for index, row := range rows {
		ratioText := "—"
		if nginx != nil && nginx.OpsPerSec > 0 {
			ratioText = fmt.Sprintf("%.2fx", row.OpsPerSec/nginx.OpsPerSec)
		}
		winner := ""
		if index == 0 {
			winner = " class='winner'"
		}
		tableRows.WriteString(fmt.Sprintf(
			"<tr%s><td>%d</td><td><strong>%s</strong></td><td>%.2f</td><td>%s</td><td>%.2f</td><td>%.2f</td><td>%.2f</td><td>%.2f</td><td>%d</td><td>%d</td></tr>",
			winner,
			index+1,
			html.EscapeString(row.Name),
			row.OpsPerSec,
			ratioText,
			row.ThroughputMiBS,
			floatOrZero(row.LatencyP50MS),
			floatOrZero(row.LatencyP95MS),
			floatOrZero(row.LatencyP99MS),
			row.Success,
			row.Errors,
		))
	}
	return fmt.Sprintf(`<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>proxysss gateway benchmark</title>
  <style>
    body { margin: 0; font-family: ui-sans-serif, system-ui, Segoe UI, sans-serif; background: linear-gradient(180deg, #0b1020 0%%, #111827 100%%); color: #e8eefc; }
    main { max-width: 1100px; margin: 0 auto; padding: 2rem 1.25rem 3rem; }
    h1 { margin: 0 0 0.5rem; font-size: 2rem; color: #5eead4; }
    .meta { color: #9fb0d9; margin-bottom: 1rem; }
    .hero { display: inline-block; background: #121a2f; border: 1px solid #24304f; border-radius: 999px; padding: 0.35rem 0.9rem; margin: 0 0 1.25rem; }
    table { width: 100%%; border-collapse: collapse; background: #121a2f; border: 1px solid #24304f; }
    th, td { padding: 0.75rem 0.9rem; border-bottom: 1px solid #24304f; text-align: right; }
    th:first-child, td:first-child, th:nth-child(2), td:nth-child(2) { text-align: left; }
    th { color: #9fb0d9; font-size: 0.85rem; text-transform: uppercase; }
    tr.winner td { background: #163d35; }
  </style>
</head>
<body>
  <main>
    <h1>Gateway throughput benchmark</h1>
    <p class="meta">Generated %s · concurrency %d · duration %ds · static index.html over HTTP/1.1</p>
    %s
    <table>
      <thead>
        <tr><th>#</th><th>Gateway</th><th>ops/sec</th><th>vs nginx</th><th>MiB/s</th><th>p50</th><th>p95</th><th>p99</th><th>success</th><th>errors</th></tr>
      </thead>
      <tbody>%s</tbody>
    </table>
  </main>
</body>
</html>`, time.Now().UTC().Format("2006-01-02 15:04:05 UTC"), concurrency, duration, ratioBanner, tableRows.String())
}

func runWriteGatewayCompare(args []string) error {
	fs := flag.NewFlagSet("write-gateway-compare", flag.ContinueOnError)
	resultsPath := fs.String("results", "", "results json path")
	outDir := fs.String("out-dir", "", "output dir")
	binary := fs.String("binary", "", "proxysss binary path")
	fs.SetOutput(io.Discard)
	if err := fs.Parse(args); err != nil {
		return err
	}
	rows, err := loadResults(*resultsPath)
	if err != nil {
		return err
	}
	version := "unknown"
	if strings.TrimSpace(*binary) != "" {
		output, err := exec.Command(*binary, "--version").CombinedOutput()
		if err == nil {
			version = strings.TrimSpace(string(output))
		}
	}
	sort.Slice(rows, func(i, j int) bool { return rows[i].OpsPerSec > rows[j].OpsPerSec })
	nginx := rowByName(rows, "nginx")
	proxy := rowByName(rows, "proxysss")
	ratio := 0.0
	if proxy != nil && nginx != nil && nginx.OpsPerSec > 0 {
		ratio = proxy.OpsPerSec / nginx.OpsPerSec
	}
	md := []string{
		"# proxysss vs nginx",
		"",
		fmt.Sprintf("- Generated: %s", time.Now().UTC().Format("2006-01-02 15:04:05 UTC")),
		fmt.Sprintf("- proxysss version: %s", version),
		"",
		"## Static benchmark snapshot",
		"",
		fmt.Sprintf("- proxysss ops/sec: `%.2f`", valueOrZero(proxy)),
		fmt.Sprintf("- nginx ops/sec: `%.2f`", valueOrZero(nginx)),
		fmt.Sprintf("- proxysss/nginx ratio: `%.3fx`", ratio),
		"",
		"## Parity reminder",
		"",
		"- Full nginx parity matrix remains CLI-first: run `proxysss config nginx-parity --format yaml` against the current binary.",
		"- Performance work must stay mixed-load safe: do not promote a win that regresses reverse proxy, SSE, WebSocket, TCP, or UDP elsewhere.",
		"",
	}
	htmlBody := fmt.Sprintf(`<!doctype html>
<html lang="en">
<head><meta charset="utf-8" /><title>proxysss vs nginx</title><style>body{font-family:Segoe UI,system-ui,sans-serif;margin:0;background:#0b1020;color:#e8eefc}main{max-width:960px;margin:0 auto;padding:2rem 1.25rem 3rem}h1{color:#5eead4}.pill{display:inline-block;background:#163d35;padding:.4rem .9rem;border-radius:999px}</style></head>
<body><main><h1>proxysss vs nginx</h1><p>Generated %s · %s</p><p class="pill">proxysss/nginx = %.3fx</p><p>Full parity matrix stays CLI-first via <code>proxysss config nginx-parity --format yaml</code>.</p></main></body></html>`,
		time.Now().UTC().Format("2006-01-02 15:04:05 UTC"),
		html.EscapeString(version),
		ratio,
	)
	if err := os.MkdirAll(*outDir, 0o755); err != nil {
		return err
	}
	if err := os.WriteFile(joinPath(*outDir, "nginx-compare.md"), []byte(strings.Join(md, "\n")), 0o644); err != nil {
		return err
	}
	if err := os.WriteFile(joinPath(*outDir, "nginx-compare.html"), []byte(htmlBody), 0o644); err != nil {
		return err
	}
	fmt.Printf("nginx compare markdown: %s\n", joinPath(*outDir, "nginx-compare.md"))
	fmt.Printf("nginx compare html:     %s\n", joinPath(*outDir, "nginx-compare.html"))
	return nil
}

func valueOrZero(row *BenchRow) float64 {
	if row == nil {
		return 0
	}
	return row.OpsPerSec
}

func runCheckSimpleGate(args []string) error {
	fs := flag.NewFlagSet("check-simple-gate", flag.ContinueOnError)
	resultsPath := fs.String("results", "", "results json path")
	baselinePath := fs.String("baseline", "", "baseline json path")
	fs.SetOutput(io.Discard)
	if err := fs.Parse(args); err != nil {
		return err
	}
	rows, err := loadResults(*resultsPath)
	if err != nil {
		return err
	}
	baselineBytes, err := os.ReadFile(*baselinePath)
	if err != nil {
		return err
	}
	var baseline SimpleBaseline
	if err := json.Unmarshal(baselineBytes, &baseline); err != nil {
		return err
	}
	proxy := rowByName(rows, "proxysss")
	nginx := rowByName(rows, "nginx")
	if proxy == nil || nginx == nil {
		return errors.New("results missing proxysss or nginx entry")
	}
	for _, row := range rows {
		if row.Errors > baseline.MaxErrorCount {
			return fmt.Errorf("benchmark gate failed: %s reported %d errors", row.Name, row.Errors)
		}
	}
	if nginx.OpsPerSec <= 0 {
		return errors.New("benchmark gate failed: nginx ops_per_sec is zero")
	}
	ratio := proxy.OpsPerSec / nginx.OpsPerSec
	fmt.Printf("benchmark gate: proxysss=%.2f ops/s nginx=%.2f ops/s ratio=%.3f min=%.3f\n", proxy.OpsPerSec, nginx.OpsPerSec, ratio, baseline.MinProxysssVsNginxOpsRatio)
	if ratio < baseline.MinProxysssVsNginxOpsRatio {
		return fmt.Errorf("benchmark gate failed: proxysss/nginx ops ratio %.3f < required %.3f", ratio, baseline.MinProxysssVsNginxOpsRatio)
	}
	fmt.Println("benchmark gate passed")
	return nil
}

func loadJSONLRows(path string) ([]BenchRow, error) {
	file, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer file.Close()
	var rows []BenchRow
	scanner := bufio.NewScanner(file)
	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		if line == "" {
			continue
		}
		var row BenchRow
		if err := json.Unmarshal([]byte(line), &row); err != nil {
			return nil, err
		}
		rows = append(rows, row)
	}
	return rows, scanner.Err()
}

func loadResults(path string) ([]BenchRow, error) {
	payload, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	var rows []BenchRow
	if err := json.Unmarshal(payload, &rows); err != nil {
		return nil, err
	}
	return rows, nil
}

func rowByName(rows []BenchRow, name string) *BenchRow {
	for i := range rows {
		if rows[i].Name == name {
			return &rows[i]
		}
	}
	return nil
}

func safeRatio(lhs, rhs float64) float64 {
	if rhs <= 0 {
		return 0
	}
	return lhs / rhs
}

func latencyMetrics(proxy, nginx BenchRow) []latencyMetric {
	return []latencyMetric{
		{Name: "p50", Proxy: proxy.LatencyP50MS, Nginx: nginx.LatencyP50MS},
		{Name: "p95", Proxy: proxy.LatencyP95MS, Nginx: nginx.LatencyP95MS},
		{Name: "p99", Proxy: proxy.LatencyP99MS, Nginx: nginx.LatencyP99MS},
	}
}

func latencyRatio(proxy, nginx *float64) float64 {
	if proxy == nil || nginx == nil || *nginx <= 0 {
		return 0
	}
	return *proxy / *nginx
}

func opsImprovementPercent(ratio float64) string {
	if ratio <= 0 || math.IsNaN(ratio) || math.IsInf(ratio, 0) {
		return "-"
	}
	return fmt.Sprintf("%+.2f%%", (ratio-1)*100)
}

func latencyImprovementPercent(proxy, nginx *float64) string {
	ratio := latencyRatio(proxy, nginx)
	if ratio <= 0 || math.IsNaN(ratio) || math.IsInf(ratio, 0) {
		return "-"
	}
	return fmt.Sprintf("%+.2f%%", (1-ratio)*100)
}

func fixedLoadColumns(proxy, nginx BenchRow) (string, string, string) {
	if proxy.TargetOpsPerSec == nil || nginx.TargetOpsPerSec == nil ||
		*proxy.TargetOpsPerSec <= 0 || *nginx.TargetOpsPerSec <= 0 {
		return "-", "-", "-"
	}
	target := math.Min(*proxy.TargetOpsPerSec, *nginx.TargetOpsPerSec)
	return fmt.Sprintf("%.2f", target),
		fmt.Sprintf("%.3fx", proxy.OpsPerSec / *proxy.TargetOpsPerSec),
		fmt.Sprintf("%.3fx", nginx.OpsPerSec / *nginx.TargetOpsPerSec)
}

func ratioGateFailed(ratio, floor float64, strict bool) bool {
	if strict {
		return ratio <= floor
	}
	return ratio < floor
}

func latencyGateFailures(scenario string, proxy, nginx BenchRow, ceiling float64, require, strict bool) []string {
	var failures []string
	for _, metric := range latencyMetrics(proxy, nginx) {
		if metric.Proxy == nil || metric.Nginx == nil || (metric.Nginx != nil && *metric.Nginx <= 0) {
			if require {
				failures = append(failures, fmt.Sprintf("%s %s latency missing proxysss=%v nginx=%v", scenario, metric.Name, metric.Proxy, metric.Nginx))
			}
			continue
		}
		ratio := latencyRatio(metric.Proxy, metric.Nginx)
		failed := ratio > ceiling
		if strict {
			failed = ratio >= ceiling
		}
		if failed {
			failures = append(failures, fmt.Sprintf("%s %s latency ratio=%.3fx %s %.2fx (proxysss=%.3fms nginx=%.3fms)", scenario, metric.Name, ratio, ternary(strict, ">=", ">"), ceiling, *metric.Proxy, *metric.Nginx))
		}
	}
	return failures
}

func floatOrZero(value *float64) float64 {
	if value == nil {
		return 0
	}
	return *value
}

func sortedKeys[V any](items map[string]V) []string {
	keys := make([]string, 0, len(items))
	for key := range items {
		keys = append(keys, key)
	}
	sort.Strings(keys)
	return keys
}

func makeSet(raw string) map[string]bool {
	set := map[string]bool{}
	for _, item := range strings.Fields(raw) {
		set[item] = true
	}
	return set
}

func sortedSetMembers(set map[string]bool) []string {
	members := make([]string, 0, len(set))
	for key := range set {
		members = append(members, key)
	}
	sort.Strings(members)
	return members
}

func ternary[T any](condition bool, whenTrue, whenFalse T) T {
	if condition {
		return whenTrue
	}
	return whenFalse
}

func joinPath(root, file string) string {
	if strings.HasSuffix(root, "/") || strings.HasSuffix(root, "\\") {
		return root + file
	}
	return root + string(os.PathSeparator) + file
}
