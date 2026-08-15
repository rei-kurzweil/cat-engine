//! Configurable MMS runtime and persistent sessions.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use crate::{CallbackHandle, EvalError, Evaluation, Evaluator, Expression, HeapHandle, Host,
    HostCapabilities, HostContext, Hostless, MaterializedCE, MeowMeowParser, MeowMeowTokenizer,
    Statement, Value};

static NEXT_SESSION_TAG: AtomicU32 = AtomicU32::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentNamePolicy {
    OpenUppercase,
    StrictRegistered,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueType {
    Any,
    Null,
    Bool,
    /// Compatibility type for the pre-0.8 single-`f64` numeric surface.
    /// New RuntimeSpecs should use a fixed-width numeric type.
    Number,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    String,
    Array,
    Table,
    Component,
    Callback,
}

impl ValueType {
    pub fn accepts(&self, value: &Value) -> bool {
        matches!(self, Self::Any)
            || matches!((self, value),
                (Self::Null, Value::Null) | (Self::Bool, Value::Bool(_))
                | (Self::Number, Value::Number(_)) | (Self::String, Value::String(_))
                | (Self::Array, Value::Array(_)) | (Self::Table, Value::Map(_) | Value::Object(_))
                | (Self::Component, Value::ComponentObject { .. })
                | (Self::Callback, Value::Function { .. }))
            || match (self, value) {
                (Self::I8, Value::Number(value)) => integer_in_range(*value, i8::MIN, i8::MAX),
                (Self::I16, Value::Number(value)) => integer_in_range(*value, i16::MIN, i16::MAX),
                (Self::I32, Value::Number(value)) => integer_in_range(*value, i32::MIN, i32::MAX),
                (Self::I64, Value::Number(value)) => {
                    value.is_finite()
                        && value.fract() == 0.0
                        && *value >= i64::MIN as f64
                        && *value < 9_223_372_036_854_775_808.0
                }
                (Self::U8, Value::Number(value)) => unsigned_integer_in_range(*value, u8::MAX),
                (Self::U16, Value::Number(value)) => unsigned_integer_in_range(*value, u16::MAX),
                (Self::U32, Value::Number(value)) => unsigned_integer_in_range(*value, u32::MAX),
                (Self::U64, Value::Number(value)) => {
                    value.is_finite()
                        && value.fract() == 0.0
                        && *value >= 0.0
                        && *value < 18_446_744_073_709_551_616.0
                }
                // Until numeric values retain their source width, an ordinary
                // Number is contextually accepted at either floating boundary.
                (Self::F32, Value::Number(value)) => {
                    !value.is_finite() || (*value as f32).is_finite()
                }
                (Self::F64, Value::Number(_)) => true,
                _ => false,
            }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::Null => "null",
            Self::Bool => "bool",
            Self::Number => "number",
            Self::I8 => "i8",
            Self::I16 => "i16",
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::U32 => "u32",
            Self::U64 => "u64",
            Self::F32 => "f32",
            Self::F64 => "f64",
            Self::String => "str",
            Self::Array => "array",
            Self::Table => "table",
            Self::Component => "component",
            Self::Callback => "callback",
        }
    }
}

fn integer_in_range<T>(value: f64, min: T, max: T) -> bool
where
    T: Copy + Into<i64>,
{
    value.is_finite()
        && value.fract() == 0.0
        && value >= min.into() as f64
        && value <= max.into() as f64
}

