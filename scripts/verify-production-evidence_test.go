package main

import (
	"strings"
	"testing"
)

const testHashA = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
const testHashB = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
const testHashC = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"

func winningScenario(name string) scenarioEvidence {
	return scenarioEvidence{
		Name:     name,
		Proxysss: runMetrics{OpsPerSec: 200, P50MS: 1, P95MS: 2, P99MS: 3},
		Nginx:    runMetrics{OpsPerSec: 100, P50MS: 2, P95MS: 3, P99MS: 4},
	}
}

func validEvidenceRun(kind string, scale int) evidenceRun {
	scenarios := []scenarioEvidence{winningScenario("websocket-long-connection")}
	if kind == "role-isolated-all-scenarios" {
		scenarios = make([]scenarioEvidence, 0, len(roleIsolatedScenarios))
		for _, name := range roleIsolatedScenarios {
			scenarios = append(scenarios, winningScenario(name))
		}
	}
	return evidenceRun{
		Kind:                kind,
		Scale:               scale,
		RoleIsolationProven: true,
		RoleMachineIDHashes: roleMachineIDHashes{Client: testHashA, Gateway: testHashB, Backend: testHashC},
		Workload:            workloadEvidence{ActiveConnections: 4096 * scale, CapacityConnections: 20000, Repetitions: 4},
		Memory: memoryEvidence{
			Proxysss:        memoryMetrics{CurrentBytes: 2, PeakBytes: 4, BytesPerConnection: 2},
			Nginx:           memoryMetrics{CurrentBytes: 1, PeakBytes: 2, BytesPerConnection: 1},
			NoRunawayGrowth: true,
		},
		Scenarios: scenarios,
		Capacity: capacityEvidence{
			Proxysss: capacityMetrics{Opened: 20000, OpenRatePerSec: 200, HandshakeP50MS: 1, HandshakeP95MS: 2, HandshakeP99MS: 3},
			Nginx:    capacityMetrics{Opened: 20000, OpenRatePerSec: 100, HandshakeP50MS: 2, HandshakeP95MS: 3, HandshakeP99MS: 4},
		},
		Artifacts: []evidenceArtifact{
			{Name: "saturation", SHA256: testHashA, URI: "artifact://saturation"},
			{Name: "equal-load", SHA256: testHashB, URI: "artifact://equal-load"},
			{Name: "capacity", SHA256: testHashC, URI: "artifact://capacity"},
		},
	}
}

func validManifest() evidenceManifest {
	runs := make([]evidenceRun, 0, 6)
	for _, kind := range []string{"role-isolated-all-scenarios", "cross-host-wss"} {
		for _, scale := range []int{1, 2, 4} {
			runs = append(runs, validEvidenceRun(kind, scale))
		}
	}
	return evidenceManifest{SchemaVersion: evidenceSchemaVersion, Tag: "v9.9.9", Commit: "deadbeef", Runs: runs}
}

func TestEvidenceManifestRequiresEveryKindAndScale(t *testing.T) {
	manifest := validManifest()
	manifest.Runs = manifest.Runs[:5]
	if err := validateEvidenceManifest(manifest, "v9.9.9", "deadbeef"); err == nil || !strings.Contains(err.Error(), "missing required benchmark evidence") {
		t.Fatalf("expected missing kind/scale failure, got %v", err)
	}
}

func TestEvidenceManifestRejectsSharedRoleHost(t *testing.T) {
	manifest := validManifest()
	manifest.Runs[3].RoleMachineIDHashes.Gateway = testHashA
	if err := validateEvidenceManifest(manifest, "v9.9.9", "deadbeef"); err == nil || !strings.Contains(err.Error(), "must be distinct") {
		t.Fatalf("expected distinct-role failure, got %v", err)
	}
}

func TestEvidenceManifestRejectsARegressionInAnyScenarioMetric(t *testing.T) {
	manifest := validManifest()
	manifest.Runs[0].Scenarios[0].Proxysss.P99MS = manifest.Runs[0].Scenarios[0].Nginx.P99MS
	if err := validateEvidenceManifest(manifest, "v9.9.9", "deadbeef"); err == nil || !strings.Contains(err.Error(), "p99_ms must be strictly lower") {
		t.Fatalf("expected p99 regression failure, got %v", err)
	}
}

func TestEvidenceManifestRejectsUnrealisticCapacityEnvelope(t *testing.T) {
	manifest := validManifest()
	manifest.Runs[0].Workload.CapacityConnections = 100000
	if err := validateEvidenceManifest(manifest, "v9.9.9", "deadbeef"); err == nil || !strings.Contains(err.Error(), "1k-50k capacity envelope") {
		t.Fatalf("expected capacity envelope failure, got %v", err)
	}
}

func TestEvidenceManifestRejectsMemoryOverTwiceNginx(t *testing.T) {
	manifest := validManifest()
	manifest.Runs[0].Memory.Proxysss.PeakBytes = 5
	if err := validateEvidenceManifest(manifest, "v9.9.9", "deadbeef"); err == nil || !strings.Contains(err.Error(), "exceeds the 2x nginx envelope") {
		t.Fatalf("expected memory-envelope failure, got %v", err)
	}
}
