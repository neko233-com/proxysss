use super::*;

fn html_yaml_examples(html: &str) -> Vec<String> {
    let defaults = serde_yaml::to_value(GatewayConfig::default()).unwrap();
    let roots = defaults.as_mapping().unwrap();
    html.split("<pre")
        .skip(1)
        .filter_map(|part| {
            let content = part.split_once('>')?.1.trim_start().strip_prefix("<code")?;
            let (attributes, content) = content.split_once('>')?;
            let code = content
                .split_once("</code>")?
                .0
                .trim()
                .replace("&quot;", "\"")
                .replace("&#39;", "'")
                .replace("&#x27;", "'")
                .replace("&apos;", "'")
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&amp;", "&");
            let first = code
                .lines()
                .find(|line| !line.trim().is_empty() && !line.trim().starts_with('#'))
                .unwrap_or("");
            let known_root = roots
                .keys()
                .any(|key| first.starts_with(&format!("{}:", key.as_str().unwrap())));
            (attributes.contains("language-yaml") || known_root).then_some(code)
        })
        .collect()
}

fn parse_documented_examples(document: &str, examples: &[String]) -> Vec<serde_yaml::Value> {
    let mut failures = Vec::new();
    let parsed = collect_documented_examples(document, examples, &mut failures);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    parsed
}

fn fill_documented_secrets(config: &mut GatewayConfig) -> bool {
    let mut replaced = false;
    for site in &mut config.services.static_sites {
        for secret in [
            &mut site.security.origin_token,
            &mut site.security.signed_url.secret,
        ] {
            if secret == "CHANGE_ME" {
                *secret = uuid::Uuid::new_v4().simple().to_string();
                replaced = true;
            }
        }
    }
    replaced
}

fn collect_documented_examples(
    document: &str,
    examples: &[String],
    failures: &mut Vec<String>,
) -> Vec<serde_yaml::Value> {
    let mut parsed = Vec::new();
    for (index, yaml) in examples.iter().enumerate() {
        let label = format!("{document} YAML #{}", index + 1);
        match serde_yaml::from_str::<GatewayConfig>(yaml) {
            Ok(mut config) => {
                let source = serde_yaml::from_str(yaml).unwrap();
                let effective = serde_yaml::to_value(&config).unwrap();
                assert_documented_fields_preserved(&source, &effective, &label);
                // CDN templates intentionally cannot start with CHANGE_ME.
                // Preserve that guard, then validate with per-test credentials.
                let originally_valid = config.validate().is_ok();
                if fill_documented_secrets(&mut config) {
                    assert!(!originally_valid, "{label}: placeholder must be rejected");
                }
                if let Err(error) = config.validate() {
                    failures.push(format!("{label}: {error}"));
                    continue;
                }
                parsed.push(source);
            }
            Err(error) => failures.push(format!("{label}: {error}")),
        }
    }
    parsed
}

fn assert_documented_fields_preserved(
    source: &serde_yaml::Value,
    effective: &serde_yaml::Value,
    path: &str,
) {
    match source {
        serde_yaml::Value::Mapping(fields) => {
            for (key, value) in fields {
                let name = key.as_str().expect("configuration field name");
                let child = format!("{path}.{name}");
                let actual = effective.get(key).unwrap_or_else(|| {
                    panic!("{child}: documented field is silently ignored or noncanonical")
                });
                assert_documented_fields_preserved(value, actual, &child);
            }
        }
        serde_yaml::Value::Sequence(items) => {
            let actual = effective.as_sequence().expect("configuration sequence");
            assert_eq!(items.len(), actual.len(), "{path}");
            for (index, value) in items.iter().enumerate() {
                assert_documented_fields_preserved(
                    value,
                    &actual[index],
                    &format!("{path}[{index}]"),
                );
            }
        }
        _ => assert_eq!(
            source, effective,
            "{path}: example value changed during parsing"
        ),
    }
}

