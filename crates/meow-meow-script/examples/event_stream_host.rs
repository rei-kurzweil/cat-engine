use meow_meow_script::{
    ConfiguredRuntime, EventStreamHost, RuntimeSpec, ValueSignature, ValueType,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = configured_runtime()?;
    let host = EventStreamHost::new();
    let mut session = runtime.runtime().session(host);

    session.eval(
        r#"
        panel.new(320) {
            title = "Inventory"
        }

        telemetry.record({
            screen = "inventory",
            count = 12,
        })
        "#,
    )?;

    for event in &session.host().events {
        println!("{event:?}");
    }

    Ok(())
}

fn configured_runtime() -> Result<ConfiguredRuntime<&'static str>, Box<dyn std::error::Error>> {
    let mut builder = RuntimeSpec::builder();
    builder.with_standard_builtins();
    builder.host_component("Panel", "Panel.default", |component| {
        component
            .host_constructor(
                "new",
                ValueSignature::new(vec![ValueType::Number], ValueType::Component),
                "Panel.new",
            )
            .host_property("title", ValueType::String, "Panel.title")
            .method(
                "show",
                ValueSignature::new(vec![], ValueType::Null),
                "Panel.show",
            );
    });
    builder.namespace("telemetry", |namespace| {
        namespace.api(
            "record",
            ValueSignature::new(vec![ValueType::Any], ValueType::Null),
            "telemetry.record",
        );
    });
    Ok(builder.build()?)
}
