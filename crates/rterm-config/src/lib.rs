mod diagnostic;
mod diff;
mod lifecycle;
mod model;
mod patch;
pub mod schemes;
mod source;
mod validate;

pub use diagnostic::ConfigDiagnostic;
pub use diff::{
    ConfigDiff, DomainConfigDiff, FontConfigDiff, InputConfigDiff, LifecycleConfigDiff,
    RenderConfigDiff, TerminalConfigDiff, WindowConfigDiff,
};
pub use lifecycle::{
    ConfigLifecycle, ConfigLifecycleEvent, ConfigSnapshot, FixedWindowDebouncer, SourceChange,
};
pub use model::{
    DomainConfig, EffectiveConfig, FontConfig, InputConfig, LifecycleConfig, RenderConfig,
    TerminalConfig, WindowConfig,
};
pub use patch::{
    ConfigPatch, DomainConfigPatch, FontConfigPatch, InputConfigPatch, LifecycleConfigPatch, Patch,
    RenderConfigPatch, TerminalConfigPatch, WindowConfigPatch, resolve_layers,
};
pub use source::{
    ConfigDiscoveryInputs, ConfigEnvironmentSnapshot, ConfigLoadAttempt, ConfigSource,
    ConfigSourceError, ConfigSourceErrorKind, DerivedConfigEnvironment, ResolvedConfigSource,
};
pub use validate::{ConfigUpdate, ValidatedConfigStore, validate};
