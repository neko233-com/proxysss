// Local documentation checks. CI remains packaging-only.
package main

import (
	"fmt"
	"net/url"
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"unicode/utf8"
)

var docHref = regexp.MustCompile(`(?i)\b(?:href|src)\s*=\s*["']([^"']+)["']`)
var docMarkdownLink = regexp.MustCompile(`\[[^\]]*\]\(([^\s)]+)\)`)

func main() {
	if err := verifyDocs("."); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	fmt.Println("documentation pairs, UTF-8, local links and HTML navigation verified")
}

func verifyDocs(root string) error {
	pairs := [][2]string{
		{"docs/ARCHITECTURE.md", "docs/architecture.html"},
		{"docs/CONFIGURATION.md", "docs/configuration.html"},
		{"docs/CDN-ORIGIN.md", "docs/cdn-origin.html"},
		{"docs/SECURITY-PERFORMANCE.md", "docs/security-performance.html"},
		{"docs/VALIDATION.md", "docs/validation.html"},
		{"ts-how-to-use.md", "docs/ts-how-to-use.html"},
		{"nginx-to-proxysss.md", "docs/nginx-to-proxysss.html"},
		{"caddy-to-proxysss.md", "docs/caddy-to-proxysss.html"},
	}
	files := []string{"AGENTS.md", "docs/AGENTS.md", "docs/index.html", "templates/docs.html"}
	for _, pair := range pairs {
		files = append(files, pair[:]...)
	}
	if err := filepath.WalkDir(filepath.Join(root, "docs"), func(path string, entry os.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if !entry.IsDir() && (strings.HasSuffix(path, ".html") || strings.HasSuffix(path, ".md")) {
			rel, err := filepath.Rel(root, path)
			if err != nil {
				return err
			}
			files = append(files, rel)
		}
		return nil
	}); err != nil {
		return err
	}
	seen := map[string]bool{}
	for _, file := range files {
		file = filepath.Clean(file)
		if seen[file] {
			continue
		}
		seen[file] = true
		payload, err := os.ReadFile(filepath.Join(root, file))
		if err != nil {
			return fmt.Errorf("documentation %s: %w", file, err)
		}
		if !utf8.Valid(payload) || len(payload) == 0 {
			return fmt.Errorf("documentation %s: empty or invalid UTF-8", file)
		}
		html := strings.HasSuffix(file, ".html")
		pattern := docMarkdownLink
		if html {
			pattern = docHref
		}
		for _, match := range pattern.FindAllSubmatch(payload, -1) {
			if err := verifyDocLink(root, file, string(match[1]), html); err != nil {
				return err
			}
		}
	}
	return nil
}

func verifyDocLink(root, source, target string, html bool) error {
	link, err := url.Parse(target)
	if err != nil {
		return fmt.Errorf("%s: invalid link %q: %w", source, target, err)
	}
	if link.Scheme != "" || link.Host != "" || link.Path == "" {
		return nil
	}
	if html && strings.EqualFold(filepath.Ext(link.Path), ".md") {
		return fmt.Errorf("%s: public HTML navigation must use HTML: %s", source, target)
	}
	if strings.HasPrefix(link.Path, "/") {
		return nil
	} // Runtime routes, not repository paths.
	path := filepath.Join(filepath.Dir(source), filepath.FromSlash(link.Path))
	if _, err := os.Stat(filepath.Join(root, path)); err != nil {
		return fmt.Errorf("%s: broken local link %s: %w", source, target, err)
	}
	if html && link.Fragment != "" && strings.HasSuffix(path, ".html") {
		page, err := os.ReadFile(filepath.Join(root, path))
		if err != nil {
			return err
		}
		id := regexp.MustCompile(`(?i)\bid\s*=\s*["']` + regexp.QuoteMeta(link.Fragment) + `["']`)
		if !id.Match(page) {
			return fmt.Errorf("%s: missing HTML anchor %s", source, target)
		}
	}
	return nil
}
