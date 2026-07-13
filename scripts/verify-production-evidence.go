// Verifies the versioned Linux benchmark evidence required for a release tag.
//
// The manifest is deliberately a compact, reviewable index of immutable raw
// artifacts. It records the measured values as well as their artifact hashes,
// so a release cannot be approved by setting a collection of "passed" flags.
package main

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"math"
	"os"
	"sort"
	"strings"
)

const evidenceSchemaVersion = 2

var roleIsolatedScenarios = []string{
	"static-small", "static-large", "cdn-hot-update", "https-static-small",
	"reverse-proxy", "generic-sse", "websocket-long-connection",
	"game-long-connection", "tcp-stream", "udp-stream",
}

type evidenceManifest struct {
	SchemaVersion int           `json:"schema_version"`
	Tag           string        `json:"tag"`
	Commit        string        `json:"commit"`
	Runs          []evidenceRun `json:"runs"`
}

type evidenceRun struct {
	Kind                string              `json:"kind"`
	Scale               int                 `json:"scale"`
	RoleIsolationProven bool                `json:"role_isolation_proven"`
	RoleMachineIDHashes roleMachineIDHashes `json:"role_machine_id_hashes"`
	Workload            workloadEvidence    `json:"workload"`
	Memory              memoryEvidence      `json:"memory"`
	Scenarios           []scenarioEvidence  `json:"scenarios"`
	Capacity            capacityEvidence    `json:"capacity"`
	Artifacts           []evidenceArtifact  `json:"artifacts"`
}

type roleMachineIDHashes struct {
	Client  string `json:"client"`
	Gateway string `json:"gateway"`
	Backend string `json:"backend"`
}

type workloadEvidence struct {
	ActiveConnections   int `json:"active_connections"`
	CapacityConnections int `json:"capacity_connections"`
	Repetitions         int `json:"repetitions"`
}

type memoryEvidence struct {
	Proxysss        memoryMetrics `json:"proxysss"`
	Nginx           memoryMetrics `json:"nginx"`
	NoRunawayGrowth bool          `json:"no_runaway_growth"`
}

type memoryMetrics struct {
	CurrentBytes       uint64 `json:"current_bytes"`
	PeakBytes          uint64 `json:"peak_bytes"`
	BytesPerConnection uint64 `json:"bytes_per_connection"`
}

type scenarioEvidence struct {
	Name     string     `json:"name"`
	Proxysss runMetrics `json:"proxysss"`
	Nginx    runMetrics `json:"nginx"`
}

type runMetrics struct {
	OpsPerSec float64 `json:"ops_per_sec"`
	P50MS     float64 `json:"p50_ms"`
	P95MS     float64 `json:"p95_ms"`
	P99MS     float64 `json:"p99_ms"`
	Errors    int     `json:"errors"`
}

type capacityEvidence struct {
	Proxysss capacityMetrics `json:"proxysss"`
	Nginx    capacityMetrics `json:"nginx"`
}

type capacityMetrics struct {
	Opened         int     `json:"opened"`
	Failed         int     `json:"failed"`
	OpenRatePerSec float64 `json:"open_rate_per_sec"`
	HandshakeP50MS float64 `json:"handshake_p50_ms"`
	HandshakeP95MS float64 `json:"handshake_p95_ms"`
	HandshakeP99MS float64 `json:"handshake_p99_ms"`
}

type evidenceArtifact struct {
	Name   string `json:"name"`
	SHA256 string `json:"sha256"`
	URI    string `json:"uri"`
}

func main() {
	if err := run(os.Args[1:]); err != nil {
		fmt.Fprintln(os.Stderr, "production evidence verification failed:", err)
		os.Exit(1)
	}
}

func run(args []string) error {
	fs := flag.NewFlagSet("verify-production-evidence", flag.ContinueOnError)
	fs.SetOutput(os.Stderr)
	manifestPath := fs.String("manifest", "", "versioned performance evidence manifest")
	tag := fs.String("tag", "", "release tag")
	commit := fs.String("commit", "", "tag commit SHA")
	if err := fs.Parse(args); err != nil {
		return err
	}
	if *manifestPath == "" || *tag == "" || *commit == "" {
		return errors.New("--manifest, --tag, and --commit are required")
	}
	payload, err := os.ReadFile(*manifestPath)
	if err != nil {
		return err
	}
	var manifest evidenceManifest
	if err := json.Unmarshal(payload, &manifest); err != nil {
		return fmt.Errorf("parse manifest: %w", err)
	}
	if err := validateEvidenceManifest(manifest, *tag, *commit); err != nil {
		return err
	}
	fingerprint := sha256.Sum256(payload)
	fmt.Printf("production evidence verified: tag=%s manifest_sha256=%s\n", *tag, hex.EncodeToString(fingerprint[:]))
	return nil
}

