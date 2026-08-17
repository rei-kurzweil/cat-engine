use meow_meow_script::{CeChild, Runner, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut runner = Runner::standard();

    runner.eval(
        r#"
        Showcase {
            title = "Standalone MMS"
            Header { "Runtime::standard()" }
            Content { "collected by StandardHost" }
        }
        "#,
    )?;

    let roots = runner.host().resolved_roots()?;
    let root = roots
        .first()
        .ok_or("the example emitted no component root")?;
    if root.tree.component_type != "Showcase"
        || root.tree.properties.len() != 1
        || root.tree.properties[0].name != "title"
        || root.tree.properties[0].value != Value::String("Standalone MMS".into())
        || root.tree.children.len() != 2
        || !matches!(&root.tree.children[0], CeChild::Spawn(child) if child.component_type == "Header")
        || !matches!(&root.tree.children[1], CeChild::Spawn(child) if child.component_type == "Content")
    {
        return Err("collected component forest did not preserve the authored structure".into());
    }

    println!(
        "collected {} root: {} ({} fields, {} children)",
        roots.len(),
        root.tree.component_type,
        root.tree.properties.len(),
        root.tree.children.len(),
    );

    Ok(())
}
