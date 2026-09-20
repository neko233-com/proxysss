package main

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestMigrationFreshness(t *testing.T) {
	root := t.TempDir()
	if err := os.Mkdir(filepath.Join(root, "docs"), 0755); err != nil {
		t.Fatal(err)
	}
	source := "# Guide\n\nCurrent behavior.\n"
	body := "<section>Current behavior.</section>"
	page := fmt.Sprintf("<!-- migration-source: guide.md sha256=%s body-sha256=%s -->\n%s\n<!-- /migration-source -->", normalizedDocHash(source), normalizedDocHash(body), body)
	for _, scenario := range []struct {
		name, markdown, html string
		valid                bool
	}{
		{"current", source, page, true},
		{"windows-newlines", strings.ReplaceAll(source, "\n", "\r\n"), strings.ReplaceAll(page, "\n", "\r\n"), true},
		{"changed-source", source + "New behavior.\n", page, false},
		{"changed-html", source, strings.ReplaceAll(page, "Current behavior.", "Stale behavior."), false},
		{"missing-marker", source, body, false},
		{"duplicate-marker", source, page + page, false},
	} {
		t.Run(scenario.name, func(t *testing.T) {
			for path, content := range map[string]string{"guide.md": scenario.markdown, "docs/guide.html": scenario.html} {
				if err := os.WriteFile(filepath.Join(root, path), []byte(content), 0644); err != nil {
					t.Fatal(err)
				}
			}
			err := verifyMigrationContent(root, "guide")
			if (err == nil) != scenario.valid {
				t.Fatalf("valid=%v, error=%v", scenario.valid, err)
			}
		})
	}
}