fn assert_documented_example_pair(
    markdown: &str,
    html: &str,
    markdown_name: &str,
    html_name: &str,
    minimum_examples: usize,
) {
    let markdown_examples = markdown
        .split("```yaml")
        .skip(1)
        .map(|part| part.split("```").next().unwrap().trim().to_owned());
    let documents = [
        (markdown_name, markdown_examples.collect::<Vec<_>>()),
        (html_name, html_yaml_examples(html)),
    ];
    let mut parsed_examples = [Vec::new(), Vec::new()];
    for (document_index, (document, examples)) in documents.iter().enumerate() {
        assert!(
            examples.len() >= minimum_examples,
            "missing examples in {document}"
        );
        parsed_examples[document_index] = parse_documented_examples(document, examples);
    }
    assert_eq!(
        parsed_examples[0].len(),
        parsed_examples[1].len(),
        "MD/HTML example count"
    );
    let mut remaining = parsed_examples[1].clone();
    for example in &parsed_examples[0] {
        let position = remaining
            .iter()
            .position(|other| other == example)
            .unwrap_or_else(|| panic!("MD example missing or different in HTML: {example:?}"));
        remaining.remove(position);
    }
}

#[test]
fn configuration_guide_examples_are_executable_and_keep_every_field() {
    assert_documented_example_pair(
        include_str!("../../docs/CONFIGURATION.md"),
        include_str!("../../docs/configuration.html"),
        "CONFIGURATION.md",
        "configuration.html",
        9,
    );
}

#[test]
fn architecture_examples_are_executable_and_keep_every_field() {
    assert_documented_example_pair(
        include_str!("../../docs/ARCHITECTURE.md"),
        include_str!("../../docs/architecture.html"),
        "ARCHITECTURE.md",
        "architecture.html",
        6,
    );
}

#[test]
fn migration_examples_are_executable_and_paired() {
    for (name, markdown, html) in [
        (
            "nginx",
            include_str!("../../nginx-to-proxysss.md"),
            include_str!("../../docs/nginx-to-proxysss.html"),
        ),
        (
            "caddy",
            include_str!("../../caddy-to-proxysss.md"),
            include_str!("../../docs/caddy-to-proxysss.html"),
        ),
    ] {
        assert_documented_example_pair(
            markdown,
            html,
            &format!("{name}.md"),
            &format!("{name}.html"),
            9,
        );
    }
}

#[test]
fn public_and_embedded_html_configurations_preserve_every_field() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs");
    let mut examples = 0;
    let mut failures = Vec::new();
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path
            .extension()
            .is_some_and(|extension| extension == "html")
        {
            let html = fs::read_to_string(&path).unwrap();
            let yaml = html_yaml_examples(&html);
            examples += yaml.len();
            collect_documented_examples(&path.display().to_string(), &yaml, &mut failures);
        }
    }
    assert!(examples >= 54, "public configuration examples disappeared");
    let embedded = crate::gateway::render_docs_html(&GatewayConfig::default());
    let yaml = html_yaml_examples(&embedded);
    assert!(
        yaml.len() >= 10,
        "embedded configuration examples disappeared"
    );
    collect_documented_examples("embedded /docs", &yaml, &mut failures);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn cdn_examples_are_executable_after_replacing_required_secrets() {
    assert_documented_example_pair(
        include_str!("../../docs/CDN-ORIGIN.md"),
        include_str!("../../docs/cdn-origin.html"),
        "CDN-ORIGIN.md",
        "cdn-origin.html",
        3,
    );
}

#[test]
fn shipped_gateway_yaml_examples_load_with_strict_fields() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut directories = vec![root.join("examples"), root.join("templates")];
    let mut failures = Vec::new();
    let mut checked = 0;
    while let Some(directory) = directories.pop() {
        for entry in fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                directories.push(path);
            } else if path
                .extension()
                .is_some_and(|extension| extension == "yaml")
                && !path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .ends_with(".plugin.yaml")
            {
                checked += 1;
                if let Err(original_error) = GatewayConfig::load(&path) {
                    let replaced_result = (|| -> Result<()> {
                        let mut config: GatewayConfig =
                            serde_yaml::from_value(load_config_value(&path)?)?;
                        config.normalize(path.parent().unwrap());
                        if !fill_documented_secrets(&mut config) {
                            return Err(original_error);
                        }
                        config.validate()
                    })();
                    if let Err(error) = replaced_result {
                        failures.push(format!("{}: {error:#}", path.display()));
                    }
                }
            }
        }
    }
    assert!(checked >= 14, "shipped gateway YAML coverage disappeared");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
