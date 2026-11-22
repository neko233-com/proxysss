// Fast same-host HTTP/1.1 keep-alive benchmark used by the 2C2G quick check.
package main

import (
	"context"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"io"
	"net/http"
	"os"
	"sort"
	"strings"
	"sync"
	"sync/atomic"
	"time"
)

type result struct {
	Label        string  `json:"label"`
	URL          string  `json:"url"`
	Workers      int     `json:"workers"`
	WarmupSecs   int     `json:"warmup_secs"`
	DurationSecs int     `json:"duration_secs"`
	Success      int64   `json:"success"`
	Errors       int64   `json:"errors"`
	RPS          float64 `json:"rps"`
	MiBPerSec    float64 `json:"mib_per_sec"`
	P50Ms        float64 `json:"p50_ms"`
	P95Ms        float64 `json:"p95_ms"`
	P99Ms        float64 `json:"p99_ms"`
	AvgMs        float64 `json:"avg_ms"`
	ErrorSample  string  `json:"error_sample,omitempty"`
}

func main() {
	var (
		url          = flag.String("url", "", "target URL")
		label        = flag.String("label", "target", "result label")
		workers      = flag.Int("workers", 64, "parallel keep-alive workers")
		warmupSecs   = flag.Int("warmup", 1, "warmup seconds")
		durationSecs = flag.Int("duration", 3, "measurement seconds")
		expected     = flag.Int("expected-bytes", 0, "required response body bytes; 0 disables")
		out          = flag.String("out", "", "optional JSON output path")
	)
	flag.Parse()
	if *url == "" {
		fmt.Fprintln(os.Stderr, "missing -url")
		os.Exit(2)
	}
	if *workers < 1 || *durationSecs < 1 || *warmupSecs < 0 {
		fmt.Fprintln(os.Stderr, "invalid workers/warmup/duration")
		os.Exit(2)
	}

	clients := make([]*http.Client, *workers)
	for i := range clients {
		tr := &http.Transport{
			Proxy:               nil,
			MaxIdleConns:        1,
			MaxIdleConnsPerHost: 1,
			MaxConnsPerHost:     1,
			IdleConnTimeout:     30 * time.Second,
			DisableCompression:  true,
			ForceAttemptHTTP2:   false,
		}
		clients[i] = &http.Client{Transport: tr, Timeout: 15 * time.Second}
	}

	do := func(client *http.Client, ctx context.Context) (int64, error) {
		req, err := http.NewRequestWithContext(ctx, http.MethodGet, *url, nil)
		if err != nil {
			return 0, err
		}
		resp, err := client.Do(req)
		if err != nil {
			return 0, err
		}
		n, readErr := io.Copy(io.Discard, resp.Body)
		closeErr := resp.Body.Close()
		if resp.StatusCode != http.StatusOK {
			return n, fmt.Errorf("status %d", resp.StatusCode)
		}
		if readErr != nil {
			return n, readErr
		}
		if closeErr != nil {
			return n, closeErr
		}
		if *expected > 0 && n != int64(*expected) {
			return n, fmt.Errorf("body bytes %d, want %d", n, *expected)
		}
		return n, nil
	}

	run := func(duration time.Duration, record bool) (int64, int64, int64, []time.Duration, []string) {
		var ok, errs, bytes atomic.Int64
		var mu sync.Mutex
		var samples []string
		var wg sync.WaitGroup
		phaseEnd := time.Now().Add(duration)
		ctx, cancel := context.WithCancel(context.Background())
		timer := time.AfterFunc(duration, cancel)
		start := make(chan struct{})
		local := make([][]time.Duration, *workers)
		for i := 0; i < *workers; i++ {
			wg.Add(1)
			go func(worker int) {
				defer wg.Done()
				<-start
				if !record {
					time.Sleep(time.Duration(worker) * 2 * time.Millisecond)
				}
				var latencies []time.Duration
				for time.Now().Before(phaseEnd) {
					began := time.Now()
					n, err := do(clients[worker], ctx)
					elapsed := time.Since(began)
					if err != nil {
						if errors.Is(err, context.Canceled) || errors.Is(err, context.DeadlineExceeded) {
							break
						}
						errs.Add(1)
						mu.Lock()
						if len(samples) < 5 {
							samples = append(samples, err.Error())
						}
						mu.Unlock()
						continue
					}
					if ctx.Err() != nil {
						break
					}
					ok.Add(1)
					bytes.Add(n)
					if record {
						latencies = append(latencies, elapsed)
					}
				}
				local[worker] = latencies
			}(i)
		}
		close(start)
		wg.Wait()
		timer.Stop()
		cancel()
		if !record {
			return ok.Load(), errs.Load(), bytes.Load(), nil, samples
		}
		total := 0
		for _, part := range local {
			total += len(part)
		}
		latencies := make([]time.Duration, 0, total)
		for _, part := range local {
			latencies = append(latencies, part...)
		}
		return ok.Load(), errs.Load(), bytes.Load(), latencies, samples
	}

	prewarm := func() bool {
		pending := make([]int, *workers)
		for i := range pending {
			pending[i] = i
		}
		deadline := time.Now().Add(30 * time.Second)
		for len(pending) > 0 && time.Now().Before(deadline) {
			next := make([]int, 0, len(pending))
			var lastErr error
			for _, worker := range pending {
				if !time.Now().Before(deadline) {
					next = append(next, worker)
					continue
				}
				ctx, cancel := context.WithTimeout(context.Background(), 1500*time.Millisecond)
				_, err := do(clients[worker], ctx)
				cancel()
				if err != nil {
					lastErr = err
					next = append(next, worker)
				}
			}
			if len(next) == 0 {
				return true
			}
			fmt.Fprintf(os.Stderr, "prewarm: %d/%d connections pending: %v\n", len(next), *workers, lastErr)
			pending = next
			time.Sleep(500 * time.Millisecond)
		}
		if len(pending) > 0 {
			fmt.Fprintf(os.Stderr, "prewarm: %d/%d connections not established\n", len(pending), *workers)
			return false
		}
		return true
	}

	if !prewarm() {
		os.Exit(2)
	}
	if *warmupSecs > 0 {
		run(time.Duration(*warmupSecs)*time.Second, false)
	}
	begin := time.Now()
	ok, errs, bytes, latencies, samples := run(time.Duration(*durationSecs)*time.Second, true)
	elapsed := time.Since(begin).Seconds()
	if elapsed <= 0 {
		elapsed = 1
	}
	measured := float64(*durationSecs)
	if elapsed < measured {
		measured = elapsed
	}
	sort.Slice(latencies, func(i, j int) bool { return latencies[i] < latencies[j] })
	pct := func(p float64) float64 {
		if len(latencies) == 0 {
			return 0
		}
		idx := int(float64(len(latencies)-1) * p)
		return float64(latencies[idx].Microseconds()) / 1000
	}
	var sum time.Duration
	for _, d := range latencies {
		sum += d
	}
	avg := 0.0
	if len(latencies) > 0 {
		avg = float64(sum.Microseconds()) / 1000 / float64(len(latencies))
	}
	r := result{
		Label:        *label,
		URL:          *url,
		Workers:      *workers,
		WarmupSecs:   *warmupSecs,
		DurationSecs: *durationSecs,
		Success:      ok,
		Errors:       errs,
		RPS:          float64(ok) / measured,
		MiBPerSec:    float64(bytes) / (1024 * 1024) / measured,
		P50Ms:        pct(0.50),
		P95Ms:        pct(0.95),
		P99Ms:        pct(0.99),
		AvgMs:        avg,
		ErrorSample:  strings.Join(samples, " | "),
	}
	fmt.Printf("%s: rps=%.0f mib/s=%.1f p50=%.3fms p95=%.3fms p99=%.3fms success=%d errors=%d\n",
		r.Label, r.RPS, r.MiBPerSec, r.P50Ms, r.P95Ms, r.P99Ms, r.Success, r.Errors)
	if *out != "" {
		b, _ := json.MarshalIndent(r, "", "  ")
		if err := os.WriteFile(*out, b, 0o644); err != nil {
			fmt.Fprintln(os.Stderr, err)
			os.Exit(1)
		}
	}
	if r.Success == 0 || float64(r.Errors) > float64(r.Success+r.Errors)*0.01 {
		os.Exit(1)
	}
}
