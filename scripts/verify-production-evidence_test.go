package main

import (
	"strings"
	"testing"
)

const testHashA = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
const testHashB = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
const testHashC = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"

func validEvidenceRun(kind string, scale int) evidenceRun {
	return evidenceRun{
		Kind:                            kind,
		Scale:                           scale,
		StrictSuperiority:               true,
		ZeroErrors:                      true,
		SaturationOpsStrictlyWon:        true,
		EqualLoadPercentilesStrictlyWon: true,
		CapacityStrictlyWon:             true,
		RoleIsolationProven:             true,
		RoleMachineIDHashes:             roleMachineIDHashes{Client: testHashA, Gateway: testHashB, Backend: testHashC},
		Memory:                          memoryEvidence{CurrentAndPeakRecorded: true, PerConnectionRecorded: true, NoRunawayGrowth: true},
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

func TestEvidenceManifestRejectsUnprovenWin(t *testing.T) {
	manifest := validManifest()
	manifest.Runs[0].EqualLoadPercentilesStrictlyWon = false
	if err := validateEvidenceManifest(manifest, "v9.9.9", "deadbeef"); err == nil || !strings.Contains(err.Error(), "all required") {
		t.Fatalf("expected strict-win failure, got %v", err)
	}
}
