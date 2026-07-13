// Verifies the versioned Linux benchmark evidence required for a release tag.
//
// The manifest deliberately contains hashes and locations of raw benchmark
// artifacts rather than benchmark payloads themselves: .benchmark is local
// operator evidence and remains ignored by git.
package main

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"os"
	"sort"
	"strings"
)

const evidenceSchemaVersion = 1

type evidenceManifest struct {
	SchemaVersion int           `json:"schema_version"`
	Tag           string        `json:"tag"`
	Commit        string        `json:"commit"`
	Runs          []evidenceRun `json:"runs"`
}

type evidenceRun struct {
	Kind                            string              `json:"kind"`
	Scale                           int                 `json:"scale"`
	StrictSuperiority               bool                `json:"strict_superiority"`
	ZeroErrors                      bool                `json:"zero_errors"`
	SaturationOpsStrictlyWon        bool                `json:"saturation_ops_strictly_won"`
	EqualLoadPercentilesStrictlyWon bool                `json:"equal_load_percentiles_strictly_won"`
	CapacityStrictlyWon             bool                `json:"capacity_strictly_won"`
	RoleIsolationProven             bool                `json:"role_isolation_proven"`
	RoleMachineIDHashes             roleMachineIDHashes `json:"role_machine_id_hashes"`
	Memory                          memoryEvidence      `json:"memory"`
	Artifacts                       []evidenceArtifact  `json:"artifacts"`
}

type roleMachineIDHashes struct {
	Client  string `json:"client"`
	Gateway string `json:"gateway"`
	Backend string `json:"backend"`
}

type memoryEvidence struct {
	CurrentAndPeakRecorded bool `json:"current_and_peak_recorded"`
	PerConnectionRecorded  bool `json:"per_connection_recorded"`
	NoRunawayGrowth        bool `json:"no_runaway_growth"`
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
	if !run.StrictSuperiority || !run.ZeroErrors || !run.SaturationOpsStrictlyWon || !run.EqualLoadPercentilesStrictlyWon || !run.CapacityStrictlyWon {
		return errors.New("strict superiority, zero errors, saturation, equal-load latency, and capacity wins are all required")
	}
	roles := []string{run.RoleMachineIDHashes.Client, run.RoleMachineIDHashes.Gateway, run.RoleMachineIDHashes.Backend}
	for _, hash := range roles {
		if !validSHA256(hash) {
			return fmt.Errorf("role machine-id hash %q is not a SHA-256 digest", hash)
		}
	}
	if !run.RoleIsolationProven {
		return errors.New("role isolation evidence is required")
	}
	if run.Kind == "cross-host-wss" && (roles[0] == roles[1] || roles[0] == roles[2] || roles[1] == roles[2]) {
		return errors.New("cross-host client, gateway, and backend machine-id hashes must be distinct")
	}
	if !run.Memory.CurrentAndPeakRecorded || !run.Memory.PerConnectionRecorded || !run.Memory.NoRunawayGrowth {
		return errors.New("memory current/peak, per-connection cost, and no-runaway-growth evidence are required")
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

func validSHA256(value string) bool {
	if len(value) != sha256.Size*2 {
		return false
	}
	_, err := hex.DecodeString(value)
	return err == nil
}
