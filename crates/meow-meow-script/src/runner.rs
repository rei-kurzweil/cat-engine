use crate::{EvalError, Evaluation, Evaluator, Host, Hostless, Runtime, Session, StandardHost};

/// Generic runner façade. Its private synchronous session is an initial
/// standalone implementation; the persistent worker can replace it without
/// changing the builder-free `Runner::standard()` entry point.
pub struct Runner {
    session: Session<StandardHost>,
}

impl Runner {
    pub fn standard() -> Self {
        Self {
            session: Runtime::standard().session(StandardHost::new()),
        }
    }

    pub fn eval(&mut self, source: &str) -> Result<Evaluation, EvalError> {
        self.session.eval(source)
    }

    pub fn host(&self) -> &StandardHost { self.session.host() }

    pub fn host_mut(&mut self) -> &mut StandardHost { self.session.host_mut() }
}

/// Convenience entry point for ordinary scripts that require no host powers.
pub struct HostlessRunner;

impl HostlessRunner {
    pub fn eval(source: &str) -> Result<Evaluation, EvalError> {
        let mut host = Hostless;
        Evaluator::new(&mut host).evaluate(source)
    }
}

/// Run a script with an application-provided synchronous host.
pub fn eval_with_host(source: &str, host: &mut impl Host) -> Result<Evaluation, EvalError> {
    Evaluator::new(host).evaluate(source)
}

/// Compatibility name for the language crate's pure runner. This does not
/// expose the old engine façade; engine-aware helpers live in
/// `mittens_engine::scripting`.
pub type MeowMeowRunner = HostlessRunner;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ComponentHandle, HostError, HostErrorKind, HostRequest, HostResponse, Value};

    #[derive(Default)]
    struct FakeHost {
        operations: Vec<String>,
        query_scopes: Vec<Option<ComponentHandle>>,
        fail_methods: bool,
    }

    impl Host for FakeHost {
        fn dispatch(&mut self, request: HostRequest) -> Result<HostResponse, HostError> {
            self.operations.push(request.operation_name().to_owned());
            match request {
                HostRequest::Query { scope, .. } => {
                    self.query_scopes.push(scope);
                    Ok(HostResponse::Component {
                        handle: ComponentHandle::from_raw(7),
                        component_type: "Fake".into(),
                    })
                }
                HostRequest::InvokeComponentMethod { operation_id, .. } if self.fail_methods => {
                    Err(HostError::failure(
                        format!("{operation_id:?}"),
                        "fake host rejected method",
                    ))
                }
                HostRequest::InvokeComponentMethod { .. } => {
                    Ok(HostResponse::Value(Value::Number(42.0)))
                }
                HostRequest::InvokeComponentMethodByName { method, .. } if self.fail_methods => {
                    Err(HostError::failure(method, "fake host rejected method"))
                }
                HostRequest::InvokeComponentMethodByName { .. } => {
                    Ok(HostResponse::Value(Value::Number(42.0)))
                }
                _ => Ok(HostResponse::Unit),
            }
        }
    }

    #[test]
    fn evaluates_pure_arithmetic() {
        let result = HostlessRunner::eval("1 + 2 * 3").unwrap();
        assert_eq!(result.value, Some(Value::Number(7.0)));
    }

    #[test]
    fn standard_runner_collects_open_component_output() {
        let mut runner = Runner::standard();
        runner.eval("SmokeRoot { SmokeChild {} }").unwrap();

        assert_eq!(runner.host().roots().len(), 1);
        assert_eq!(runner.host().roots()[0].tree.component_type, "SmokeRoot");
    }

    #[test]
    fn engine_expression_is_a_typed_host_error() {
        let error = HostlessRunner::eval("Text { \"hello\" }").unwrap_err();
        let EvalError::Host(error) = error else {
            panic!("expected host error")
        };
        assert_eq!(error.kind, HostErrorKind::UnsupportedHostOperation);
        assert_eq!(error.operation, "spawn");
    }

    #[test]
    fn queries_and_methods_dispatch_through_the_host() {
        let mut host = FakeHost::default();
        let result = eval_with_host("query(\"#target\").answer()", &mut host).unwrap();
        assert_eq!(result.value, Some(Value::Number(42.0)));
        assert_eq!(
            host.operations,
            ["query", "invoke_component_method_by_name"]
        );
    }

    #[test]
    fn component_scoped_dot_query_dispatches_as_a_query_not_a_component_method() {
        let mut host = FakeHost::default();
        let result = eval_with_host("query(\"#root\").query(\"#target\")", &mut host).unwrap();
        assert_eq!(result.value, Some(Value::ComponentObject {
            id: ComponentHandle::from_raw(7),
            component_type: "Fake".into(),
        }));
        assert_eq!(host.operations, ["query", "query", "attach"]);
        assert_eq!(
            host.query_scopes,
            [None, Some(ComponentHandle::from_raw(7))]
        );
    }

    #[test]
    fn host_failures_propagate_without_panicking() {
        let mut host = FakeHost {
            fail_methods: true,
            ..FakeHost::default()
        };
        let error = eval_with_host("query(\"#target\").explode()", &mut host).unwrap_err();
        let EvalError::Host(error) = error else {
            panic!("expected host error")
        };
        assert_eq!(error.kind, HostErrorKind::HostFailure);
        assert_eq!(error.operation, "explode");
    }
}
