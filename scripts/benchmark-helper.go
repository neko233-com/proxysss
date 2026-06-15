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
	"net/http"
	"os"
	"os/exec"
	"regexp"
	"sort"
	"strconv"
	"strings"
	"time"
)

type BenchRow struct {
	Scenario       string   `json:"scenario,omitempty"`
	Gateway        string   `json:"gateway,omitempty"`
	Name           string   `json:"name"`
	Protocol       string   `json:"protocol,omitempty"`
	Target         string   `json:"target,omitempty"`
	URL            string   `json:"url,omitempty"`
	Concurrency    int      `json:"concurrency"`
	DurationSecs   int      `json:"duration_secs"`
	Success        int      `json:"success"`
	Errors         int      `json:"errors"`
	OpsPerSec      float64  `json:"ops_per_sec"`
	ThroughputMiBS float64  `json:"throughput_mib_s"`
	LatencyP50MS   *float64 `json:"latency_p50_ms"`
	LatencyP95MS   *float64 `json:"latency_p95_ms"`
	LatencyP99MS   *float64 `json:"latency_p99_ms"`
}

type SimpleBaseline struct {
	MaxErrorCount              int     `json:"max_error_count"`
	MinProxysssVsNginxOpsRatio float64 `json:"min_proxysss_vs_nginx_ops_ratio"`
}

