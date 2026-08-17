use meow_meow_script::{
    ConfiguredRuntime, JsonLinesHost, RuntimeSpec, ValueSignature, ValueType,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = configured_runtime()?;
    let host = JsonLinesHost::new(Vec::new());
    let mut session = runtime.runtime().session(host);

    session.eval(
        r#"
        button.new("save") {
            label = "Save"
        }

        audit.write("button emitted")
        "#,
    )?;

    let bytes = session.host_mut().into_inner_ref();
    print!("{}", String::from_utf8_lossy(bytes));

    Ok(())
}

fn configured_runtime() -> Result<ConfiguredRuntime<&'static str>, Box<dyn std::error::Error>> {
    let mut builder = RuntimeSpec::builder();
    builder.with_standard_builtins();
    builder.host_component("Button", "Button.default", |component| {
        component
            .host_constructor(
                "new",
                ValueSignature::new(vec![ValueType::String], ValueType::Component),
                "Button.new",
            )
            .host_property("label", ValueType::String, "Button.label")
            .method(
                "click",
                ValueSignature::new(vec![], ValueType::Null),
                "Button.click",
            );
    });
    builder.namespace("audit", |namespace| {
        namespace.api(
            "write",
            ValueSignature::new(vec![ValueType::String], ValueType::Null),
            "audit.write",
        );
    });
    Ok(builder.build()?)
}
