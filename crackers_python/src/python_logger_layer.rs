use pyo3::prelude::*;
// PythonLoggerLayer: tracing subscriber layer for forwarding Rust logs to Python logging
use pyo3::Python;
use pyo3::types::PyModule;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::{Context, Layer};

pub struct PythonLoggerLayer;

impl<S> Layer<S> for PythonLoggerLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        struct MessageVisitor {
            message: Option<String>,
        }
        impl tracing_subscriber::field::Visit for MessageVisitor {
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                if field.name() == "message" {
                    self.message = Some(format!("{:?}", value));
                }
            }
        }
        Python::attach(|py| {
            if let Ok(logging) = PyModule::import(py, "logging") {
                let meta = event.metadata();
                let module_path = meta.module_path().unwrap_or("");
                let file = meta.file().unwrap_or("");
                let line = meta.line().map(|l| l.to_string()).unwrap_or_default();
                let level = meta.level();
                let mut visitor = MessageVisitor { message: None };
                event.record(&mut visitor);
                let message = visitor.message.unwrap_or_default();
                let msg = format!("[{}:{}:{}] {}", module_path, file, line, message);
                let py_level = match *level {
                    Level::ERROR => "error",
                    Level::WARN => "warning",
                    Level::INFO => "info",
                    Level::DEBUG => "debug",
                    Level::TRACE => "debug",
                };
                let _ = logging.call_method1(py_level, (msg,));
            }
        });
    }
}
