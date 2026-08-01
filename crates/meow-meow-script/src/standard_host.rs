//! Crate-owned host for standalone MMS evaluation.

use crate::{
    ComponentHandle, Host, HostContext, HostError, HostErrorKind, HostRequest, HostResponse,
    MaterializedCE,
};

#[derive(Debug, Clone, PartialEq)]
pub struct CollectedComponent {
    pub handle: ComponentHandle,
    pub tree: MaterializedCE,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalAttachment {
    pub parent: Option<ComponentHandle>,
    pub child: ComponentHandle,
}

/// Standalone host that collects component output without emulating engine
/// operations.
#[derive(Debug, Default)]
pub struct StandardHost {
    roots: Vec<CollectedComponent>,
    registered: Vec<CollectedComponent>,
    attachments: Vec<LocalAttachment>,
}

impl StandardHost {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn roots(&self) -> &[CollectedComponent] {
        &self.roots
    }

    pub fn registered(&self) -> &[CollectedComponent] {
        &self.registered
    }

    pub fn attachments(&self) -> &[LocalAttachment] {
        &self.attachments
    }
}

impl Host for StandardHost {
    fn dispatch_with_context(
        &mut self,
        context: &mut HostContext,
        request: HostRequest,
    ) -> Result<HostResponse, HostError> {
        match request {
            HostRequest::Emit { tree } | HostRequest::Spawn { tree } => {
                let handle = context.allocate_component();
                let component_type = tree.component_type.clone();
                self.roots.push(CollectedComponent { handle, tree });
                Ok(HostResponse::Component {
                    handle,
                    component_type,
                })
            }
            HostRequest::RegisterComponent { tree } | HostRequest::Register { tree } => {
                let handle = context.allocate_component();
                let component_type = tree.component_type.clone();
                self.registered.push(CollectedComponent { handle, tree });
                Ok(HostResponse::Component {
                    handle,
                    component_type,
                })
            }
            HostRequest::Attach { parent, child } => {
                require_local_handle(context, child, "attach")?;
                if let Some(parent) = parent {
                    require_local_handle(context, parent, "attach")?;
                }
                self.attachments.push(LocalAttachment { parent, child });
                Ok(HostResponse::Unit)
            }
            other => Err(HostError::unsupported(other.operation_name())),
        }
    }
}

fn require_local_handle(
    context: &HostContext,
    handle: ComponentHandle,
    operation: &str,
) -> Result<(), HostError> {
    if context.owns_component(handle) {
        Ok(())
    } else {
        Err(HostError {
            kind: HostErrorKind::ForeignHandle,
            operation: operation.into(),
            message: format!("component handle {handle:?} is stale or foreign"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CeChild, ComponentNamePolicy, EvalError, EventStreamHost, HostCapabilities, HostEvent,
        Runtime, Value,
    };

    #[test]
    fn standard_runtime_collects_open_component_forest_in_authored_order() {
        let runtime = Runtime::standard();
        let mut session = runtime.session(StandardHost::new()).unwrap();

        session
            .eval(
                r#"
            Scene {
                title = "smoke"
                Header { "first" }
                Body { "second" }
            }
            "#,
            )
            .unwrap();

        let roots = session.host().roots();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].tree.component_type, "Scene");
        assert_eq!(
            roots[0].tree.named,
            [("title".into(), Value::String("smoke".into()))]
        );
        assert_eq!(roots[0].tree.children.len(), 2);
        assert!(
            matches!(&roots[0].tree.children[0], CeChild::Spawn(child) if child.component_type == "Header")
        );
        assert!(
            matches!(&roots[0].tree.children[1], CeChild::Spawn(child) if child.component_type == "Body")
        );
    }

    #[test]
    fn strict_runtime_rejects_unregistered_component_names() {
        let mut builder = Runtime::builder();
        builder.component_name_policy(ComponentNamePolicy::StrictRegistered);
        let mut session = builder.build().session(StandardHost::new()).unwrap();

        let EvalError::Runtime(message) = session.eval("UnknownComponent {}").unwrap_err() else {
            panic!("expected strict catalog validation error")
        };
        assert!(
            message.contains("unknown component 'UnknownComponent'"),
            "{message}"
        );
    }

    #[test]
    fn standard_host_returns_typed_errors_for_engine_only_operations() {
        let runtime = Runtime::standard();
        let mut session = runtime.session(StandardHost::new()).unwrap();

        let EvalError::Host(error) = session.eval("query(\"#engine\")").unwrap_err() else {
            panic!("expected typed host error")
        };
        assert_eq!(error.kind, HostErrorKind::UnsupportedHostOperation);
        assert_eq!(error.operation, "query");
    }

    #[test]
    fn standard_runtime_accepts_a_custom_host_without_a_builder() {
        let runtime = Runtime::standard();
        let host = EventStreamHost::new(HostCapabilities::default());
        let mut session = runtime.session(host).unwrap();

        session.eval("CustomRoot { CustomChild {} }").unwrap();

        assert!(matches!(
            &session.host().events[..],
            [HostEvent::Emit { tree, .. }] if tree.component_type == "CustomRoot"
        ));
    }
}