var (
	successPattern    = regexp.MustCompile(`success\s+:\s+(\d+)`)
	errorPattern      = regexp.MustCompile(`errors\s+:\s+(\d+)`)
	opsPattern        = regexp.MustCompile(`ops/sec\s+:\s+([\d.]+)`)
	throughputPattern = regexp.MustCompile(`throughput\s+:\s+([\d.]+)\s+MiB/s`)
	p50Pattern        = regexp.MustCompile(`latency p50\s+:\s+([\d.]+)\s+ms`)
	p95Pattern        = regexp.MustCompile(`latency p95\s+:\s+([\d.]+)\s+ms`)
	p99Pattern        = regexp.MustCompile(`latency p99\s+:\s+([\d.]+)\s+ms`)
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
	default:
		usage()
		err = fmt.Errorf("unknown subcommand %q", os.Args[1])
	}
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
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
	fs.SetOutput(io.Discard)
	if err := fs.Parse(args); err != nil {
		return err
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
			for idx := 0; idx < 8; idx++ {
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
			_, _ = io.WriteString(w, "data: [DONE]\n\n")
			flusher.Flush()
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

func runQuickGate(args []string) error {
	fs := flag.NewFlagSet("quick-gate", flag.ContinueOnError)
	minRatio := fs.Float64("min-ratio", 0.97, "minimum acceptable ratio")
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
		if proxy.Errors > 0 || nginx.Errors > 0 {
			failures = append(failures, fmt.Sprintf("%s errors proxysss=%d nginx=%d", scenario, proxy.Errors, nginx.Errors))
		} else if ratio < *minRatio {
			failures = append(failures, fmt.Sprintf("%s ratio=%.3fx < %.2fx", scenario, ratio, *minRatio))
		}
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
	aggregateRatio := fs.Float64("aggregate-ratio", 0.97, "aggregate mixed ratio")
	mixedMatrix := fs.Bool("mixed-matrix", true, "whether results came from mixed matrix")
	cpuCores := fs.String("cpu-cores", "", "detected cores")
	httpConcurrency := fs.String("http-concurrency", "", "http concurrency")
	httpsConcurrency := fs.String("https-concurrency", "", "https concurrency")
	staticLargeConcurrency := fs.String("static-large-concurrency", "", "static large concurrency")
	sseConcurrency := fs.String("sse-concurrency", "", "sse concurrency")
	streamConnections := fs.String("stream-connections", "", "stream connections")
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
	scenarios := sortedKeys(byScenario)
	for _, scenario := range scenarios {
		gateways := byScenario[scenario]
		proxy := gateways["proxysss"]
		nginx := gateways["nginx"]
		protocol := proxy.Protocol
		if protocol == "" {
			protocol = nginx.Protocol
		}
		switch protocol {
		case "udp":
			if proxy.Errors > nginx.Errors+2 {
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

	var lines []string
	lines = append(lines,
		"# proxysss all-scenarios benchmark",
		"",
		fmt.Sprintf("- Matrix mode: `%s`", ternary(*mixedMatrix, "mixed concurrent", "serial diagnostic")),
		fmt.Sprintf("- Detected CPU cores: `%s`", *cpuCores),
		fmt.Sprintf("- Auto concurrency: HTTP `%s`, HTTPS `%s`, static-large `%s`, SSE `%s`, TCP/UDP/WebSocket `%s`", *httpConcurrency, *httpsConcurrency, *staticLargeConcurrency, *sseConcurrency, *streamConnections),
		fmt.Sprintf("- Non-critical minimum proxysss/nginx ops ratio: `%.2f` except diagnostic scenarios `%s`", *minRatio, strings.Join(sortedSetMembers(diagnosticSet), ", ")),
		fmt.Sprintf("- SSE stream error tolerance: `proxysss <= nginx + %d`", *sseTolerance),
		fmt.Sprintf("- WebSocket reconnect/error tolerance: `proxysss <= nginx + %d`", *websocketTolerance),
		fmt.Sprintf("- Critical long-connection fair ratio gate: `%.2f` for `%s`", *criticalRatio, strings.Join(sortedSetMembers(criticalSet), ", ")),
		fmt.Sprintf("- Aggregate mixed-load fair ratio gate: `%.2f`", *aggregateRatio),
		fmt.Sprintf("- Result file: `%s`", *resultsPath),
		"",
		"| Scenario | proxysss ops/s | nginx ops/s | Ratio | proxysss p95 ms | nginx p95 ms | Errors |",
		"| --- | ---: | ---: | ---: | ---: | ---: | ---: |",
	)

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
		lines = append(lines, fmt.Sprintf(
			"| %s | %.2f | %.2f | %.3fx | %.3f | %.3f | %d |",
			scenario,
			proxy.OpsPerSec,
			nginx.OpsPerSec,
			ratio,
			floatOrZero(proxy.LatencyP95MS),
			floatOrZero(nginx.LatencyP95MS),
			errCount,
		))
	}
	aggregate := safeRatio(proxyTotalOps, nginxTotalOps)
	lines = append(lines,
		"",
		fmt.Sprintf("- Aggregate proxysss ops/s: `%.2f`", proxyTotalOps),
		fmt.Sprintf("- Aggregate nginx ops/s: `%.2f`", nginxTotalOps),
		fmt.Sprintf("- Aggregate proxysss/nginx ratio: `%.3fx`", aggregate),
		"",
	)
	if err := os.WriteFile(*mdPath, []byte(strings.Join(lines, "\n")), 0o644); err != nil {
		return err
	}
	if err := os.WriteFile(*htmlPath, []byte(buildAllScenariosHTML(byScenario, ratioPairs, aggregate)), 0o644); err != nil {
		return err
	}

	var softFailures []string
	for _, item := range ratioPairs {
		if !diagnosticSet[item.Scenario] && item.Ratio < *minRatio {
			softFailures = append(softFailures, fmt.Sprintf("%s ratio=%.3f", item.Scenario, item.Ratio))
		}
	}
	var criticalFailures []string
	for _, item := range ratioPairs {
		if criticalSet[item.Scenario] && item.Ratio < *criticalRatio {
			criticalFailures = append(criticalFailures, fmt.Sprintf("%s ratio=%.3f < %.2f", item.Scenario, item.Ratio, *criticalRatio))
		}
	}
	if len(errorFailures) > 0 {
		return fmt.Errorf("benchmark errors: %s", strings.Join(errorFailures, "; "))
	}
	if len(criticalFailures) > 0 {
		return fmt.Errorf("critical fair benchmark ratio gate failed: %s", strings.Join(criticalFailures, "; "))
	}
	if *mixedMatrix && aggregate < *aggregateRatio {
		return fmt.Errorf("aggregate mixed fair benchmark ratio gate failed: %.3f < %.2f", aggregate, *aggregateRatio)
	}
	if len(softFailures) > 0 {
		return fmt.Errorf("benchmark ratio gate failed: %s", strings.Join(softFailures, "; "))
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
        <tr><th>Scenario</th><th>proxysss ops/s</th><th>nginx ops/s</th><th>ratio</th></tr>
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
