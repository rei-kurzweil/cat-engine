//! The crate-owned MMS runtime specification used by Mittens.
//!
//! This is deliberately a small first vertical slice. Component names and
//! aliases are authoritative here, while constructor/property/method/signal
//! declarations are migrated in subsequent slices.

use meow_meow_script as mms;

/// Engine implementations attached to host-effectful runtime declarations.
///
/// The initial slice binds a smoke API; component spawning still uses the
/// universal host protocol. Keeping this type and the completed table in the
/// build result prevents callers from reconstructing either half later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MittensBinding {
    Smoke,
}

/// The two inseparable products of the Mittens runtime builder.
#[derive(Debug)]
pub struct MittensRuntime {
    runtime: mms::Runtime,
    bindings: mms::ImplementationBindings<MittensBinding>,
}

impl MittensRuntime {
    pub fn runtime(&self) -> &mms::Runtime {
        &self.runtime
    }

    pub fn bindings(&self) -> &mms::ImplementationBindings<MittensBinding> {
        &self.bindings
    }
}

/// Build the strict MMS vocabulary and its opaque engine binding table.
pub fn build_mittens_runtime() -> Result<MittensRuntime, mms::RuntimeSpecError> {
    let mut builder = mms::RuntimeSpec::builder::<MittensBinding>();
    builder
        .component_name_policy(mms::ComponentNamePolicy::StrictRegistered)
        .with_standard_builtins();

    for &canonical in crate::scripting::component_registry::SUPPORTED_COMPONENT_NAMES {
        // `MusicNote` is currently both a standard builtin table and an
        // engine component name. The RuntimeSpec correctly rejects that
        // ambiguous global declaration; resolving the language spelling is a
        // separate vocabulary decision. The `NOTE` component shortform stays
        // on the legacy path until then.
        if canonical == "MusicNote" {
            continue;
        }
        builder.component(canonical, |component| {
            for shortform in mms::COMPONENT_SHORTFORMS.iter().filter(|entry| {
                entry.full == canonical && !entry.short.eq_ignore_ascii_case(canonical)
            }) {
                component.alias(shortform.short);
            }

            let floats = |count| {
                mms::ValueSignature::new(
                    vec![mms::ValueType::F32; count],
                    mms::ValueType::Component,
                )
            };
            let unsigned = |count| {
                mms::ValueSignature::new(
                    vec![mms::ValueType::U32; count],
                    mms::ValueType::Component,
                )
            };
            let no_args = || mms::ValueSignature::new(Vec::new(), mms::ValueType::Component);
            match canonical {
                "Transform" => {
                    for method in ["position", "scale", "rotation"] {
                        component
                            .constructor(method, floats(3))
                            .builder_call(method, floats(3));
                    }
                }
                "Renderable" => {
                    component.constructor("cube", no_args());
                }
                "Color" => {
                    component.constructor("rgba", floats(4));
                }
                "Emissive" => {
                    component
                        .constructor("on", no_args())
                        .constructor("off", no_args())
                        .constructor("intensity", floats(1))
                        .builder_call("intensity", floats(1));
                }
                "AmbientLight" => {
                    component.constructor("rgb", floats(3));
                }
                "Bloom" => {
                    component
                        .constructor("intensity", floats(1))
                        .builder_call("intensity", floats(1));
                }
                "RendererSettings" => {
                    component
                        .constructor("window_size", unsigned(2))
                        .builder_call("window_size", unsigned(2));
                }
                _ => {}
            }
        });
    }

    builder.namespace("mittens", |namespace| {
        namespace.api(
            "smoke",
            mms::ValueSignature::new(Vec::new(), mms::ValueType::Null),
            MittensBinding::Smoke,
        );
    });

    let build = builder.build()?;
    let (spec, bindings) = build.into_parts();
    Ok(MittensRuntime {
        runtime: mms::Runtime::from_spec(spec),
        bindings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_spec_contains_registry_names_and_supported_shortforms() {
        let configured = build_mittens_runtime().unwrap();
        let spec = configured.runtime().spec();

        for name in crate::scripting::component_registry::SUPPORTED_COMPONENT_NAMES {
            if *name == "MusicNote" {
                continue;
            }
            assert!(spec.component(name).is_some(), "missing component {name}");
            configured
                .runtime()
                .materialize_component(&format!("{name} {{}}"))
                .unwrap_or_else(|error| panic!("component {name} is not parseable: {error}"));
        }
        for shortform in mms::COMPONENT_SHORTFORMS {
            if shortform.full != "MusicNote"
                && crate::scripting::component_registry::SUPPORTED_COMPONENT_NAMES
                    .contains(&shortform.full)
            {
                assert!(
                    spec.component(shortform.short).is_some(),
                    "missing alias {} for {}",
                    shortform.short,
                    shortform.full
                );
                configured
                    .runtime()
                    .materialize_component(&format!("{} {{}}", shortform.short))
                    .unwrap_or_else(|error| {
                        panic!("alias {} is not parseable: {error}", shortform.short)
                    });
            }
        }
        let smoke_id = spec.api(Some("mittens"), "smoke").unwrap().operation_id();
        assert_eq!(
            configured.bindings().get(smoke_id),
            Some(&MittensBinding::Smoke)
        );
        assert!(spec.component("DefinitelyNotAMittensComponent").is_none());
        assert!(configured
            .runtime()
            .materialize_component("DefinitelyNotAMittensComponent {}")
            .is_err());
        assert!(configured
            .runtime()
            .materialize_component("RendererSettings.window_size(960, 720) {}")
            .is_ok());
        for invalid in ["720.5", "-1", "4294967296"] {
            let error = configured
                .runtime()
                .materialize_component(&format!(
                    "RendererSettings.window_size(960, {invalid}) {{}}"
                ))
                .unwrap_err()
                .to_string();
            assert!(error.contains("expected u32"), "{error}");
        }
    }
}