func validateEvidenceManifest(manifest evidenceManifest, tag, commit string) error {
	if manifest.SchemaVersion != evidenceSchemaVersion {
		return fmt.Errorf("schema_version=%d, expected %d", manifest.SchemaVersion, evidenceSchemaVersion)
	}
	if manifest.Tag != tag {
		return fmt.Errorf("manifest tag %q does not match release tag %q", manifest.Tag, tag)
	}
	if !strings.EqualFold(manifest.Commit, commit) {
		return fmt.Errorf("manifest commit %q does not match tag commit %q", manifest.Commit, commit)
	}
	if len(manifest.Runs) == 0 {
		return errors.New("manifest has no benchmark runs")
	}

	requiredKinds := []string{"role-isolated-all-scenarios", "cross-host-wss"}
	requiredScales := []int{1, 2, 4}
	byKey := make(map[string]evidenceRun, len(manifest.Runs))
	for _, run := range manifest.Runs {
		key := fmt.Sprintf("%s/%d", run.Kind, run.Scale)
		if _, exists := byKey[key]; exists {
			return fmt.Errorf("duplicate benchmark evidence %s", key)
		}
		byKey[key] = run
		if err := validateEvidenceRun(run); err != nil {
			return fmt.Errorf("%s: %w", key, err)
		}
	}
	for _, kind := range requiredKinds {
		for _, scale := range requiredScales {
			key := fmt.Sprintf("%s/%d", kind, scale)
			if _, exists := byKey[key]; !exists {
				return fmt.Errorf("missing required benchmark evidence %s", key)
			}
		}
	}
	return nil
}

func validateEvidenceRun(run evidenceRun) error {
	if run.Kind != "role-isolated-all-scenarios" && run.Kind != "cross-host-wss" {
		return fmt.Errorf("unsupported kind %q", run.Kind)
	}
	if run.Scale <= 0 {
		return errors.New("scale must be positive")
	}
	if !run.RoleIsolationProven {
		return errors.New("role isolation evidence is required")
	}
	roles := []string{run.RoleMachineIDHashes.Client, run.RoleMachineIDHashes.Gateway, run.RoleMachineIDHashes.Backend}
	for _, hash := range roles {
		if !validSHA256(hash) {
			return fmt.Errorf("role machine-id hash %q is not a SHA-256 digest", hash)
		}
	}
	if run.Kind == "cross-host-wss" && (roles[0] == roles[1] || roles[0] == roles[2] || roles[1] == roles[2]) {
		return errors.New("cross-host client, gateway, and backend machine-id hashes must be distinct")
	}
	if run.Workload.ActiveConnections <= 0 || run.Workload.CapacityConnections < 1_000 || run.Workload.CapacityConnections > 50_000 {
		return errors.New("workload needs active connections and a 1k-50k capacity envelope")
	}
	if run.Workload.Repetitions < 4 || run.Workload.Repetitions%2 != 0 {
		return errors.New("workload repetitions must be even and at least four")
	}
	if err := validateMemory(run.Memory); err != nil {
		return err
	}
	if !run.Memory.NoRunawayGrowth {
		return errors.New("memory current/peak, per-connection cost, and no-runaway-growth evidence are required")
	}
	if err := validateScenarios(run.Kind, run.Scenarios); err != nil {
		return err
	}
	if err := validateCapacity(run.Capacity, run.Workload.CapacityConnections); err != nil {
		return err
	}
	if len(run.Artifacts) < 3 {
		return errors.New("at least saturation, equal-load, and capacity raw artifacts are required")
	}
	seen := map[string]bool{}
	for _, artifact := range run.Artifacts {
		if artifact.Name == "" || artifact.URI == "" || !validSHA256(artifact.SHA256) {
			return errors.New("every raw artifact needs name, URI, and SHA-256")
		}
		seen[artifact.Name] = true
	}
	missing := []string{}
	for _, name := range []string{"saturation", "equal-load", "capacity"} {
		if !seen[name] {
			missing = append(missing, name)
		}
	}
	if len(missing) > 0 {
		sort.Strings(missing)
		return fmt.Errorf("missing raw artifacts: %s", strings.Join(missing, ", "))
	}
	return nil
}