fn unsigned_integer_in_range<T>(value: f64, max: T) -> bool
where
    T: Copy + Into<u64>,
{
    value.is_finite()
        && value.fract() == 0.0
        && value >= 0.0
        && value <= max.into() as f64
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueSignature {
    pub arguments: Vec<ValueType>,
    /// The number of leading arguments which must be supplied. Remaining
    /// declared arguments are optional and retain their declared types.
    pub minimum_arguments: usize,
    pub result: ValueType,
    pub variadic: bool,
}

/// Opaque identity assigned to a declaration whose implementation crosses the
/// host boundary. IDs can only be allocated by [`RuntimeSpecBuilder`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OperationId(u32);

impl fmt::Debug for OperationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("OperationId").field(&self.0).finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentBodyMode {
    Standard,
    PropsOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ImplementationTarget {
    Pure,
    Host(OperationId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallableDeclaration {
    name: String,
    signature: ValueSignature,
    target: ImplementationTarget,
}

impl CallableDeclaration {
    pub fn name(&self) -> &str { &self.name }
    pub fn signature(&self) -> &ValueSignature { &self.signature }
    pub fn operation_id(&self) -> Option<OperationId> {
        match self.target { ImplementationTarget::Pure => None, ImplementationTarget::Host(id) => Some(id) }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyDeclaration {
    name: String,
    value_type: ValueType,
    target: ImplementationTarget,
}

impl PropertyDeclaration {
    pub fn name(&self) -> &str { &self.name }
    pub fn value_type(&self) -> &ValueType { &self.value_type }
    pub fn operation_id(&self) -> Option<OperationId> {
        match self.target { ImplementationTarget::Pure => None, ImplementationTarget::Host(id) => Some(id) }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalDeclaration {
    name: String,
    fields: Vec<(String, ValueType)>,
    operation_id: OperationId,
}

impl SignalDeclaration {
    pub fn name(&self) -> &str { &self.name }
    pub fn fields(&self) -> impl ExactSizeIterator<Item = (&str, &ValueType)> {
        self.fields.iter().map(|(name, ty)| (name.as_str(), ty))
    }
    pub fn operation_id(&self) -> OperationId { self.operation_id }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentDeclaration {
    name: String,
    target: ImplementationTarget,
    aliases: Vec<String>,
    body_mode: ComponentBodyMode,
    constructors: Vec<CallableDeclaration>,
    builder_calls: Vec<CallableDeclaration>,
    positionals: Vec<ValueType>,
    properties: Vec<PropertyDeclaration>,
    methods: Vec<CallableDeclaration>,
    signals: Vec<SignalDeclaration>,
}

impl ComponentDeclaration {
    pub fn name(&self) -> &str { &self.name }
    pub fn operation_id(&self) -> Option<OperationId> {
        match self.target { ImplementationTarget::Pure => None, ImplementationTarget::Host(id) => Some(id) }
    }
    pub fn aliases(&self) -> impl ExactSizeIterator<Item = &str> { self.aliases.iter().map(String::as_str) }
    pub fn body_mode(&self) -> ComponentBodyMode { self.body_mode }
    pub fn constructors(&self) -> impl ExactSizeIterator<Item = &CallableDeclaration> { self.constructors.iter() }
    pub fn builder_calls(&self) -> impl ExactSizeIterator<Item = &CallableDeclaration> { self.builder_calls.iter() }
    pub fn positionals(&self) -> impl ExactSizeIterator<Item = &ValueType> { self.positionals.iter() }
    pub fn properties(&self) -> impl ExactSizeIterator<Item = &PropertyDeclaration> { self.properties.iter() }
    pub fn methods(&self) -> impl ExactSizeIterator<Item = &CallableDeclaration> { self.methods.iter() }
    pub fn signals(&self) -> impl ExactSizeIterator<Item = &SignalDeclaration> { self.signals.iter() }
    pub fn method(&self, name: &str) -> Option<&CallableDeclaration> {
        self.methods.iter().find(|method| method.name.eq_ignore_ascii_case(name))
    }
    pub fn signal(&self, name: &str) -> Option<&SignalDeclaration> {
        self.signals.iter().find(|signal| signal.name.eq_ignore_ascii_case(name))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltinDeclaration {
    name: String,
    signature: ValueSignature,
    target: ImplementationTarget,
}

impl BuiltinDeclaration {
    pub fn name(&self) -> &str { &self.name }
    pub fn signature(&self) -> &ValueSignature { &self.signature }
    pub fn operation_id(&self) -> Option<OperationId> {
        match self.target { ImplementationTarget::Pure => None, ImplementationTarget::Host(id) => Some(id) }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiDeclaration {
    namespace: Option<String>,
    name: String,
    signature: ValueSignature,
    operation_id: OperationId,
}

impl ApiDeclaration {
    pub fn namespace(&self) -> Option<&str> { self.namespace.as_deref() }
    pub fn name(&self) -> &str { &self.name }
    pub fn signature(&self) -> &ValueSignature { &self.signature }
    pub fn operation_id(&self) -> OperationId { self.operation_id }
}

/// Immutable, crate-owned description of one MMS vocabulary.
#[derive(Debug, Clone)]
pub struct RuntimeSpec {
    component_name_policy: ComponentNamePolicy,
    components: Vec<ComponentDeclaration>,
    builtins: Vec<BuiltinDeclaration>,
    apis: Vec<ApiDeclaration>,
}

impl RuntimeSpec {
    pub fn builder<I>() -> RuntimeSpecBuilder<I> { RuntimeSpecBuilder::new() }
    pub fn component_name_policy(&self) -> ComponentNamePolicy { self.component_name_policy }
    pub fn components(&self) -> impl ExactSizeIterator<Item = &ComponentDeclaration> { self.components.iter() }
    pub fn builtins(&self) -> impl ExactSizeIterator<Item = &BuiltinDeclaration> { self.builtins.iter() }
    pub fn apis(&self) -> impl ExactSizeIterator<Item = &ApiDeclaration> { self.apis.iter() }
    pub fn component(&self, name: &str) -> Option<&ComponentDeclaration> {
        self.components.iter().find(|component| component.name.eq_ignore_ascii_case(name)
            || component.aliases.iter().any(|alias| alias.eq_ignore_ascii_case(name)))
    }
    pub fn api(&self, namespace: Option<&str>, name: &str) -> Option<&ApiDeclaration> {
        self.apis.iter().find(|api| api.name.eq_ignore_ascii_case(name)
            && match (api.namespace.as_deref(), namespace) {
                (None, None) => true,
                (Some(left), Some(right)) => left.eq_ignore_ascii_case(right),
                _ => false,
            })
    }
}

/// Metadata-free host implementation table built alongside a [`RuntimeSpec`].
#[derive(Debug)]
pub struct ImplementationBindings<I> {
    entries: Vec<(OperationId, I)>,
}

impl<I> ImplementationBindings<I> {
    pub fn len(&self) -> usize { self.entries.len() }
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }
    pub fn get(&self, id: OperationId) -> Option<&I> {
        self.entries.iter().find_map(|(candidate, implementation)| (*candidate == id).then_some(implementation))
    }
    pub fn iter(&self) -> impl ExactSizeIterator<Item = (OperationId, &I)> {
        self.entries.iter().map(|(id, implementation)| (*id, implementation))
    }
}

/// The inseparable executable runtime and host bindings produced by one
/// [`RuntimeSpecBuilder`] build.
///
/// An [`OperationId`] is meaningful only with the bindings from the same
/// configured runtime, so this type prevents callers from compiling a spec
/// and accidentally pairing it with a different binding table.
#[derive(Debug)]
pub struct ConfiguredRuntime<I> {
    runtime: Runtime,
    bindings: ImplementationBindings<I>,
}

impl<I> ConfiguredRuntime<I> {
    pub fn runtime(&self) -> &Runtime { &self.runtime }
    pub fn spec(&self) -> &RuntimeSpec { self.runtime.spec() }
    pub fn bindings(&self) -> &ImplementationBindings<I> { &self.bindings }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSpecErrorKind {
    DuplicateName,
    NameConflict,
    InvalidNesting,
    ConflictingSignature,
    MissingImplementation,
    OrphanImplementation,
    DuplicateOperationId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSpecError {
    pub kind: RuntimeSpecErrorKind,
    pub path: String,
    pub message: String,
}

impl fmt::Display for RuntimeSpecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(&self.message) }
}
impl std::error::Error for RuntimeSpecError {}

pub struct RuntimeSpecBuilder<I> {
    component_name_policy: ComponentNamePolicy,
    components: Vec<ComponentDeclaration>,
    builtins: Vec<BuiltinDeclaration>,
    apis: Vec<ApiDeclaration>,
    bindings: Vec<(OperationId, I)>,
    next_operation_id: u32,
}

impl<I> Default for RuntimeSpecBuilder<I> {
    fn default() -> Self {
        Self { component_name_policy: ComponentNamePolicy::StrictRegistered, components: vec![],
            builtins: vec![], apis: vec![], bindings: vec![], next_operation_id: 1 }
    }
}

impl<I> RuntimeSpecBuilder<I> {
    pub fn new() -> Self { Self::default() }
    pub fn component_name_policy(&mut self, policy: ComponentNamePolicy) -> &mut Self {
        self.component_name_policy = policy; self
    }
    pub fn with_standard_builtins(&mut self) -> &mut Self {
        for name in ["null", "range", "len", "query", "query_all", "Math", "MusicNote"] {
            self.pure_builtin(name, ValueSignature::any());
        }
        self
    }
    pub fn pure_builtin(&mut self, name: impl Into<String>, signature: ValueSignature) -> &mut Self {
        self.builtins.push(BuiltinDeclaration { name: name.into(), signature, target: ImplementationTarget::Pure }); self
    }
    pub fn host_builtin(&mut self, name: impl Into<String>, signature: ValueSignature, implementation: I) -> &mut Self {
        let id = self.bind(implementation);
        self.builtins.push(BuiltinDeclaration { name: name.into(), signature, target: ImplementationTarget::Host(id) }); self
    }
    pub fn component(&mut self, name: impl Into<String>, configure: impl FnOnce(&mut ComponentBuilder<'_, I>)) -> &mut Self {
        let declaration = ComponentDeclaration { name: name.into(), target: ImplementationTarget::Pure,
            aliases: vec![], body_mode: ComponentBodyMode::Standard,
            constructors: vec![], builder_calls: vec![], positionals: vec![], properties: vec![], methods: vec![], signals: vec![] };
        let mut builder = ComponentBuilder { declaration, bindings: &mut self.bindings, next_operation_id: &mut self.next_operation_id };
        configure(&mut builder);
        self.components.push(builder.declaration); self
    }
    /// Declares a component whose factory is implemented by the host.
    pub fn host_component(
        &mut self,
        name: impl Into<String>,
        implementation: I,
        configure: impl FnOnce(&mut ComponentBuilder<'_, I>),
    ) -> &mut Self {
        let operation_id = self.bind(implementation);
        let declaration = ComponentDeclaration { name: name.into(), target: ImplementationTarget::Host(operation_id),
            aliases: vec![], body_mode: ComponentBodyMode::Standard,
            constructors: vec![], builder_calls: vec![], positionals: vec![], properties: vec![], methods: vec![], signals: vec![] };
        let mut builder = ComponentBuilder { declaration, bindings: &mut self.bindings, next_operation_id: &mut self.next_operation_id };
        configure(&mut builder);
        self.components.push(builder.declaration); self
    }
    pub fn host_api(&mut self, name: impl Into<String>, signature: ValueSignature, implementation: I) -> &mut Self {
        let id = self.bind(implementation);
        self.apis.push(ApiDeclaration { namespace: None, name: name.into(), signature, operation_id: id }); self
    }
    pub fn namespace(&mut self, name: impl Into<String>, configure: impl FnOnce(&mut NamespaceBuilder<'_, I>)) -> &mut Self {
        let mut builder = NamespaceBuilder { name: name.into(), apis: &mut self.apis,
            bindings: &mut self.bindings, next_operation_id: &mut self.next_operation_id };
        configure(&mut builder); self
    }
    pub fn build(self) -> Result<ConfiguredRuntime<I>, RuntimeSpecError> {
        validate_runtime_spec(&self.components, &self.builtins, &self.apis)?;
        validate_bindings(&self.components, &self.builtins, &self.apis, &self.bindings)?;
        let spec = RuntimeSpec { component_name_policy: self.component_name_policy,
            components: self.components, builtins: self.builtins, apis: self.apis };
        Ok(ConfiguredRuntime { runtime: Runtime::from_spec(spec),
            bindings: ImplementationBindings { entries: self.bindings } })
    }
    fn bind(&mut self, implementation: I) -> OperationId {
        let id = OperationId(self.next_operation_id);
        self.next_operation_id = self.next_operation_id.checked_add(1).expect("runtime operation ID space exhausted");
        self.bindings.push((id, implementation)); id
    }
}

pub struct ComponentBuilder<'a, I> {
    declaration: ComponentDeclaration,
    bindings: &'a mut Vec<(OperationId, I)>,
    next_operation_id: &'a mut u32,
}

impl<I> ComponentBuilder<'_, I> {
    pub fn alias(&mut self, alias: impl Into<String>) -> &mut Self { self.declaration.aliases.push(alias.into()); self }
    /// Marks this component's factory as host-implemented.
    pub fn host_implementation(&mut self, implementation: I) -> &mut Self {
        let id = self.bind(implementation);
        self.declaration.target = ImplementationTarget::Host(id);
        self
    }
    pub fn body_mode(&mut self, mode: ComponentBodyMode) -> &mut Self { self.declaration.body_mode = mode; self }
    pub fn positional(&mut self, ty: ValueType) -> &mut Self { self.declaration.positionals.push(ty); self }
    pub fn constructor(&mut self, name: impl Into<String>, signature: ValueSignature) -> &mut Self {
        self.declaration.constructors.push(CallableDeclaration { name: name.into(), signature, target: ImplementationTarget::Pure }); self
    }
    pub fn host_constructor(&mut self, name: impl Into<String>, signature: ValueSignature, implementation: I) -> &mut Self {
        let id = self.bind(implementation);
        self.declaration.constructors.push(CallableDeclaration { name: name.into(), signature, target: ImplementationTarget::Host(id) }); self
    }
    pub fn builder_call(&mut self, name: impl Into<String>, signature: ValueSignature) -> &mut Self {
        self.declaration.builder_calls.push(CallableDeclaration { name: name.into(), signature, target: ImplementationTarget::Pure }); self
    }
    pub fn host_builder_call(&mut self, name: impl Into<String>, signature: ValueSignature, implementation: I) -> &mut Self {
        let id = self.bind(implementation);
        self.declaration.builder_calls.push(CallableDeclaration { name: name.into(), signature, target: ImplementationTarget::Host(id) }); self
    }
    pub fn property(&mut self, name: impl Into<String>, ty: ValueType) -> &mut Self {
        self.declaration.properties.push(PropertyDeclaration { name: name.into(), value_type: ty, target: ImplementationTarget::Pure }); self
    }
    pub fn host_property(&mut self, name: impl Into<String>, ty: ValueType, implementation: I) -> &mut Self {
        let id = self.bind(implementation);
        self.declaration.properties.push(PropertyDeclaration { name: name.into(), value_type: ty, target: ImplementationTarget::Host(id) }); self
    }
    pub fn method(&mut self, name: impl Into<String>, signature: ValueSignature, implementation: I) -> &mut Self {
        let id = self.bind(implementation);
        self.declaration.methods.push(CallableDeclaration { name: name.into(), signature, target: ImplementationTarget::Host(id) }); self
    }
    pub fn signal(&mut self, name: impl Into<String>, fields: impl Into<Vec<(String, ValueType)>>, implementation: I) -> &mut Self {
        let id = self.bind(implementation);
        self.declaration.signals.push(SignalDeclaration { name: name.into(), fields: fields.into(), operation_id: id }); self
    }
    fn bind(&mut self, implementation: I) -> OperationId {
        let id = OperationId(*self.next_operation_id);
        *self.next_operation_id = self.next_operation_id.checked_add(1).expect("runtime operation ID space exhausted");
        self.bindings.push((id, implementation)); id
    }
}

pub struct NamespaceBuilder<'a, I> {
    name: String,
    apis: &'a mut Vec<ApiDeclaration>,
    bindings: &'a mut Vec<(OperationId, I)>,
    next_operation_id: &'a mut u32,
}

impl<I> NamespaceBuilder<'_, I> {
    pub fn api(&mut self, name: impl Into<String>, signature: ValueSignature, implementation: I) -> &mut Self {
        let id = OperationId(*self.next_operation_id);
        *self.next_operation_id = self.next_operation_id.checked_add(1).expect("runtime operation ID space exhausted");
        self.bindings.push((id, implementation));
        self.apis.push(ApiDeclaration { namespace: Some(self.name.clone()), name: name.into(), signature, operation_id: id }); self
    }
}

fn validate_runtime_spec(components: &[ComponentDeclaration], builtins: &[BuiltinDeclaration], apis: &[ApiDeclaration]) -> Result<(), RuntimeSpecError> {
    let mut global_names = HashMap::<String, String>::new();
    for builtin in builtins {
        validate_declaration_name(&builtin.name, &builtin.name)?;
        claim_name(&mut global_names, &builtin.name, &builtin.name)?;
    }
    for component in components {
        validate_declaration_name(&component.name, &component.name)?;
        claim_name(&mut global_names, &component.name, &component.name)?;
        for alias in &component.aliases {
            let path = format!("{}.alias({alias})", component.name);
            validate_declaration_name(alias, &path)?;
            claim_name(&mut global_names, alias, &path)?;
        }
        validate_named(&component.name, "constructor", component.constructors.iter().map(|item| (item.name.clone(), item.signature.clone())))?;
        validate_named(&component.name, "builder_call", component.builder_calls.iter().map(|item| (item.name.clone(), item.signature.clone())))?;
        validate_named(&component.name, "property", component.properties.iter().map(|item| (item.name.clone(), ValueSignature::new(vec![], item.value_type.clone()))))?;
        validate_named(&component.name, "method", component.methods.iter().map(|item| (item.name.clone(), item.signature.clone())))?;
        validate_named(&component.name, "signal", component.signals.iter().map(|item| (item.name.clone(), ValueSignature::new(item.fields.iter().map(|(_, ty)| ty.clone()).collect::<Vec<_>>(), ValueType::Null))))?;
        for signal in &component.signals {
            let mut fields = HashSet::new();
            for (field, _) in &signal.fields {
                if !fields.insert(field.to_lowercase()) { return Err(spec_error(RuntimeSpecErrorKind::DuplicateName,
                    format!("{}.signal({}).field({field})", component.name, signal.name), format!("duplicate signal field '{field}'"))); }
            }
        }
    }
    let mut namespaces = HashMap::<String, String>::new();
    for api in apis {
        validate_declaration_name(&api.name, &api_path(api.namespace.as_deref(), &api.name))?;
        if let Some(namespace) = &api.namespace {
            validate_declaration_name(namespace, namespace)?;
            if !namespaces.contains_key(&namespace.to_lowercase()) {
                claim_name(&mut global_names, namespace, namespace)?;
                namespaces.insert(namespace.to_lowercase(), namespace.clone());
            }
        } else { claim_name(&mut global_names, &api.name, &api.name)?; }
    }
    let mut api_names = HashSet::new();
    for api in apis {
        let key = api_key(api.namespace.as_deref(), &api.name);
        if !api_names.insert(key) { return Err(spec_error(RuntimeSpecErrorKind::DuplicateName,
            api_path(api.namespace.as_deref(), &api.name), "duplicate API declaration".into())); }
    }
    Ok(())
}

fn validate_declaration_name(name: &str, path: &str) -> Result<(), RuntimeSpecError> {
    if name.trim().is_empty() || name.contains('.') {
        return Err(spec_error(RuntimeSpecErrorKind::InvalidNesting, path.into(),
            format!("'{name}' is not a valid declaration name")));
    }
    Ok(())
}

fn validate_named(component: &str, kind: &str, items: impl Iterator<Item = (String, ValueSignature)>) -> Result<(), RuntimeSpecError> {
    let mut seen = HashMap::<String, ValueSignature>::new();
    for (name, signature) in items {
        validate_declaration_name(&name, &format!("{component}.{kind}({name})"))?;
        let key = name.to_lowercase();
        if let Some(previous) = seen.get(&key) {
            let error_kind = if previous == &signature { RuntimeSpecErrorKind::DuplicateName } else { RuntimeSpecErrorKind::ConflictingSignature };
            return Err(spec_error(error_kind, format!("{component}.{kind}({name})"), format!("duplicate {kind} '{name}'")));
        }
        seen.insert(key, signature);
    }
    Ok(())
}

fn validate_bindings<I>(components: &[ComponentDeclaration], builtins: &[BuiltinDeclaration], apis: &[ApiDeclaration], bindings: &[(OperationId, I)]) -> Result<(), RuntimeSpecError> {
    let mut declarations = HashMap::<OperationId, String>::new();
    let mut declare = |id: OperationId, path: String| {
        if declarations.insert(id, path.clone()).is_some() {
            Err(spec_error(RuntimeSpecErrorKind::DuplicateOperationId, path, "operation ID is assigned to more than one declaration".into()))
        } else { Ok(()) }
    };
    for builtin in builtins {
        if let Some(id) = builtin.operation_id() { declare(id, builtin.name.clone())?; }
    }
    for component in components {
        if let Some(id) = component.operation_id() { declare(id, component.name.clone())?; }
        for item in &component.constructors {
            if let Some(id) = item.operation_id() { declare(id, format!("{}.constructor({})", component.name, item.name))?; }
        }
        for item in &component.builder_calls {
            if let Some(id) = item.operation_id() { declare(id, format!("{}.builder_call({})", component.name, item.name))?; }
        }
        for item in &component.properties {
            if let Some(id) = item.operation_id() { declare(id, format!("{}.property({})", component.name, item.name))?; }
        }
        for item in &component.methods {
            if let Some(id) = item.operation_id() { declare(id, format!("{}.method({})", component.name, item.name))?; }
        }
        for item in &component.signals { declare(item.operation_id, format!("{}.signal({})", component.name, item.name))?; }
    }
    for api in apis { declare(api.operation_id, api_path(api.namespace.as_deref(), &api.name))?; }

    let mut bound = HashSet::new();
    for (id, _) in bindings {
        if !bound.insert(*id) { return Err(spec_error(RuntimeSpecErrorKind::DuplicateOperationId,
            format!("{id:?}"), "operation ID has more than one implementation".into())); }
        if !declarations.contains_key(id) { return Err(spec_error(RuntimeSpecErrorKind::OrphanImplementation,
            format!("{id:?}"), "implementation is not reachable from a declaration".into())); }
    }
    if let Some((_, path)) = declarations.iter().find(|(id, _)| !bound.contains(id)) {
        return Err(spec_error(RuntimeSpecErrorKind::MissingImplementation, path.clone(),
            "host-effectful declaration has no implementation".into()));
    }
    Ok(())
}

fn claim_name(names: &mut HashMap<String, String>, name: &str, path: &str) -> Result<(), RuntimeSpecError> {
    let key = name.to_lowercase();
    if let Some(previous) = names.get(&key) {
        let kind = if previous == name { RuntimeSpecErrorKind::DuplicateName } else { RuntimeSpecErrorKind::NameConflict };
        return Err(spec_error(kind, path.into(), format!("name '{name}' conflicts with '{previous}'")));
    }
    names.insert(key, name.into()); Ok(())
}

fn spec_error(kind: RuntimeSpecErrorKind, path: String, message: String) -> RuntimeSpecError {
    RuntimeSpecError { kind, message: format!("{path}: {message}"), path }
}

fn api_path(namespace: Option<&str>, name: &str) -> String {
    namespace.map_or_else(|| name.into(), |namespace| format!("{namespace}.api({name})"))
}

impl ValueSignature {
    pub fn new(arguments: impl Into<Vec<ValueType>>, result: ValueType) -> Self {
        let arguments = arguments.into();
        let minimum_arguments = arguments.len();
        Self { arguments, minimum_arguments, result, variadic: false }
    }
    /// Declares a bounded signature whose trailing arguments may be omitted.
    pub fn with_optional(
        arguments: impl Into<Vec<ValueType>>,
        minimum_arguments: usize,
        result: ValueType,
    ) -> Self {
        let arguments = arguments.into();
        assert!(minimum_arguments <= arguments.len());
        Self { arguments, minimum_arguments, result, variadic: false }
    }
    pub fn any() -> Self {
        Self {
            arguments: vec![],
            minimum_arguments: 0,
            result: ValueType::Any,
            variadic: true,
        }
    }
}

pub type ComponentCallback = Arc<dyn Fn(&mut MaterializedCE) -> Result<(), String> + Send + Sync>;

#[derive(Clone)]
pub struct ComponentSpec {
    pub name: String,
    pub aliases: Vec<String>,
    pub constructors: HashMap<String, ValueSignature>,
    pub builder_calls: HashMap<String, ValueSignature>,
    pub properties: HashMap<String, ValueType>,
    pub positional: Vec<ValueType>,
    pub methods: HashMap<String, ValueSignature>,
    pub required_capability: Option<String>,
    pub normalize: Option<ComponentCallback>,
    pub validate: Option<ComponentCallback>,
}

impl fmt::Debug for ComponentSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ComponentSpec").field("name", &self.name).field("aliases", &self.aliases).finish_non_exhaustive()
    }
}

impl ComponentSpec {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), aliases: vec![], constructors: HashMap::new(),
            builder_calls: HashMap::new(), properties: HashMap::new(), positional: vec![],
            methods: HashMap::new(), required_capability: None, normalize: None, validate: None }
    }
    pub fn alias(mut self, alias: impl Into<String>) -> Self { self.aliases.push(alias.into()); self }
    pub fn constructor(mut self, name: impl Into<String>, signature: ValueSignature) -> Self { self.constructors.insert(name.into(), signature); self }
    pub fn builder_call(mut self, name: impl Into<String>, signature: ValueSignature) -> Self { self.builder_calls.insert(name.into(), signature); self }
    pub fn property(mut self, name: impl Into<String>, ty: ValueType) -> Self { self.properties.insert(name.into(), ty); self }
    pub fn positional(mut self, ty: ValueType) -> Self { self.positional.push(ty); self }
    pub fn method(mut self, name: impl Into<String>, signature: ValueSignature) -> Self { self.methods.insert(name.into(), signature); self }
    pub fn requires(mut self, capability: impl Into<String>) -> Self { self.required_capability = Some(capability.into()); self }
    pub fn normalize_with(mut self, callback: impl Fn(&mut MaterializedCE) -> Result<(), String> + Send + Sync + 'static) -> Self { self.normalize = Some(Arc::new(callback)); self }
    pub fn validate_with(mut self, callback: impl Fn(&mut MaterializedCE) -> Result<(), String> + Send + Sync + 'static) -> Self { self.validate = Some(Arc::new(callback)); self }
}

#[derive(Debug, Clone)]
pub struct HostApiSpec {
    pub id: String,
    pub namespace: Option<String>,
    pub name: String,
    pub signature: ValueSignature,
    pub required_capability: String,
}

impl HostApiSpec {
    pub fn function(name: impl Into<String>, signature: ValueSignature) -> Self {
        let name = name.into(); Self { id: name.clone(), namespace: None, name,
            signature, required_capability: String::new() }
    }
    pub fn method(namespace: impl Into<String>, name: impl Into<String>, signature: ValueSignature) -> Self {
        let namespace = namespace.into(); let name = name.into();
        Self { id: format!("{namespace}.{name}"), namespace: Some(namespace), name, signature,
            required_capability: String::new() }
    }
    pub fn id(mut self, id: impl Into<String>) -> Self { self.id = id.into(); self }
    pub fn requires(mut self, capability: impl Into<String>) -> Self { self.required_capability = capability.into(); self }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogErrorKind { DuplicateName, NameConflict, CapabilityMismatch }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogError { pub kind: CatalogErrorKind, pub name: String, pub message: String }
impl fmt::Display for CatalogError { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(&self.message) } }
impl std::error::Error for CatalogError {}

#[derive(Debug, Clone)]
pub(crate) struct ComponentOperationIds {
    pub factory: Option<OperationId>,
    pub constructors: HashMap<String, OperationId>,
    pub builder_calls: HashMap<String, OperationId>,
    pub properties: HashMap<String, OperationId>,
}

#[derive(Debug, Clone)]
pub(crate) struct Catalog {
    pub component_name_policy: ComponentNamePolicy,
    pub components: HashMap<String, Arc<ComponentSpec>>,
    pub canonical_components: Vec<Arc<ComponentSpec>>,
    pub component_operations: HashMap<String, Arc<ComponentOperationIds>>,
    pub apis: HashMap<String, Arc<HostApiSpec>>,
    pub api_operation_ids: HashMap<String, OperationId>,
    pub namespaces: HashSet<String>,
    pub builtins: HashSet<String>,
}

impl Catalog {
    pub(crate) fn has_namespace(&self, name: &str) -> bool {
        self.namespaces.iter().any(|namespace| namespace.eq_ignore_ascii_case(name))
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeBuilder { catalog: Catalog }

impl Default for RuntimeBuilder {
    fn default() -> Self {
        Self { catalog: Catalog { component_name_policy: ComponentNamePolicy::StrictRegistered,
            components: HashMap::new(), canonical_components: vec![], apis: HashMap::new(),
            component_operations: HashMap::new(), api_operation_ids: HashMap::new(),
            namespaces: HashSet::new(), builtins: ["null", "range", "len", "query", "query_all", "Math", "MusicNote"].into_iter().map(str::to_owned).collect() } }
    }
}

impl RuntimeBuilder {
    pub fn new() -> Self { Self::default() }
    pub fn component_name_policy(&mut self, policy: ComponentNamePolicy) -> &mut Self {
        self.catalog.component_name_policy = policy; self
    }
    pub fn register_builtin(&mut self, name: impl Into<String>) -> Result<&mut Self, CatalogError> {
        let name = name.into(); self.ensure_free(&name)?; self.catalog.builtins.insert(name); Ok(self)
    }
    pub fn register_component(&mut self, spec: ComponentSpec) -> Result<&mut Self, CatalogError> {
        let spec = Arc::new(spec);
        for name in std::iter::once(&spec.name).chain(&spec.aliases) { self.ensure_free(name)?; }
        for name in std::iter::once(&spec.name).chain(&spec.aliases) {
            self.catalog.components.insert(name.to_lowercase(), spec.clone());
        }
        self.catalog.canonical_components.push(spec); Ok(self)
    }
    pub fn register_host_api(&mut self, spec: HostApiSpec) -> Result<&mut Self, CatalogError> {
        if let Some(namespace) = &spec.namespace {
            if !self.catalog.namespaces.contains(namespace) { self.ensure_free(namespace)?; }
            self.catalog.namespaces.insert(namespace.clone());
        } else { self.ensure_free(&spec.name)?; }
        let key = api_key(spec.namespace.as_deref(), &spec.name);
        if self.catalog.apis.contains_key(&key) { return Err(duplicate(&key)); }
        self.catalog.apis.insert(key, Arc::new(spec)); Ok(self)
    }
    fn ensure_free(&self, name: &str) -> Result<(), CatalogError> {
        let lower = name.to_lowercase();
        if self.catalog.components.contains_key(&lower) || self.catalog.builtins.iter().any(|v| v.eq_ignore_ascii_case(name))
            || self.catalog.namespaces.iter().any(|v| v.eq_ignore_ascii_case(name))
            || self.catalog.apis.values().any(|api| api.namespace.is_none() && api.name.eq_ignore_ascii_case(name)) {
            return Err(duplicate(name));
        } Ok(())
    }
    pub fn build(self) -> Runtime { Runtime::from_legacy_catalog(self.catalog) }
}

fn duplicate(name: &str) -> CatalogError { CatalogError { kind: CatalogErrorKind::DuplicateName, name: name.into(), message: format!("catalog name '{name}' is already registered") } }
pub(crate) fn api_key(namespace: Option<&str>, name: &str) -> String { namespace.map_or_else(|| name.to_lowercase(), |ns| format!("{}.{}", ns.to_lowercase(), name.to_lowercase())) }

#[derive(Debug, Clone)]
pub struct Runtime {
    spec: Arc<RuntimeSpec>,
    pub(crate) catalog: Arc<Catalog>,
    check_host_capabilities: bool,
}
impl Runtime {
    pub fn builder() -> RuntimeBuilder { RuntimeBuilder::new() }
    pub fn from_spec(spec: RuntimeSpec) -> Self {
        let catalog = compile_catalog(&spec);
        Self { spec: Arc::new(spec), catalog: Arc::new(catalog), check_host_capabilities: false }
    }
    pub fn standard() -> Self {
        let mut builder = RuntimeSpec::builder::<()>();
        builder.component_name_policy(ComponentNamePolicy::OpenUppercase).with_standard_builtins();
        let build = builder.build().expect("the standard runtime specification is valid");
        build.runtime
    }
    pub fn spec(&self) -> &RuntimeSpec { &self.spec }
    pub fn component_names(&self) -> impl Iterator<Item = &str> { self.catalog.components.keys().map(String::as_str) }
    pub fn materialize_component(&self, source: &str) -> Result<MaterializedCE, EvalError> {
        let tokens = MeowMeowTokenizer::new(source)
            .tokenize()
            .map_err(|e| EvalError::Tokenize(format!("{e:?}")))?;
        let parser = match self.catalog.component_name_policy {
            ComponentNamePolicy::OpenUppercase => MeowMeowParser::with_open_component_names(
                tokens,
                self.catalog.components.keys().cloned(),
            ),
            ComponentNamePolicy::StrictRegistered => MeowMeowParser::with_component_names(
                tokens,
                self.catalog.components.keys().cloned(),
            ),
        };
        let statements = parser
            .parse_program()
            .map_err(|e| EvalError::Parse(e.message))?;
        let [Statement::Expression(Expression::Component(component))] = statements.as_slice() else {
            return Err(EvalError::Runtime("expected exactly one component expression".into()));
        };
        let mut host = Hostless;
        let tag = NEXT_SESSION_TAG.fetch_add(1, Ordering::Relaxed);
        let mut context = HostContext::new(tag);
        let mut evaluator = Evaluator::for_session(
            &mut host,
            vec![HashMap::from([("null".into(), Value::Null)])],
            HeapHandle::new(),
            HashMap::new(),
            self.catalog.clone(),
            &mut context,
        );
        evaluator.materialize(component)
    }
    pub fn session<H: Host>(&self, host: H) -> Result<Session<H>, CatalogError> {
        if self.check_host_capabilities {
            check_capabilities(&self.catalog, &host.capabilities())?;
        }
        let tag = NEXT_SESSION_TAG.fetch_add(1, Ordering::Relaxed);
        Ok(Session { runtime: self.clone(), host, scopes: vec![HashMap::from([("null".into(), Value::Null)])],
            heap: HeapHandle::new(), callbacks: HashMap::new(), context: HostContext::new(tag) })
    }

    fn from_legacy_catalog(catalog: Catalog) -> Self {
        let components = catalog.canonical_components.iter().map(|component| ComponentDeclaration {
            name: component.name.clone(), target: ImplementationTarget::Pure,
            aliases: component.aliases.clone(), body_mode: ComponentBodyMode::Standard,
            constructors: component.constructors.iter().map(|(name, signature)| CallableDeclaration {
                name: name.clone(), signature: signature.clone(), target: ImplementationTarget::Pure }).collect(),
            builder_calls: component.builder_calls.iter().map(|(name, signature)| CallableDeclaration {
                name: name.clone(), signature: signature.clone(), target: ImplementationTarget::Pure }).collect(),
            positionals: component.positional.clone(),
            properties: component.properties.iter().map(|(name, value_type)| PropertyDeclaration {
                name: name.clone(), value_type: value_type.clone(), target: ImplementationTarget::Pure }).collect(),
            methods: component.methods.iter().map(|(name, signature)| CallableDeclaration {
                name: name.clone(), signature: signature.clone(), target: ImplementationTarget::Pure }).collect(),
            signals: vec![],
        }).collect();
        let builtins = catalog.builtins.iter().map(|name| BuiltinDeclaration {
            name: name.clone(), signature: ValueSignature::any(), target: ImplementationTarget::Pure }).collect();
        let apis = catalog.apis.values().enumerate().map(|(index, api)| ApiDeclaration {
            namespace: api.namespace.clone(), name: api.name.clone(), signature: api.signature.clone(),
            operation_id: OperationId(u32::try_from(index + 1).expect("legacy API count exceeds operation ID space")),
        }).collect();
        let spec = RuntimeSpec { component_name_policy: catalog.component_name_policy, components, builtins, apis };
        Self { spec: Arc::new(spec), catalog: Arc::new(catalog), check_host_capabilities: true }
    }
}

fn compile_catalog(spec: &RuntimeSpec) -> Catalog {
    let mut catalog = Catalog { component_name_policy: spec.component_name_policy, components: HashMap::new(),
        canonical_components: vec![], apis: HashMap::new(), namespaces: HashSet::new(),
        component_operations: HashMap::new(), api_operation_ids: HashMap::new(),
        builtins: spec.builtins.iter().map(|builtin| builtin.name.clone()).collect() };
    for declaration in &spec.components {
        let operations = Arc::new(ComponentOperationIds {
            factory: declaration.operation_id(),
            constructors: declaration.constructors.iter().filter_map(|item| {
                item.operation_id().map(|id| (item.name.to_lowercase(), id))
            }).collect(),
            builder_calls: declaration.builder_calls.iter().filter_map(|item| {
                item.operation_id().map(|id| (item.name.to_lowercase(), id))
            }).collect(),
            properties: declaration.properties.iter().filter_map(|item| {
                item.operation_id().map(|id| (item.name.to_lowercase(), id))
            }).collect(),
        });
        let component = Arc::new(ComponentSpec {
            name: declaration.name.clone(), aliases: declaration.aliases.clone(),
            constructors: declaration.constructors.iter().map(|item| (item.name.clone(), item.signature.clone())).collect(),
            builder_calls: declaration.builder_calls.iter().map(|item| (item.name.clone(), item.signature.clone())).collect(),
            properties: declaration.properties.iter().map(|item| (item.name.clone(), item.value_type.clone())).collect(),
            positional: declaration.positionals.clone(),
            methods: declaration.methods.iter().map(|item| (item.name.clone(), item.signature.clone())).collect(),
            required_capability: None, normalize: None, validate: None,
        });
        for name in std::iter::once(&component.name).chain(&component.aliases) {
            catalog.components.insert(name.to_lowercase(), component.clone());
            catalog.component_operations.insert(name.to_lowercase(), operations.clone());
        }
        catalog.canonical_components.push(component);
    }
    for declaration in &spec.apis {
        if let Some(namespace) = &declaration.namespace { catalog.namespaces.insert(namespace.clone()); }
        let id = api_key(declaration.namespace.as_deref(), &declaration.name);
        catalog.api_operation_ids.insert(id.clone(), declaration.operation_id);
        catalog.apis.insert(id.clone(), Arc::new(HostApiSpec { id, namespace: declaration.namespace.clone(),
            name: declaration.name.clone(), signature: declaration.signature.clone(), required_capability: String::new() }));
    }
    catalog
}

fn check_capabilities(catalog: &Catalog, host: &HostCapabilities) -> Result<(), CatalogError> {
    for component in &catalog.canonical_components {
        if !host.components.contains(&component.name.to_lowercase()) {
            return Err(CatalogError { kind: CatalogErrorKind::CapabilityMismatch, name: component.name.clone(),
                message: format!("host does not support component '{}'", component.name) });
        }
        if let Some(capability) = &component.required_capability {
            if !host.component_operations.contains(capability) { return Err(CatalogError { kind: CatalogErrorKind::CapabilityMismatch,
                name: capability.clone(), message: format!("host is missing component capability '{capability}'") }); }
        }
    }
    for api in catalog.apis.values() {
        let required = if api.required_capability.is_empty() { &api.id } else { &api.required_capability };
        if !host.api_ids.contains(required) { return Err(CatalogError { kind: CatalogErrorKind::CapabilityMismatch,
            name: required.clone(), message: format!("host is missing API capability '{required}'") }); }
    }
    Ok(())
}

pub struct Session<H: Host> {
    runtime: Runtime,
    host: H,
    pub(crate) scopes: Vec<HashMap<String, Value>>,
    pub(crate) heap: HeapHandle,
    pub(crate) callbacks: HashMap<CallbackHandle, Value>,
    pub(crate) context: HostContext,
}

impl<H: Host> Session<H> {
    pub fn eval(&mut self, source: &str) -> Result<Evaluation, EvalError> {
        let scopes = std::mem::take(&mut self.scopes);
        let callbacks = std::mem::take(&mut self.callbacks);
        let mut evaluator = Evaluator::for_session(&mut self.host, scopes, self.heap.clone(), callbacks,
            self.runtime.catalog.clone(), &mut self.context);
        let result = evaluator.evaluate(source);
        let (scopes, callbacks) = evaluator.into_session_state();
        self.scopes = scopes; self.callbacks = callbacks; result
    }
    pub fn invoke_callback(&mut self, handle: CallbackHandle, args: Vec<Value>) -> Result<Value, EvalError> {
        if !self.context.owns_callback(handle) { return Err(EvalError::Runtime(format!("stale or foreign callback {handle:?}"))); }
        let callback = self.callbacks.get(&handle).cloned().ok_or_else(|| EvalError::Runtime(format!("unknown callback {handle:?}")))?;
        let scopes = std::mem::take(&mut self.scopes); let callbacks = std::mem::take(&mut self.callbacks);
        let mut evaluator = Evaluator::for_session(&mut self.host, scopes, self.heap.clone(), callbacks,
            self.runtime.catalog.clone(), &mut self.context);
        let result = evaluator.invoke_value(callback, args);
        let (scopes, callbacks) = evaluator.into_session_state(); self.scopes = scopes; self.callbacks = callbacks; result
    }
    pub fn host(&self) -> &H { &self.host }
    pub fn host_mut(&mut self) -> &mut H { &mut self.host }
    pub fn context(&self) -> &HostContext { &self.context }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EventStreamHost, HostError, HostRequest, HostResponse, TransportValue};

    struct FixedHandleHost {
        capabilities: HostCapabilities,
        handle: crate::ComponentHandle,
        requests: Vec<HostRequest>,
    }

    impl crate::Host for FixedHandleHost {
        fn capabilities(&self) -> HostCapabilities { self.capabilities.clone() }
        fn dispatch_with_context(&mut self, _context: &mut HostContext, request: HostRequest) -> Result<HostResponse, HostError> {
            self.requests.push(request.clone());
            match request {
                HostRequest::Emit { tree } | HostRequest::RegisterComponent { tree } => {
                    Ok(HostResponse::Component { handle: self.handle, component_type: tree.component_type })
                }
                HostRequest::InvokeComponentMethod { .. } => Ok(HostResponse::Unit),
                other => Err(HostError::unsupported(other.operation_name())),
            }
        }
    }

    fn runtime() -> Runtime {
        let mut builder = Runtime::builder();
        builder.register_component(ComponentSpec::new("Panel").alias("panel")
            .constructor("new", ValueSignature::new(vec![ValueType::Number], ValueType::Component))
            .property("title", ValueType::String)
            .method("show", ValueSignature::new(vec![], ValueType::Null))
            .normalize_with(|tree| { tree.component_type = "Panel".into(); Ok(()) })).unwrap();
        builder.register_host_api(HostApiSpec::method("log", "write",
            ValueSignature::new(vec![ValueType::String], ValueType::Null)).requires("log.write")).unwrap();
        builder.register_host_api(HostApiSpec::method("sink", "write",
            ValueSignature::new(vec![ValueType::Any], ValueType::Null)).requires("sink.write")).unwrap();
        builder.build()
    }

    fn capabilities() -> HostCapabilities {
        HostCapabilities::default().supports_component("Panel").supports_api("log.write").supports_api("sink.write")
    }

    #[test]
    fn catalog_parses_lowercase_aliases_and_issues_handles() {
        let mut session = runtime().session(EventStreamHost::new(capabilities())).unwrap();
        session.eval("panel.new(2) { title = \"hello\" }").unwrap();
        let crate::HostEvent::Emit { handle, tree } = &session.host().events[0] else { panic!() };
        assert!(session.context().owns_component(*handle));
        assert_eq!(tree.component_type, "Panel");
    }

    #[test]
    fn emitted_component_identity_comes_from_host_response() {
        let handle = crate::ComponentHandle::from_raw(0xfeed_face);
        let host = FixedHandleHost { capabilities: capabilities(), handle, requests: vec![] };
        let mut session = runtime().session(host).unwrap();
        let result = session.eval("panel.new(2) { title = \"hello\" }").unwrap();
        assert_eq!(result.value, Some(Value::ComponentObject { id: handle, component_type: "Panel".into() }));
        assert!(session.context().owns_component(handle));
        assert!(matches!(session.host().requests[0], HostRequest::Emit { .. }));
    }

    #[test]
    fn materializes_component_without_host_session() {
        let tree = runtime().materialize_component("panel.new(2) { title = \"hello\" }").unwrap();
        assert_eq!(tree.component_type, "Panel");
        let constructor = tree.constructor.as_ref().unwrap();
        assert_eq!(constructor.name, "new");
        assert_eq!(constructor.operation_id, None);
        assert_eq!(constructor.arguments, vec![Value::Number(2.0)]);
        assert_eq!(tree.properties[0].name, "title");
        assert_eq!(tree.properties[0].operation_id, None);
        assert_eq!(tree.properties[0].value, Value::String("hello".into()));
    }

    #[test]
    fn bindings_and_table_identity_persist_between_evaluations() {
        let mut session = runtime().session(EventStreamHost::new(capabilities())).unwrap();
        session.eval("let table = { value = 1 }; let alias = table").unwrap();
        session.eval("alias[\"value\"] = 9").unwrap();
        let result = session.eval("table[\"value\"]").unwrap();
        assert_eq!(result.value, Some(Value::Number(9.0)));
    }

    #[test]
    fn namespace_api_is_transport_safe() {
        let mut session = runtime().session(EventStreamHost::new(capabilities())).unwrap();
        session.eval("log.write(\"hello\")").unwrap();
        assert!(matches!(&session.host().events[0], crate::HostEvent::Api { id, args }
            if id == "log.write" && args == &vec![TransportValue::String("hello".into())]));
    }

    #[test]
    fn duplicate_and_capability_failures_are_typed() {
        let mut builder = Runtime::builder();
        builder.register_component(ComponentSpec::new("Panel")).unwrap();
        assert_eq!(builder.register_component(ComponentSpec::new("panel")).unwrap_err().kind, CatalogErrorKind::DuplicateName);
        let error = match builder.build().session(EventStreamHost::new(HostCapabilities::default())) {
            Err(error) => error,
            Ok(_) => panic!("expected capability mismatch"),
        };
        assert_eq!(error.kind, CatalogErrorKind::CapabilityMismatch);
    }

    #[test]
    fn suggestions_and_schema_validation_are_reported() {
        let mut session = runtime().session(EventStreamHost::new(capabilities())).unwrap();
        let error = session.eval("panel.neew(2)").unwrap_err().to_string();
        assert!(error.contains("did you mean 'new'"), "{error}");
        let error = session.eval("panel.new(\"bad\")").unwrap_err().to_string();
        assert!(error.contains("wrong type"), "{error}");
    }

    #[test]
    fn host_boundary_rejects_cyclic_tables() {
        let mut session = runtime().session(EventStreamHost::new(capabilities())).unwrap();
        session.eval("let table = { label = \"root\" }; table[\"self\"] = table").unwrap();
        let error = session.eval("sink.write(table)").unwrap_err().to_string();
        assert!(error.contains("cyclic table"), "{error}");
    }

    #[test]
    fn dotted_unknown_component_suggests_registered_names() {
        let mut session = runtime().session(EventStreamHost::new(capabilities())).unwrap();
        let error = session.eval("panal.new(2)").unwrap_err().to_string();
        assert!(error.contains("did you mean 'panel'"), "{error}");
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TestBinding {
        Component,
        Construct,
        Rounded,
        SetTitle,
        Show,
        Click,
        Log,
        Clock,
    }

    #[test]
    fn nested_runtime_spec_preserves_order_and_binds_every_host_declaration() {
        let mut builder = RuntimeSpec::builder::<TestBinding>();
        builder
            .component_name_policy(ComponentNamePolicy::StrictRegistered)
            .with_standard_builtins()
            .host_builtin("clock", ValueSignature::new(vec![], ValueType::Number), TestBinding::Clock)
            .host_component("Panel", TestBinding::Component, |component| {
                component
                    .alias("Pane")
                    .body_mode(ComponentBodyMode::PropsOnly)
                    .host_constructor("new", ValueSignature::new(vec![ValueType::Number], ValueType::Component), TestBinding::Construct)
                    .host_builder_call("rounded", ValueSignature::new(vec![ValueType::Number], ValueType::Component), TestBinding::Rounded)
                    .positional(ValueType::String)
                    .host_property("title", ValueType::String, TestBinding::SetTitle)
                    .property("debug", ValueType::Bool)
                    .method("show", ValueSignature::new(vec![], ValueType::Null), TestBinding::Show)
                    .signal("click", vec![("button".into(), ValueType::Number)], TestBinding::Click);
            })
            .namespace("log", |namespace| {
                namespace.api("write", ValueSignature::new(vec![ValueType::String], ValueType::Null), TestBinding::Log);
            });

        let build = builder.build().unwrap();
        let panel = build.spec().component("pane").unwrap();
        assert_eq!(panel.name(), "Panel");
        assert_eq!(panel.body_mode(), ComponentBodyMode::PropsOnly);
        assert_eq!(panel.properties().map(PropertyDeclaration::name).collect::<Vec<_>>(), vec!["title", "debug"]);
        assert_eq!(panel.signals().next().unwrap().fields().map(|(name, _)| name).collect::<Vec<_>>(), vec!["button"]);

        let ids = [
            build.spec().builtins().find(|builtin| builtin.name() == "clock").unwrap().operation_id().unwrap(),
            panel.operation_id().unwrap(),
            panel.constructors().next().unwrap().operation_id().unwrap(),
            panel.builder_calls().next().unwrap().operation_id().unwrap(),
            panel.properties().next().unwrap().operation_id().unwrap(),
            panel.method("show").unwrap().operation_id().unwrap(),
            panel.signal("click").unwrap().operation_id(),
            build.spec().api(Some("log"), "write").unwrap().operation_id(),
        ];
        assert_eq!(build.bindings().len(), ids.len());
        assert_eq!(ids.map(|id| *build.bindings().get(id).unwrap()), [TestBinding::Clock, TestBinding::Component, TestBinding::Construct, TestBinding::Rounded,
            TestBinding::SetTitle, TestBinding::Show, TestBinding::Click, TestBinding::Log]);

        let tree = build.runtime().materialize_component("Pane.new(2) { title = \"hello\" }").unwrap();
        assert_eq!(tree.component_type, "Panel");
        assert_eq!(tree.factory_operation_id, panel.operation_id());
        assert_eq!(
            tree.constructor.as_ref().and_then(|operation| operation.operation_id),
            panel.constructors().next().unwrap().operation_id()
        );
        assert_eq!(
            tree.properties[0].operation_id,
            panel.properties().next().unwrap().operation_id()
        );
        assert!(build.runtime().materialize_component("Unknown {}").is_err());
        let tree = build.runtime().materialize_component("Pane { rounded(3) }").unwrap();
        assert_eq!(tree.builder_calls[0].name, "rounded");
        assert_eq!(
            tree.builder_calls[0].operation_id,
            panel.builder_calls().next().unwrap().operation_id()
        );
        assert_eq!(tree.builder_calls[0].arguments, vec![Value::Number(3.0)]);
        let error = build.runtime().materialize_component("Pane { missing(3) }").unwrap_err();
        assert!(error.to_string().contains("unknown builder call 'missing'"));
    }

    #[test]
    fn nested_runtime_spec_reports_deterministic_paths_for_conflicts() {
        let mut builder = RuntimeSpec::builder::<()>();
        builder.component("Transform", |component| {
            component
                .method("set_position", ValueSignature::new(vec![ValueType::Number], ValueType::Null), ())
                .method("SET_POSITION", ValueSignature::new(vec![ValueType::String], ValueType::Null), ());
        });
        let error = builder.build().unwrap_err();
        assert_eq!(error.kind, RuntimeSpecErrorKind::ConflictingSignature);
        assert_eq!(error.path, "Transform.method(SET_POSITION)");

        let mut builder = RuntimeSpec::builder::<()>();
        builder.component("Panel", |component| { component.alias("panel"); });
        let error = builder.build().unwrap_err();
        assert_eq!(error.kind, RuntimeSpecErrorKind::NameConflict);
        assert_eq!(error.path, "Panel.alias(panel)");
    }

    #[test]
    fn standard_runtime_is_backed_by_an_open_runtime_spec() {
        let runtime = Runtime::standard();
        assert_eq!(runtime.spec().component_name_policy(), ComponentNamePolicy::OpenUppercase);
        assert!(runtime.spec().builtins().any(|builtin| builtin.name() == "range"));
        assert!(runtime.materialize_component("Unregistered { title = \"ok\" }").is_ok());
    }

    #[test]
    fn fixed_width_numeric_types_validate_the_temporary_number_representation() {
        assert!(ValueType::I64.accepts(&Value::Number(-7.0)));
        assert!(!ValueType::I64.accepts(&Value::Number(1.5)));
        assert!(ValueType::U32.accepts(&Value::Number(u32::MAX as f64)));
        assert!(!ValueType::U32.accepts(&Value::Number(-1.0)));
        assert!(!ValueType::U32.accepts(&Value::Number(u32::MAX as f64 + 1.0)));
        assert!(!ValueType::U32.accepts(&Value::Number(f64::NAN)));
        assert!(ValueType::F32.accepts(&Value::Number(1.25)));
        assert!(!ValueType::F32.accepts(&Value::Number(f64::MAX)));
        assert!(ValueType::F64.accepts(&Value::Number(f64::MAX)));
        assert_eq!(ValueType::U32.name(), "u32");
    }

    #[test]
    fn optional_signature_arguments_are_bounded_and_typed() {
        let mut builder = RuntimeSpec::builder::<()>();
        builder.component("Mesh", |component| {
            component.constructor(
                "star",
                ValueSignature::with_optional(
                    vec![ValueType::U32, ValueType::F32],
                    0,
                    ValueType::Component,
                ),
            );
        });
        let runtime = builder.build().unwrap();

        assert!(runtime.runtime().materialize_component("Mesh.star() {}").is_ok());
        assert!(runtime.runtime().materialize_component("Mesh.star(5) {}").is_ok());
        assert!(runtime.runtime().materialize_component("Mesh.star(5, 0.4) {}").is_ok());
        assert!(runtime.runtime().materialize_component("Mesh.star(-1) {}").is_err());
        assert!(runtime.runtime().materialize_component("Mesh.star(5, \"wide\") {}").is_err());
        assert!(runtime.runtime().materialize_component("Mesh.star(5, 0.4, 2) {}").is_err());
    }
}
