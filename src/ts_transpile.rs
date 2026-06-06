//! TypeScript -> JavaScript transpilation embedded in the proxysss binary.
//!
//! proxysss does not depend on any external `deno`, `tsc`, or `node` toolchain.
//! Plugin and gateway scripts are authored in TypeScript and stripped to plain
//! modern JavaScript at load time using the same fast type-stripper that powers
//! Node.js `--experimental-strip-types` and Deno (`swc_ts_fast_strip`).
//!
//! Only TypeScript *type syntax* is removed; the embedded QuickJS engine already
//! supports modern ECMAScript, so no down-leveling is required. This keeps the
//! transform deterministic, allocation-light, and free of network or filesystem
//! access.

use anyhow::{anyhow, Result};
use swc_common::{
    errors::{ColorConfig, Handler},
    sync::Lrc,
    Globals, SourceMap, GLOBALS,
};
use swc_ts_fast_strip::{operate, Mode, Options, TransformConfig};

/// Transpile a TypeScript (or JavaScript) module source into plain JavaScript.
///
/// `filename` is only used for diagnostics. The returned string is an ES module
/// that the embedded QuickJS runtime can evaluate directly.
///
/// `Mode::Transform` is used so that fuller TypeScript constructs (`enum`,
/// `namespace`, parameter properties) are supported in addition to plain type
/// annotations, matching what plugin authors expect from "TypeScript".
pub fn transpile_module(filename: &str, source: &str) -> Result<String> {
    // Plain JavaScript (`.js`/`.mjs`/`.cjs`) runs directly in the embedded
    // QuickJS engine, so it is passed through untouched. Only the TypeScript
    // family is stripped.
    if !needs_transpile(filename) {
        return Ok(source.to_string());
    }

    let cm: Lrc<SourceMap> = Lrc::default();
    let handler = Handler::with_tty_emitter(ColorConfig::Never, false, false, Some(cm.clone()));

    let options = Options {
        module: Some(true),
        filename: Some(filename.to_string()),
        mode: Mode::Transform,
        transform: Some(TransformConfig::default()),
        ..Default::default()
    };

    let output = GLOBALS.set(&Globals::new(), || {
        operate(&cm, &handler, source.to_string(), options)
    });

    match output {
        Ok(out) => Ok(out.code),
        Err(error) => Err(anyhow!(
            "failed to transpile TypeScript module {}: {}",
            filename,
            error
        )),
    }
}

fn needs_transpile(filename: &str) -> bool {
    let lower = filename.to_ascii_lowercase();
    lower.ends_with(".ts")
        || lower.ends_with(".tsx")
        || lower.ends_with(".mts")
        || lower.ends_with(".cts")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_type_annotations() {
        let source = r#"
            export default {
                name: "demo",
                priority: 10 as number,
                access(message: { ctx: { path?: string } }): { upstream: string } | void {
                    const path: string = message.ctx.path ?? "";
                    if (path === "/healthz") {
                        return { upstream: "proxysss://healthz" };
                    }
                },
            };
        "#;

        let js = transpile_module("demo.ts", source).expect("transpile");
        assert!(js.contains("export default"));
        assert!(js.contains("access"));
        // Type annotations must be gone.
        assert!(!js.contains(": string"));
        assert!(!js.contains("as number"));
    }

    #[test]
    fn supports_enums() {
        let source = r#"
            enum Color { Red, Green }
            export default { name: "enum-demo", value: Color.Green };
        "#;
        let js = transpile_module("enum.ts", source).expect("transpile");
        assert!(js.contains("export default"));
    }

    #[test]
    fn passes_through_plain_javascript() {
        let source = "export default { name: \"plain\" };\n";
        let js = transpile_module("plain.js", source).expect("transpile");
        assert_eq!(js, source);
    }

    #[test]
    fn reports_syntax_errors() {
        let source = "export default { : : : };";
        let result = transpile_module("broken.ts", source);
        assert!(result.is_err());
    }
}