func validateMemory(memory memoryEvidence) error {
	for _, item := range []struct {
		name  string
		proxy uint64
		nginx uint64
	}{
		{"current_bytes", memory.Proxysss.CurrentBytes, memory.Nginx.CurrentBytes},
		{"peak_bytes", memory.Proxysss.PeakBytes, memory.Nginx.PeakBytes},
		{"bytes_per_connection", memory.Proxysss.BytesPerConnection, memory.Nginx.BytesPerConnection},
	} {
		if item.proxy == 0 || item.nginx == 0 {
			return fmt.Errorf("memory %s needs values for both gateways", item.name)
		}
		// Cross multiplication avoids float rounding and makes the declared
		// 2x production envelope exact for all uint64 values.
		if item.proxy > item.nginx && item.proxy-item.nginx > item.nginx {
			return fmt.Errorf("memory %s exceeds the 2x nginx envelope (proxysss=%d nginx=%d)", item.name, item.proxy, item.nginx)
		}
	}
	return nil
}

func validateScenarios(kind string, scenarios []scenarioEvidence) error {
	required := map[string]bool{"websocket-long-connection": true}
	if kind == "role-isolated-all-scenarios" {
		required = make(map[string]bool, len(roleIsolatedScenarios))
		for _, scenario := range roleIsolatedScenarios {
			required[scenario] = true
		}
	}
	seen := make(map[string]bool, len(scenarios))
	for _, scenario := range scenarios {
		if !required[scenario.Name] {
			return fmt.Errorf("unexpected scenario %q", scenario.Name)
		}
		if seen[scenario.Name] {
			return fmt.Errorf("duplicate scenario %q", scenario.Name)
		}
		seen[scenario.Name] = true
		if err := validateScenarioMetrics(scenario); err != nil {
			return fmt.Errorf("scenario %s: %w", scenario.Name, err)
		}
	}
	for scenario := range required {
		if !seen[scenario] {
			return fmt.Errorf("missing required scenario %q", scenario)
		}
	}
	return nil
}

func validateScenarioMetrics(scenario scenarioEvidence) error {
	if scenario.Proxysss.Errors != 0 || scenario.Nginx.Errors != 0 {
		return fmt.Errorf("zero errors required (proxysss=%d nginx=%d)", scenario.Proxysss.Errors, scenario.Nginx.Errors)
	}
	for _, item := range []struct {
		name         string
		proxy, nginx float64
		lowerBetter  bool
	}{
		{"ops_per_sec", scenario.Proxysss.OpsPerSec, scenario.Nginx.OpsPerSec, false},
		{"p50_ms", scenario.Proxysss.P50MS, scenario.Nginx.P50MS, true},
		{"p95_ms", scenario.Proxysss.P95MS, scenario.Nginx.P95MS, true},
		{"p99_ms", scenario.Proxysss.P99MS, scenario.Nginx.P99MS, true},
	} {
		if !positiveFinite(item.proxy) || !positiveFinite(item.nginx) {
			return fmt.Errorf("%s needs positive finite values", item.name)
		}
		if item.lowerBetter && item.proxy >= item.nginx {
			return fmt.Errorf("%s must be strictly lower (proxysss=%g nginx=%g)", item.name, item.proxy, item.nginx)
		}
		if !item.lowerBetter && item.proxy <= item.nginx {
			return fmt.Errorf("%s must be strictly higher (proxysss=%g nginx=%g)", item.name, item.proxy, item.nginx)
		}
	}
	return nil
}

func validateCapacity(capacity capacityEvidence, requested int) error {
	if capacity.Proxysss.Opened != requested || capacity.Nginx.Opened != requested || capacity.Proxysss.Failed != 0 || capacity.Nginx.Failed != 0 {
		return errors.New("capacity requires both gateways to open every requested connection with zero failures")
	}
	return validateScenarioMetrics(scenarioEvidence{
		Name:     "capacity",
		Proxysss: runMetrics{OpsPerSec: capacity.Proxysss.OpenRatePerSec, P50MS: capacity.Proxysss.HandshakeP50MS, P95MS: capacity.Proxysss.HandshakeP95MS, P99MS: capacity.Proxysss.HandshakeP99MS},
		Nginx:    runMetrics{OpsPerSec: capacity.Nginx.OpenRatePerSec, P50MS: capacity.Nginx.HandshakeP50MS, P95MS: capacity.Nginx.HandshakeP95MS, P99MS: capacity.Nginx.HandshakeP99MS},
	})
}

func positiveFinite(value float64) bool {
	return value > 0 && !math.IsInf(value, 0) && !math.IsNaN(value)
}

func validSHA256(value string) bool {
	if len(value) != sha256.Size*2 {
		return false
	}
	_, err := hex.DecodeString(value)
	return err == nil
}
